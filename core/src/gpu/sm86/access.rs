use super::*;

impl<'a> Decoder<'a> {
    // Silicon faults on an address past shared memory, which here wraps instead of indexing out of bounds.
    pub(super) fn shared_word_at(&mut self, address: u32) -> u32 {
        let [two, last] = [2, SHARED_WORDS - 1].map(|c| self.cached_const_u32(c));
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], address, two);
        let index = self.ir.emit_bitwise_and(self.type_u32[1], index, last);
        self.workgroup_word(ResourceKind::Shared, SHARED_WORDS, index)
    }

    fn atomic_update(&mut self, ptr: u32, scope: u32, sem: u32, update: impl Fn(&mut Self, u32) -> u32) -> u32 {
        let relaxed = self.cached_const_u32(spirv::memory_semantics::RELAXED);
        let [header, body, merge] = [(); 3].map(|_| self.ir.alloc_id());
        self.ir.emit_branch(header);
        self.ir.emit_label_id(header);
        self.ir.emit_loop_merge(merge, body, 0);
        self.ir.emit_branch(body);
        self.ir.emit_label_id(body);
        let old = self.ir.emit_atomic_load(self.type_u32[1], ptr, scope, relaxed);
        let new = update(self, old);
        let seen = self.ir.emit_atomic_compare_exchange(self.type_u32[1], ptr, scope, sem, relaxed, new, old);
        let done = self.ir.emit_iequal(self.type_bool[1], seen, old);
        self.ir.emit_branch_conditional(done, merge, header);
        self.ir.emit_label_id(merge);
        old
    }

    // Guest cbuf slot N is a std140 UBO at set 0, binding 1 + N, holding uvec4 data[4096]; the host support UBO is binding 0.
    pub(super) fn load_cbuf(&mut self, kind: ResourceKind, offset: u32) -> u32 {
        let var = match self.resource(kind) {
            Some(var) => var,
            None => self.declare_cbuf(kind),
        };
        let [two, three, four] = [2, 3, 4].map(|i| self.cached_const_u32(i));
        let vec = self.ir.emit_shift_right_logical(self.type_u32[1], offset, four);
        let word = self.ir.emit_shift_right_logical(self.type_u32[1], offset, two);
        let elem = self.ir.emit_bitwise_and(self.type_u32[1], word, three);
        let ptr = self.ir.emit_access_chain(self.type_ptr_cbuf_u32, var, &[self.const_u32_0, vec, elem]);
        self.ir.emit_load(self.type_u32[1], ptr)
    }

    fn declare_cbuf(&mut self, kind: ResourceKind) -> u32 {
        use spirv::{decoration as deco, storage_class::UNIFORM};
        let binding = match kind {
            ResourceKind::Cbuf(slot) if slot < 8 => CBUF_BINDING + slot,
            ResourceKind::Support => SUPPORT_BINDING,
            _ => panic!("{kind:?} has no compute uniform binding"),
        };
        if self.type_ptr_cbuf == 0 {
            let len = self.cached_const_u32(4096);
            let array = self.ir.emit_type_array(self.type_u32[4], len);
            let block = self.ir.emit_type_struct(&[array]);
            // std140 pads every array element to 16 bytes, so uvec4 is the only dense u32 array a UBO can hold.
            self.ir.emit_decorate(array, deco::ARRAY_STRIDE, &[16]);
            self.ir.emit_decorate(block, deco::BLOCK, &[]);
            self.ir.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
            self.type_ptr_cbuf = self.ir.emit_type_pointer(UNIFORM, block);
            self.type_ptr_cbuf_u32 = self.ir.emit_type_pointer(UNIFORM, self.type_u32[1]);
        }
        let var = self.ir.emit_variable(self.type_ptr_cbuf, UNIFORM);
        self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
        self.ir.emit_decorate(var, deco::BINDING, &[binding]);
        self.resources.push(Resource { kind, var });
        var
    }

    pub(super) fn global_word_at(&mut self, address: u32) -> u32 {
        let var = match self.resource(ResourceKind::Global) {
            Some(var) => var,
            None => self.declare_ssbo(ResourceKind::Global, GLOBAL_BINDING),
        };
        let base = self.load_cbuf(ResourceKind::Support, self.const_u32_0);
        let rel = self.ir.emit_isub(self.type_u32[1], address, base);
        let two = self.cached_const_u32(2);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], rel, two);
        self.ir.emit_access_chain(self.type_ptr_ssbo_u32, var, &[self.const_u32_0, index])
    }

    pub(super) fn global_address(&mut self, inst: u128, uniform: u32, offset_shift: u32) -> u32 {
        let mut address = self.load_register((inst >> 24) as u32 & 0xff);
        if inst >> 91 & 1 != 0 && inst >> 76 & 1 == 0 {
            let ur = (inst >> uniform & 0x3f) as u32;
            let value = self.load_ureg(ur);
            address = self.ir.emit_iadd(self.type_u32[1], address, value);
        }
        let offset = if inst >> 91 & 1 != 0 || offset_shift == 40 {
            (((inst >> 40) as i32) << 8 >> 8) as u32
        } else {
            (inst >> 32) as u32
        };
        let offset = self.cached_const_u32(offset);
        self.ir.emit_iadd(self.type_u32[1], address, offset)
    }

    pub(super) fn atomic_op(&mut self, wide: bool, ptr: u32, scope: u32, sem: u32, op: u32, signed: bool, b: u32) -> u32 {
        let ty = if wide { self.type_u64() } else { self.type_u32[1] };
        match op {
            0 => self.ir.emit_atomic_iadd(ty, ptr, scope, sem, b),
            1 if signed => self.ir.emit_atomic_smin(ty, ptr, scope, sem, b),
            1 => self.ir.emit_atomic_umin(ty, ptr, scope, sem, b),
            2 if signed => self.ir.emit_atomic_smax(ty, ptr, scope, sem, b),
            2 => self.ir.emit_atomic_umax(ty, ptr, scope, sem, b),
            3 if !wide => self.atomic_update(ptr, scope, sem, |d, old| {
                let wraps = d.ir.emit_ugreater_than_equal(d.type_bool[1], old, b);
                let next = d.ir.emit_iadd(ty, old, d.const_u32_1);
                d.ir.emit_select(ty, wraps, d.const_u32_0, next)
            }),
            4 if !wide => self.atomic_update(ptr, scope, sem, |d, old| {
                let zero = d.ir.emit_iequal(d.type_bool[1], old, d.const_u32_0);
                let above = d.ir.emit_ugreater_than(d.type_bool[1], old, b);
                let wraps = d.ir.emit_logical_or(d.type_bool[1], zero, above);
                let next = d.ir.emit_isub(ty, old, d.const_u32_1);
                d.ir.emit_select(ty, wraps, b, next)
            }),
            5 => self.ir.emit_atomic_and(ty, ptr, scope, sem, b),
            6 => self.ir.emit_atomic_or(ty, ptr, scope, sem, b),
            7 => self.ir.emit_atomic_xor(ty, ptr, scope, sem, b),
            8 => self.ir.emit_atomic_exchange(ty, ptr, scope, sem, b),
            9 => todo!("ATOMG.SAFEADD has no public semantics; Eden also leaves it TODO"),
            _ => todo!("atomic op {op}"),
        }
    }

    // Session 6: ATOM.ADD.F32 flushes both operands and the sum, and it and ATOM.F16x2 store canonical NaNs.
    pub(super) fn atomic_fadd(&mut self, ptr: u32, scope: u32, sem: u32, value: u32) -> u32 {
        self.atomic_update(ptr, scope, sem, |d, old| {
            let [old, value] = [old, value].map(|word| d.fp_operand(word, false, false, true));
            let sum = d.ir.emit_fadd(d.type_f32[1], old, value);
            let sum = d.canonical_nan_bits(sum);
            d.flush_subnormal(sum)
        })
    }

    pub(super) fn atomic_f16x2(&mut self, ptr: u32, scope: u32, sem: u32, op: u32, value: u32) -> u32 {
        self.atomic_update(ptr, scope, sem, |d, old| {
            let ty = d.type_f32[1];
            let [old, value] = [old, value].map(|word| d.half2_lanes(word, 0, false, false, false));
            let new = [0, 1].map(|i| match op {
                0 => d.ir.emit_fadd(ty, old[i], value[i]),
                1 => d.glsl(spirv::glsl::N_MIN, ty, &[old[i], value[i]]),
                2 => d.glsl(spirv::glsl::N_MAX, ty, &[old[i], value[i]]),
                _ => panic!("ATOM.F16x2 operation {op}"),
            });
            d.pack_half2(new)
        })
    }

    // Local memory is a lazy 4 KiB Private array per invocation.
    fn local_word_at(&mut self, address: u32) -> u32 {
        let var = match self.resource(ResourceKind::Local) {
            Some(var) => var,
            None => {
                let len = self.cached_const_u32(1024);
                let array = self.ir.emit_type_array(self.type_u32[1], len);
                let ptr = self.ir.emit_type_pointer(storage_class::PRIVATE, array);
                let var = self.ir.emit_variable(ptr, storage_class::PRIVATE);
                self.resources.push(Resource { kind: ResourceKind::Local, var });
                var
            }
        };
        let two = self.cached_const_u32(2);
        let index = self.ir.emit_shift_right_logical(self.type_u32[1], address, two);
        self.ir.emit_access_chain(self.type_ptr_u32[1], var, &[index])
    }

    pub(super) fn load_narrow(&mut self, word: u32, address: u32, size: u32) -> u32 {
        if size == 4 {
            return word;
        }
        let three = self.cached_const_u32(3);
        let byte = self.ir.emit_bitwise_and(self.type_u32[1], address, three);
        let eight = self.cached_const_u32(8);
        let offset = self.ir.emit_imul(self.type_u32[1], byte, eight);
        let width = self.cached_const_u32(if size < 2 { 8 } else { 16 });
        if size & 1 != 0 {
            self.ir.emit_bit_field_s_extract(self.type_u32[1], word, offset, width)
        } else {
            self.ir.emit_bit_field_u_extract(self.type_u32[1], word, offset, width)
        }
    }

    fn store_narrow(&mut self, kind: ResourceKind, ptr: u32, address: u32, size: u32, value: u32) {
        if size >= 4 {
            return self.ir.emit_store(ptr, value);
        }
        let ty = self.type_u32[1];
        let three = self.cached_const_u32(3);
        let byte = self.ir.emit_bitwise_and(ty, address, three);
        let offset = self.ir.emit_shift_left_logical(ty, byte, three);
        let low = self.cached_const_u32(if size < 2 { 0xff } else { 0xffff });
        let mask = self.ir.emit_shift_left_logical(ty, low, offset);
        let keep = self.ir.emit_not(ty, mask);
        let value = self.ir.emit_shift_left_logical(ty, value, offset);
        let value = self.ir.emit_bitwise_and(ty, value, mask);
        let scope = match kind {
            ResourceKind::Local => {
                let word = self.ir.emit_load(ty, ptr);
                let word = self.ir.emit_bitwise_and(ty, word, keep);
                let word = self.ir.emit_bitwise_or(ty, word, value);
                return self.ir.emit_store(ptr, word);
            }
            ResourceKind::Shared => spirv::scope::WORKGROUP,
            _ => spirv::scope::DEVICE,
        };
        // Lanes storing into one word race, so the bytes merge in a compare-exchange loop where the last writer of a byte wins.
        let [scope, relaxed] = [scope, spirv::memory_semantics::RELAXED].map(|c| self.cached_const_u32(c));
        self.atomic_update(ptr, scope, relaxed, |s, old| {
            let kept = s.ir.emit_bitwise_and(ty, old, keep);
            s.ir.emit_bitwise_or(ty, kept, value)
        });
    }

    fn memory_word_at(&mut self, kind: ResourceKind, address: u32) -> u32 {
        match kind {
            ResourceKind::Global => self.global_word_at(address),
            ResourceKind::Local => self.local_word_at(address),
            ResourceKind::Shared => self.shared_word_at(address),
            _ => unreachable!(),
        }
    }

    pub(super) fn load_memory(&mut self, kind: ResourceKind, address: u32, size: u32, dst: u32) {
        for i in 0..access_words(size) {
            let offset = self.cached_const_u32(4 * i);
            let word_address = self.ir.emit_iadd(self.type_u32[1], address, offset);
            let ptr = self.memory_word_at(kind, word_address);
            let word = self.ir.emit_load(self.type_u32[1], ptr);
            let value = if size <= 4 { self.load_narrow(word, address, size) } else { word };
            self.store_register(nth_reg(dst, RZ, i), value);
        }
    }

    pub(super) fn store_memory(&mut self, kind: ResourceKind, address: u32, size: u32, src: u32) {
        for i in 0..access_words(size) {
            let offset = self.cached_const_u32(4 * i);
            let word_address = self.ir.emit_iadd(self.type_u32[1], address, offset);
            let ptr = self.memory_word_at(kind, word_address);
            let value = self.load_register(nth_reg(src, RZ, i));
            self.store_narrow(kind, ptr, address, size, value);
        }
    }

    pub(super) fn load_dynamic_cbuf(&mut self, bank: u32, offset: u32) -> u32 {
        let mut value = self.load_cbuf(ResourceKind::Cbuf(0), offset);
        for slot in 1..8 {
            let candidate = self.load_cbuf(ResourceKind::Cbuf(slot), offset);
            let slot = self.cached_const_u32(slot);
            let selected = self.ir.emit_iequal(self.type_bool[1], bank, slot);
            value = self.ir.emit_select(self.type_u32[1], selected, candidate, value);
        }
        value
    }
}
