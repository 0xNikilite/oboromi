use super::*;

const BMOV_R_B: u32 = 0x355;
const BMOV_B_R: u32 = 0x356;
const BMOV_B_B_CLEAR: u32 = 0xf55;
const BMOV_B_B: u32 = 0xf56;
const BMOV_ATEXIT_R: u32 = 0x357;
const BMOV_ATEXIT_I: u32 = 0x957;

impl<'a> Decoder<'a> {
    fn pack_lanes(&mut self, bits: Vec<u32>) -> u32 {
        let mut mask = self.const_u32_0;
        for (lane, bit) in (0..).zip(bits) {
            let offset = self.cached_const_u32(lane);
            mask = self.ir.emit_bit_field_insert(self.type_u32[1], mask, bit, offset, self.const_u32_1);
        }
        mask
    }

    pub fn bmov(&mut self, inst: u128) {
        let state = (inst >> 24 & 0x3f) as u32;
        let clear = inst >> 84 & 1 != 0;
        let op = opcode(inst);
        if matches!(op, BMOV_ATEXIT_R | BMOV_ATEXIT_I) {
            let pc = if op == BMOV_ATEXIT_R { self.load_register((inst >> 32 & 0xff) as u32) } else { self.cached_const_u32(((inst >> 34) as u32) << 2) };
            let ptr = self.barrier_token(ATEXIT_PC);
            return self.ir.emit_store(ptr, pc);
        }
        assert!(state < 16 && (op == BMOV_R_B || clear == (op == BMOV_B_B_CLEAR)), "unsupported BMOV state {state}");
        let state_ptr = self.barrier_token(state);
        match op {
            BMOV_R_B => {
                let token = self.ir.emit_load(self.type_u32[1], state_ptr);
                self.store_register((inst >> 16 & 0xff) as u32, token);
            }
            BMOV_B_R => {
                let token = self.load_register((inst >> 32 & 0xff) as u32);
                self.ir.emit_store(state_ptr, token);
            }
            BMOV_B_B_CLEAR | BMOV_B_B => {
                let other = self.barrier_token((inst >> 16 & 0xf) as u32);
                let (source, destination) = if clear { (state_ptr, other) } else { (other, state_ptr) };
                let token = self.ir.emit_load(self.type_u32[1], source);
                self.ir.emit_store(destination, token);
            }
            op => panic!("unsupported BMOV opcode {op:#x}"),
        }
        if clear {
            self.ir.emit_store(state_ptr, self.const_u32_0);
        }
    }
    pub fn match_(&mut self, inst: u128) {
        let ra = (inst >> 24 & 0xff) as u32;
        let words = if inst >> 73 & 1 != 0 { vec![ra, nth_reg(ra, RZ, 1)] } else { vec![ra] };
        let mut same = self.cached_const_u32(u32::MAX);
        let mut all = same;
        for reg in words {
            let own = self.load_register(reg);
            let (bits, active) = self.warp_values(own).into_iter().map(|[v, active]| {
                let eq = self.ir.emit_iequal(self.type_bool[1], v, own);
                let bit = self.bool_bit(eq);
                (self.ir.emit_bitwise_and(self.type_u32[1], bit, active), active)
            }).unzip();
            let mask = self.pack_lanes(bits);
            all = self.pack_lanes(active);
            same = self.ir.emit_bitwise_and(self.type_u32[1], same, mask);
        }
        let rd = (inst >> 16 & 0xff) as u32;
        if inst >> 79 & 1 != 0 {
            return self.store_register(rd, same);
        }
        let uniform = self.ir.emit_iequal(self.type_bool[1], same, all);
        let mask = self.ir.emit_select(self.type_u32[1], uniform, all, self.const_u32_0);
        self.store_register(rd, mask);
        let bit = self.bool_bit(uniform);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, bit);
    }
    pub fn r2ur(&mut self, inst: u128) {
        assert!(inst >> 81 & 7 == 7, "R2UR with a predicate destination");
        let mut value = self.load_register((inst >> 24 & 0xff) as u32);
        if inst >> 84 & 1 != 0 {
            value = self.warp_values(value).into_iter().fold(self.const_u32_0, |acc, [v, _]| self.ir.emit_bitwise_or(self.type_u32[1], acc, v));
        }
        self.store_ureg((inst >> 16 & 0x3f) as u32, value);
    }
    pub fn redux(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let signed = inst >> 73 & 1 != 0;
        let identity = match (inst >> 78 & 7, signed) {
            (0, _) | (4, false) => u32::MAX,
            (4, true) => i32::MAX as u32,
            (5, true) => i32::MIN as u32,
            _ => 0,
        };
        let identity = self.cached_const_u32(identity);
        let lanes: Vec<u32> = self.warp_values(a).into_iter().map(|[v, active]| {
            let active = self.ir.emit_inot_equal(self.type_bool[1], active, self.const_u32_0);
            self.ir.emit_select(ty, active, v, identity)
        }).collect();
        let mut acc = lanes[0];
        for &v in &lanes[1..] {
            acc = match inst >> 78 & 7 {
                op @ 0..=2 => self.bool_op(op, acc, v),
                3 => self.ir.emit_iadd(ty, acc, v),
                4 => self.glsl(if signed { spirv::glsl::S_MIN } else { spirv::glsl::U_MIN }, ty, &[acc, v]),
                5 => self.glsl(if signed { spirv::glsl::S_MAX } else { spirv::glsl::U_MAX }, ty, &[acc, v]),
                op => panic!("REDUX op {op} is invalid"),
            };
        }
        self.store_ureg((inst >> 16 & 0x3f) as u32, acc);
    }
    pub fn shfl(&mut self, inst: u128) {
        let (ty, bool_ty) = (self.type_u32[1], self.type_bool[1]);
        let b = if matches!(inst >> 9 & 7, 4 | 7) { self.cached_const_u32((inst >> 53 & 0x1f) as u32) } else { self.load_register((inst >> 32 & 0xff) as u32) };
        let c = if matches!(inst >> 9 & 7, 2 | 7) { self.cached_const_u32((inst >> 40 & 0x1fff) as u32) } else { self.load_register((inst >> 64 & 0xff) as u32) };
        let a = self.load_register((inst >> 24 & 0xff) as u32);
        let [five, eight, lane_bits] = [5, 8, 31].map(|i| self.cached_const_u32(i));
        let lane = self.special_register(0);
        let b = self.ir.emit_bitwise_and(ty, b, lane_bits);
        let clamp = self.ir.emit_bitwise_and(ty, c, lane_bits);
        let segment = self.ir.emit_bit_field_u_extract(ty, c, eight, five);
        let outside = self.ir.emit_not(ty, segment);
        let min_lane = self.ir.emit_bitwise_and(ty, lane, segment);
        let clamp = self.ir.emit_bitwise_and(ty, clamp, outside);
        let max_lane = self.ir.emit_bitwise_or(ty, min_lane, clamp);
        let (src, in_range) = match inst >> 58 & 3 {
            0 => {
                let offset = self.ir.emit_bitwise_and(ty, b, outside);
                let src = self.ir.emit_bitwise_or(ty, min_lane, offset);
                (src, self.ir.emit_uless_than_equal(bool_ty, src, max_lane))
            }
            1 => {
                let src = self.ir.emit_isub(ty, lane, b);
                (src, self.ir.emit_sgreater_than_equal(bool_ty, src, max_lane))
            }
            mode => {
                let src = if mode == 2 { self.ir.emit_iadd(ty, lane, b) } else { self.ir.emit_bitwise_xor(ty, lane, b) };
                (src, self.ir.emit_uless_than_equal(bool_ty, src, max_lane))
            }
        };
        let src = self.ir.emit_select(ty, in_range, src, lane);
        let [val, active] = self.warp_exchange(a, &[src])[0];
        self.store_register((inst >> 16 & 0xff) as u32, val);
        let active = self.ir.emit_inot_equal(bool_ty, active, self.const_u32_0);
        let in_range = self.ir.emit_logical_and(bool_ty, in_range, active);
        let bit = self.bool_bit(in_range);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, bit);
    }
    pub fn vote(&mut self, inst: u128) {
        let (ballot, result) = self.vote_ballot(inst);
        self.store_register((inst >> 16 & 0xff) as u32, ballot);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, result);
    }
    pub fn voteu(&mut self, inst: u128) {
        let (ballot, result) = self.vote_ballot(inst);
        self.store_ureg((inst >> 16 & 0x3f) as u32, ballot);
        self.store_pred_bit(true, (inst >> 81 & 7) as u32, result);
    }
    fn vote_ballot(&mut self, inst: u128) -> (u32, u32) {
        let p = self.pp_bit(inst);
        let (bits, active) = self.warp_values(p).into_iter().map(|[bit, active]| (bit, active)).unzip();
        let ballot = self.pack_lanes(bits);
        let active = self.pack_lanes(active);
        let bool_ty = self.type_bool[1];
        let every = self.ir.emit_iequal(bool_ty, ballot, active);
        let none = self.ir.emit_iequal(bool_ty, ballot, self.const_u32_0);
        let result = match inst >> 72 & 3 {
            0 => every,
            1 => self.ir.emit_logical_not(bool_ty, none),
            2 => self.ir.emit_logical_or(bool_ty, every, none),
            _ => panic!("VOTE mode 3 is invalid"),
        };
        (ballot, self.bool_bit(result))
    }
}
