use super::*;

impl<'a> Decoder<'a> {
    fn global_f32_at(&mut self, address: u32) -> u32 {
        use spirv::{decoration as deco, storage_class::STORAGE_BUFFER};
        let kind = ResourceKind::Ssbo(2);
        let var = match self.resource(kind) {
            Some(var) => var,
            None => {
                let array = self.ir.emit_type_runtime_array(self.type_f32[1]);
                let block = self.ir.emit_type_struct(&[array]);
                let ptr = self.ir.emit_type_pointer(STORAGE_BUFFER, block);
                self.ir.emit_decorate(array, deco::ARRAY_STRIDE, &[4]);
                self.ir.emit_decorate(block, deco::BLOCK, &[]);
                self.ir.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
                let var = self.ir.emit_variable(ptr, STORAGE_BUFFER);
                self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
                self.ir.emit_decorate(var, deco::BINDING, &[GLOBAL_BINDING]);
                self.resources.push(Resource { kind, var });
                var
            }
        };
        let base = self.load_cbuf(ResourceKind::Support, self.const_u32_0);
        let rel = self.ir.emit_isub(self.type_u32[1], address, base);
        let two = self.cached_const_u32(2);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], rel, two);
        let ptr = self.ir.emit_type_pointer(STORAGE_BUFFER, self.type_f32[1]);
        self.ir.emit_access_chain(ptr, var, &[self.const_u32_0, index])
    }

    fn global_u64_at(&mut self, address: u32) -> u32 {
        use spirv::{decoration as deco, storage_class::STORAGE_BUFFER};
        let ty = self.type_u64();
        self.ir.emit_capability(INT64_ATOMICS);
        let var = match self.resource(ResourceKind::Global64) {
            Some(var) => var,
            None => {
                let array = self.ir.emit_type_runtime_array(ty);
                let block = self.ir.emit_type_struct(&[array]);
                self.type_ptr_ssbo64 = self.ir.emit_type_pointer(STORAGE_BUFFER, block);
                self.type_ptr_ssbo_u64 = self.ir.emit_type_pointer(STORAGE_BUFFER, ty);
                self.ir.emit_decorate(array, deco::ARRAY_STRIDE, &[8]);
                self.ir.emit_decorate(block, deco::BLOCK, &[]);
                self.ir.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
                let var = self.ir.emit_variable(self.type_ptr_ssbo64, STORAGE_BUFFER);
                self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
                self.ir.emit_decorate(var, deco::BINDING, &[GLOBAL_BINDING]);
                self.resources.push(Resource { kind: ResourceKind::Global64, var });
                var
            }
        };
        let base = self.load_cbuf(ResourceKind::Support, self.const_u32_0);
        let rel = self.ir.emit_isub(self.type_u32[1], address, base);
        let three = self.cached_const_u32(3);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], rel, three);
        self.ir.emit_access_chain(self.type_ptr_ssbo_u64, var, &[self.const_u32_0, index])
    }

    fn global_atomic(&mut self, inst: u128, op: u32, sz: u32, rb: u32) -> u32 {
        use spirv::memory_semantics::{ACQUIRE_RELEASE, UNIFORM_MEMORY};
        let wide = matches!(sz, 2 | 5);
        let address = self.global_address(inst, 64, 40);
        let scope = self.cached_const_u32(spirv::scope::DEVICE);
        let sem = self.cached_const_u32(ACQUIRE_RELEASE | UNIFORM_MEMORY);
        let ptr = if wide {
            assert!(crate::gpu::device().is_ok_and(|d| d.shader_buffer_int64_atomics), "64-bit global atomics require shaderBufferInt64Atomics");
            self.global_u64_at(address)
        } else {
            self.global_word_at(address)
        };
        let load = |this: &mut Self, reg| if wide { this.load_pair(reg) } else { this.load_register(reg) };
        let value = load(self, rb);
        if inst & 1 != 0 {
            let swap = load(self, (inst >> 64 & 0xff) as u32);
            let ty = if wide { self.type_u64() } else { self.type_u32[1] };
            return self.ir.emit_atomic_compare_exchange(ty, ptr, scope, sem, self.const_u32_0, swap, value);
        }
        match sz {
            3 => {
                assert!(op == 0, "ATOM.F32 only defines ADD");
                assert!(crate::gpu::device().is_ok_and(|d| d.shader_buffer_float32_atomic_add), "ATOM.ADD.F32 requires shaderBufferFloat32AtomicAdd");
                let ptr = self.global_f32_at(address);
                self.atomic_fadd(ptr, scope, sem, value)
            }
            4 => {
                assert!(crate::gpu::device().is_ok_and(|d| d.shader_float16), "ATOM.F16x2 requires shaderFloat16");
                self.atomic_f16x2(ptr, scope, sem, op, value)
            }
            6.. => panic!("ATOM size {sz}"),
            _ => self.atomic_op(wide, ptr, scope, sem, op, matches!(sz, 1 | 5), value),
        }
    }

    pub fn atom(&mut self, inst: u128) {
        match opcode(inst) {
            0x1f8a => {
                use spirv::memory_semantics::{ACQUIRE_RELEASE, UNIFORM_MEMORY};
                let address = self.global_address(inst, 64, 40);
                let [scope, sem] = [spirv::scope::DEVICE, ACQUIRE_RELEASE | UNIFORM_MEMORY].map(|c| self.cached_const_u32(c));
                self.mbarrier(inst, address, Self::global_word_at, scope, sem);
            }
            _ => self.atomg(inst),
        }
    }
    // ptxas lowers mbarrier.init(n) to a store of 0x40000000 - n and 0x80000000 - n, mbarrier.test_wait.parity to bit 31
    // of the high word, and arrive_drop to POPC.INC on the low word before an ARRIVE. So an arrive adds one to the
    // high word, and the one that clears its bits 0..30 completes the phase and reloads them with the low word + 0x40000000.
    pub(super) fn mbarrier(&mut self, inst: u128, address: u32, word_at: fn(&mut Self, u32) -> u32, scope: u32, sem: u32) {
        let (ty, rd) = (self.type_u32[1], (inst >> 16 & 0xff) as u32);
        let four = self.cached_const_u32(4);
        let hi_address = self.ir.emit_iadd(ty, address, four);
        let [lo, hi] = [address, hi_address].map(|address| word_at(self, address));
        match inst >> 87 & 0xf {
            9 => {
                let old_lo = self.ir.emit_atomic_load(ty, lo, scope, self.const_u32_0);
                let [pending, phase_offset] = [0x7fff_ffff, 0x4000_0000].map(|c| self.cached_const_u32(c));
                let reload = self.ir.emit_iadd(ty, old_lo, phase_offset);
                let old_hi = self.atomic_update(hi, scope, sem, |s, old| {
                    let new = s.ir.emit_iadd(ty, old, s.const_u32_1);
                    let left = s.ir.emit_bitwise_and(ty, new, pending);
                    let completes = s.ir.emit_iequal(s.type_bool[1], left, s.const_u32_0);
                    let reloaded = s.ir.emit_bitwise_or(ty, new, reload);
                    s.ir.emit_select(ty, completes, reloaded, new)
                });
                self.store_register(rd, old_lo);
                self.store_register(nth_reg(rd, RZ, 1), old_hi);
            }
            10 => {
                let old = self.ir.emit_atomic_iadd(ty, lo, scope, sem, self.const_u32_1);
                self.store_register(rd, old);
            }
            op => panic!("ATOM arrive-class operation {op}"),
        }
    }
    pub fn atomg(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0xf) << 0) as u32;
        let op = (((inst >> 87) & 0xf) << 0) as u32;
        let old = self.global_atomic(inst, op, sz, rb);
        if matches!(sz, 2 | 5) {
            self.store_pair(rd, old);
        } else {
            self.store_register(rd, old);
        }
    }
    pub fn red(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0xf) << 0) as u32;
        let pp = (((inst >> 87) & 0x7) << 0) as u32;
        self.global_atomic(inst, pp, sz, rb);
    }
}
