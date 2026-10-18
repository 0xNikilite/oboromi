//! TTU traversal on a Vulkan ray query. The driver's own code around the TTU does the ray query's bookkeeping in
//! registers: it stages the ray with TTUST, starts the unit with TTUGO, reads the outcome back with TTULD and runs
//! TTUGO again while the continuation it read is not zero, committing candidates itself and shortening tmax to the
//! committed t (query-candidates.comp.bin, NVIDIA 580 on an RTX 3090). A lane's TTU addresses are private words here,
//! and TTUGO is one OpRayQueryProceedKHR on the lane's ray query, written back where the driver's code reads it.
use super::*;

const TTUGO: u32 = 0x3d3;
const TTU_WORDS: u32 = 0x100;
const CONFIG: u32 = 0x00;
const ORIGIN: u32 = 0x40;
const DIRECTION: u32 = 0x50;
const ROOT: u32 = 0x200;
const HIT: u32 = 0x300;
const INSTANCE: u32 = 0x320;
const CONTINUATION: u32 = 0x380;
// Bit 31 of the hit's t hands the driver a candidate to commit or skip; 0xe0000000 in place of v ends the traversal.
const CANDIDATE: u32 = 1 << 31;
const DONE: u32 = 0xe000_0000;
const TRIANGLE: u32 = 1;
const GET_T: u32 = 6018;
const GET_INSTANCE_ID: u32 = 6020;
const GET_PRIMITIVE_INDEX: u32 = 6023;
const GET_BARYCENTRICS: u32 = 6024;
const GET_FRONT_FACE: u32 = 6025;
const GET_TYPE: u32 = 4479;

/// The cbuf word pair holding the TLAS address: rayQueryInitialize null-tests it with `ISETP.NE.U32.AND.EX P, PT, RZ,
/// c[bank][offset + 4]` ahead of the first TTUGO.
pub(super) fn acceleration_structure_address(words: &[u128]) -> Option<(u32, u32)> {
    let go = words.iter().position(|&inst| opcode(inst) == TTUGO)?;
    let null_test = words[..go].iter().rev().find(|&&inst| opcode(inst) == 0xa0c && inst >> 24 & 0xff == u128::from(RZ) && inst >> 72 & 1 != 0)?;
    Some(((null_test >> 54 & 0x1f) as u32, (null_test >> 40 & 0x3fff) as u32 * 4 - 4))
}

impl Decoder<'_> {
    pub fn ttuopen(&mut self, _inst: u128) {}
    pub fn ttumacrofuse(&mut self, _inst: u128) {}
    pub fn ttucctl(&mut self, _inst: u128) {}

    pub fn ttust(&mut self, inst: u128) {
        let [rb, rc] = [32, 64].map(|lo| (inst >> lo & 0xff) as u32);
        for (i, reg) in (0..).zip([nth_reg(rc, RZ, 0), nth_reg(rc, RZ, 1), nth_reg(rb, RZ, 0), nth_reg(rb, RZ, 1)]) {
            let value = self.load_register(reg);
            self.store_ttu(ttu_address(inst) + 4 * i, value);
        }
    }

    pub fn ttuld(&mut self, inst: u128) {
        assert!(inst >> 81 & 7 == u128::from(PT), "TTULD predicate output");
        let [rd, rd2] = [16, 64].map(|lo| (inst >> lo & 0xff) as u32);
        for (i, reg) in (0..).zip([nth_reg(rd2, RZ, 0), nth_reg(rd2, RZ, 1), nth_reg(rd, RZ, 0), nth_reg(rd, RZ, 1)]) {
            let value = self.load_ttu(ttu_address(inst) + 4 * i);
            self.store_register(reg, value);
        }
    }

    pub fn ttugo(&mut self, _inst: u128) {
        self.ray_queries += 1;
        if self.ray_queries > self.ray_budget {
            self.store_ttu(ROOT + 4, self.const_u32_0);
            self.store_ttu(CONTINUATION + 8, self.const_u32_0);
            self.store_ttu(CONTINUATION + 12, self.const_u32_0);
            let done = self.cached_const_u32(DONE);
            self.store_ttu(HIT + 4, done);
            return;
        }
        let (u32_ty, bool_ty) = (self.type_u32[1], self.type_bool[1]);
        let query = self.ray_query();
        // The driver stages the TLAS root only to start a traversal.
        let root = self.load_ttu(ROOT + 4);
        let fresh = self.ir.emit_inot_equal(bool_ty, root, self.const_u32_0);
        let [initialize, resume, confirm, resumed, started] = [(); 5].map(|_| self.ir.alloc_id());
        self.ir.emit_selection_merge(started, 0);
        self.ir.emit_branch_conditional(fresh, initialize, resume);
        self.ir.emit_label_id(initialize);
        self.store_ttu(ROOT + 4, self.const_u32_0);
        // The first config word holds the facing culls at bits 4 and 5, TerminateOnFirstHit at 7 and the cull mask in
        // the top byte, the second Opaque at bit 5.
        let [config, opacity] = [CONFIG, CONFIG + 4].map(|addr| self.load_ttu(addr));
        let [culling, terminate, opaque, cull_mask] = [(config, 0x30, 0), (config, 4, 5), (opacity, 1, 5), (config, 0xff, 24)].map(|(word, mask, shift)| {
            let shift = self.cached_const_u32(shift);
            let mask = self.cached_const_u32(mask);
            let word = self.ir.emit_shift_right_logical(u32_ty, word, shift);
            self.ir.emit_bitwise_and(u32_ty, word, mask)
        });
        let flags = self.ir.emit_bitwise_or(u32_ty, culling, terminate);
        let flags = self.ir.emit_bitwise_or(u32_ty, flags, opaque);
        let [(origin, tmin), (direction, tmax)] = [ORIGIN, DIRECTION].map(|addr| {
            let [x, y, z, w] = [0, 4, 8, 12].map(|i| {
                let word = self.load_ttu(addr + i);
                self.bitcast_f32(word)
            });
            (self.ir.emit_composite_construct(self.type_f32[3], &[x, y, z]), w)
        });
        let accel = self.acceleration_structure();
        self.ir.emit_ray_query_initialize(query, accel, flags, cull_mask, origin, tmin, direction, tmax);
        self.ir.emit_branch(started);
        // The driver confirms a candidate by staging its t as the new tmax.
        self.ir.emit_label_id(resume);
        let tmax = self.load_ttu(DIRECTION + 12);
        let t = self.load_ttu(HIT + 12);
        let magnitude = self.cached_const_u32(!CANDIDATE);
        let t = self.ir.emit_bitwise_and(u32_ty, t, magnitude);
        let confirmed = self.ir.emit_iequal(bool_ty, tmax, t);
        self.ir.emit_selection_merge(resumed, 0);
        self.ir.emit_branch_conditional(confirmed, confirm, resumed);
        self.ir.emit_label_id(confirm);
        self.ir.emit_ray_query_confirm_intersection(query);
        self.ir.emit_branch(resumed);
        self.ir.emit_label_id(resumed);
        self.ir.emit_branch(started);
        self.ir.emit_label_id(started);
        let proceeding = self.ir.emit_ray_query_proceed(bool_ty, query);
        let more = self.bool_bit(proceeding);
        self.store_ttu(CONTINUATION + 8, more);
        self.store_ttu(CONTINUATION + 12, more);
        let [candidate, finished, hit, committed, done] = [(); 5].map(|_| self.ir.alloc_id());
        self.ir.emit_selection_merge(done, 0);
        self.ir.emit_branch_conditional(proceeding, candidate, finished);
        self.ir.emit_label_id(candidate);
        self.store_hit(query, self.const_u32_0, CANDIDATE);
        self.ir.emit_branch(done);
        self.ir.emit_label_id(finished);
        let done_word = self.cached_const_u32(DONE);
        self.store_ttu(HIT + 4, done_word);
        let kind = self.ir.emit_ray_query_get_intersection(GET_TYPE, u32_ty, query, self.const_u32_1);
        let triangle = self.cached_const_u32(TRIANGLE);
        let triangle = self.ir.emit_iequal(bool_ty, kind, triangle);
        self.ir.emit_selection_merge(committed, 0);
        self.ir.emit_branch_conditional(triangle, hit, committed);
        self.ir.emit_label_id(hit);
        self.store_hit(query, self.const_u32_1, 0);
        self.ir.emit_branch(committed);
        self.ir.emit_label_id(committed);
        self.ir.emit_branch(done);
        self.ir.emit_label_id(done);
    }

    // The driver reads the hit as u with the back face in its sign, v, the primitive and t, and the instance plus one.
    fn store_hit(&mut self, query: u32, which: u32, t_flag: u32) {
        let (u32_ty, f32_ty) = (self.type_u32[1], self.type_f32[1]);
        let barycentrics = self.ir.emit_ray_query_get_intersection(GET_BARYCENTRICS, self.type_f32[2], query, which);
        let [u, v] = [0, 1].map(|i| {
            let value = self.ir.emit_composite_extract(f32_ty, barycentrics, &[i]);
            self.bitcast_u32(value)
        });
        let front = self.ir.emit_ray_query_get_intersection(GET_FRONT_FACE, self.type_bool[1], query, which);
        let sign = self.cached_const_u32(1 << 31);
        let back = self.ir.emit_select(u32_ty, front, self.const_u32_0, sign);
        let u = self.ir.emit_bitwise_or(u32_ty, u, back);
        let primitive = self.ir.emit_ray_query_get_intersection(GET_PRIMITIVE_INDEX, u32_ty, query, which);
        let t = self.ir.emit_ray_query_get_intersection(GET_T, f32_ty, query, which);
        let t = self.bitcast_u32(t);
        let t_flag = self.cached_const_u32(t_flag);
        let t = self.ir.emit_bitwise_or(u32_ty, t, t_flag);
        for (i, value) in (0..).zip([u, v, primitive, t]) {
            self.store_ttu(HIT + 4 * i, value);
        }
        let instance = self.ir.emit_ray_query_get_intersection(GET_INSTANCE_ID, u32_ty, query, which);
        let instance = self.ir.emit_iadd(u32_ty, instance, self.const_u32_1);
        self.store_ttu(INSTANCE + 12, instance);
    }

    fn ttu_word(&mut self, addr: u32) -> u32 {
        let var = match self.resource(ResourceKind::Ttu) {
            Some(var) => var,
            None => {
                let len = self.cached_const_u32(TTU_WORDS);
                let array = self.ir.emit_type_array(self.type_u32[1], len);
                let zeros = self.ir.emit_constant_composite(array, &[self.const_u32_0; TTU_WORDS as usize]);
                let ptr = self.ir.emit_type_pointer(storage_class::PRIVATE, array);
                let var = self.ir.emit_variable_init(ptr, storage_class::PRIVATE, zeros);
                self.resources.push(Resource { kind: ResourceKind::Ttu, var });
                var
            }
        };
        assert!(addr < 4 * TTU_WORDS, "TTU address {addr:#x}");
        let index = self.cached_const_u32(addr / 4);
        self.ir.emit_access_chain(self.type_ptr_u32[1], var, &[index])
    }

    fn load_ttu(&mut self, addr: u32) -> u32 {
        let word = self.ttu_word(addr);
        self.ir.emit_load(self.type_u32[1], word)
    }

    fn store_ttu(&mut self, addr: u32, value: u32) {
        let word = self.ttu_word(addr);
        self.ir.emit_store(word, value);
    }

    fn ray_query(&mut self) -> u32 {
        if let Some(var) = self.resource(ResourceKind::RayQuery) {
            return var;
        }
        self.ir.emit_capability(spirv::capability::RAY_QUERY);
        self.ir.emit_extension("SPV_KHR_ray_query");
        let ty = self.ir.emit_type_ray_query();
        let ptr = self.ir.emit_type_pointer(storage_class::PRIVATE, ty);
        let var = self.ir.emit_variable(ptr, storage_class::PRIVATE);
        self.resources.push(Resource { kind: ResourceKind::RayQuery, var });
        var
    }

    fn acceleration_structure(&mut self) -> u32 {
        let (cbuf_bank, byte_offset) = self.acceleration_structure_address.expect("TTUGO without a null-tested TLAS address ahead of it");
        let kind = ResourceKind::AccelerationStructure { cbuf_bank, byte_offset };
        if self.type_acceleration_structure == 0 {
            self.type_acceleration_structure = self.ir.emit_type_acceleration_structure();
        }
        let ty = self.type_acceleration_structure;
        let var = match self.resource(kind) {
            Some(var) => var,
            None => {
                let ptr = self.ir.emit_type_pointer(storage_class::UNIFORM_CONSTANT, ty);
                let var = self.ir.emit_variable(ptr, storage_class::UNIFORM_CONSTANT);
                self.ir.emit_decorate(var, spirv::decoration::DESCRIPTOR_SET, &[self.descriptor_set()]);
                self.resources.push(Resource { kind, var });
                var
            }
        };
        self.ir.emit_load(ty, var)
    }

    pub(super) fn bind_acceleration_structure(&mut self) {
        let kinds: Vec<_> = self.resources.iter().map(|r| r.kind).collect();
        if let Some(r) = self.resources.iter().find(|r| matches!(r.kind, ResourceKind::AccelerationStructure { .. })) {
            self.ir.emit_decorate(r.var, spirv::decoration::BINDING, &[acceleration_structure_binding(&kinds)]);
        }
    }
}

fn ttu_address(inst: u128) -> u32 {
    (inst >> 40 & 0xffff) as u32
}
