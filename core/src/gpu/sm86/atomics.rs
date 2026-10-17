use super::*;

impl<'a> Decoder<'a> {
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
            0x1f8a => todo!("ATOM.ARRIVE and ATOM.POPC.INC have no published memory-word state transition in the SM86 table, Eden, Ryujinx, or NAK"),
            _ => self.atomg(inst),
        }
    }
    pub fn atomg(&mut self, inst: u128) {
        // Session 6: ATOM.CAST on global memory leaves both the memory word and Rd untouched on silicon.
        if opcode(inst) == 0x38b && inst >> 87 & 1 != 0 {
            return;
        }
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
