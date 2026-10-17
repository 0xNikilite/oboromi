use super::*;

impl<'a> Decoder<'a> {
    fn shared_address(&mut self, inst: u128, uniform: u32) -> u32 {
        assert!(inst >> 78 & 3 == 0, "shared address stride {}", inst >> 78 & 3);
        self.global_address(inst, uniform, 40)
    }

    fn shared_u64_at(&mut self, address: u32) -> u32 {
        use spirv::storage_class::WORKGROUP;
        let ty = self.type_u64();
        self.ir.emit_capability(INT64_ATOMICS);
        let var = match self.resource(ResourceKind::Shared64) {
            Some(var) => var,
            None => {
                let len = self.cached_const_u32(SHARED_WORDS / 2);
                let array = self.ir.emit_type_array(ty, len);
                let ptr = self.ir.emit_type_pointer(WORKGROUP, array);
                self.type_ptr_shared_u64 = self.ir.emit_type_pointer(WORKGROUP, ty);
                let var = self.ir.emit_variable(ptr, WORKGROUP);
                self.resources.push(Resource { kind: ResourceKind::Shared64, var });
                var
            }
        };
        let [three, last] = [3, SHARED_WORDS / 2 - 1].map(|c| self.cached_const_u32(c));
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], address, three);
        let index = self.ir.emit_bitwise_and(self.type_u32[1], index, last);
        self.ir.emit_access_chain(self.type_ptr_shared_u64, var, &[index])
    }

    pub fn atoms(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x3) << 0) as u32;
        let op = (((inst >> 87) & 0xf) << 0) as u32;
        let cas = opcode(inst) == 0x38d;
        match opcode(inst) {
            0x1f8c => todo!("ATOMS.ARRIVE and ATOMS.POPC.INC expose encodings but not state-transition semantics"),
            0x38d if op & 1 == 1 => todo!("ATOMS.CAST exposes operands but not cast semantics"),
            _ => {}
        }
        let address = self.shared_address(inst, 64);
        if matches!(sz, 2 | 5) {
            assert!(crate::gpu::device().is_ok_and(|d| d.shader_shared_int64_atomics), "64-bit shared atomics require shaderSharedInt64Atomics");
            let ptr = self.shared_u64_at(address);
            let lo_ptr = self.shared_word_at(address);
            let four = self.cached_const_u32(4);
            let hi_address = self.ir.emit_iadd(self.type_u32[1], address, four);
            let hi_ptr = self.shared_word_at(hi_address);
            let words = [lo_ptr, hi_ptr].map(|p| self.ir.emit_load(self.type_u32[1], p));
            let words = self.ir.emit_composite_construct(self.type_u32[2], &words);
            let ty = self.type_u64();
            let initial = self.ir.emit_bitcast(ty, words);
            self.ir.emit_store(ptr, initial);
            let scope = self.cached_const_u32(spirv::scope::WORKGROUP);
            let sem = self.cached_const_u32(spirv::memory_semantics::ACQUIRE_RELEASE | spirv::memory_semantics::WORKGROUP_MEMORY);
            self.ir.emit_control_barrier(scope, scope, sem);
            let b = self.load_pair(rb);
            let old = if cas {
                let c = self.load_pair((inst >> 64) as u32 & 0xff);
                let relaxed = self.cached_const_u32(spirv::memory_semantics::RELAXED);
                self.ir.emit_atomic_compare_exchange(ty, ptr, scope, sem, relaxed, c, b)
            } else {
                self.atomic_op(true, ptr, scope, sem, op, sz == 5, b)
            };
            self.ir.emit_control_barrier(scope, scope, sem);
            let final_value = self.ir.emit_load(ty, ptr);
            let final_words = self.ir.emit_bitcast(self.type_u32[2], final_value);
            for (i, dst) in [lo_ptr, hi_ptr].into_iter().enumerate() {
                let word = self.ir.emit_composite_extract(self.type_u32[1], final_words, &[i as u32]);
                self.ir.emit_store(dst, word);
            }
            self.ir.emit_control_barrier(scope, scope, sem);
            self.store_pair(rd, old);
            return;
        }
        if sz >= 2 {
            todo!("floating-point shared atomics require VK_EXT_shader_atomic_float");
        }
        let ptr = self.shared_word_at(address);
        let b = self.load_register(rb);
        let (ty, signed) = (self.type_u32[1], sz == 1);
        let scope = self.cached_const_u32(spirv::scope::WORKGROUP);
        let relaxed = self.cached_const_u32(spirv::memory_semantics::RELAXED);
        let old = if cas {
            let c = self.load_register((inst >> 64) as u32 & 0xff);
            self.ir.emit_atomic_compare_exchange(ty, ptr, scope, relaxed, relaxed, c, b)
        } else {
            self.atomic_op(false, ptr, scope, relaxed, op, signed, b)
        };
        self.store_register(rd, old);
    }
    pub fn ldgsts(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 32) as i32) << 20 >> 20) as u32;
        let rb_offset = (((inst >> 44) as i32) << 12 >> 12) as u32;
        let ra_urc = (((inst >> 64) & 0x3f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let [ra_offset, rb_offset] = [ra_offset, rb_offset].map(|v| self.cached_const_u32(v));
        let mut global = self.load_register(ra);
        let mut shared = self.load_register(rd);
        const LDGSTS_UR_GLOBAL: u32 = 0x1fae;
        let ur = self.load_ureg(ra_urc);
        if opcode(inst) == LDGSTS_UR_GLOBAL {
            global = self.ir.emit_iadd(self.type_u32[1], global, ur);
        } else {
            shared = self.ir.emit_iadd(self.type_u32[1], shared, ur);
        }
        global = self.ir.emit_iadd(self.type_u32[1], global, ra_offset);
        shared = self.ir.emit_iadd(self.type_u32[1], shared, rb_offset);
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(4 * i);
            let global = self.ir.emit_iadd(self.type_u32[1], global, offset);
            let shared = self.ir.emit_iadd(self.type_u32[1], shared, offset);
            let src = self.global_word_at(global);
            let value = self.ir.emit_load(self.type_u32[1], src);
            let dst = self.shared_word_at(shared);
            self.ir.emit_store(dst, value);
        }
    }
    pub fn lds(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.shared_address(inst, 32);
        self.load_memory(ResourceKind::Shared, address, sz, rd);
    }
    pub fn sts(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.shared_address(inst, 64);
        self.store_memory(ResourceKind::Shared, address, sz, rb);
    }
}
