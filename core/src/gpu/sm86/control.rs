use super::*;

impl<'a> Decoder<'a> {
    // No-ops: ARRIVES(unmodeled), R2B(unmodeled).
    // The workgroup agrees on the lowest pc a lane can run through three rotating words, so one barrier per step keeps a
    // fast invocation's next vote out of the word a slow one is still reading. A lane waiting on BSYNC Bx cannot run while
    // another lane of its warp holds the same Bx token and is not at a BSYNC Bx itself. A lane past WARPSYNC waits until
    // every running lane of its mask waits on the same mask, and one past BAR.SYNC or BAR.RED until the step whose tally
    // meets the barrier's thread count, or finds no running lane left to arrive when it has none. That release lands
    // after the vote, so it keeps the loop going for one more step even when no lane could run in this one.
    pub(super) fn schedule(&mut self, words: &[u128], pc: u32) -> [u32; 2] {
        let (ty, bool_ty) = (self.type_u32[1], self.type_bool[1]);
        let end = self.cached_const_u32(words.len() as u32);
        let running = self.ir.emit_uless_than(bool_ty, pc, end);
        let mut runnable = running;
        let mut barriers: Vec<u32> = words.iter().filter(|&&inst| opcode(inst) == BSYNC).map(|&inst| (inst >> 16 & 0xf) as u32).collect();
        barriers.sort_unstable();
        barriers.dedup();
        self.active = self.ir.emit_constant_true(bool_ty);
        for barrier in barriers {
            let mut arrived = self.ir.emit_constant_false(bool_ty);
            for (at, &inst) in (0..).zip(words) {
                if opcode(inst) == BSYNC && (inst >> 16 & 0xf) as u32 == barrier {
                    let at = self.cached_const_u32(at);
                    let mut here = self.ir.emit_iequal(bool_ty, pc, at);
                    if let Some(guard) = self.guard(inst) {
                        here = self.ir.emit_logical_and(bool_ty, here, guard);
                    }
                    arrived = self.ir.emit_logical_or(bool_ty, arrived, here);
                }
            }
            let ptr = self.barrier_token(barrier);
            let token = self.ir.emit_load(ty, ptr);
            let away = self.ir.emit_logical_not(bool_ty, arrived);
            let owing = self.ir.emit_logical_and(bool_ty, running, away);
            let owed = self.ir.emit_select(ty, owing, token, self.const_u32_0);
            let mut outstanding = self.ir.emit_constant_false(bool_ty);
            for [other, _] in self.warp_values(owed) {
                let same = self.ir.emit_iequal(bool_ty, other, token);
                outstanding = self.ir.emit_logical_or(bool_ty, outstanding, same);
            }
            let held = self.ir.emit_inot_equal(bool_ty, token, self.const_u32_0);
            let blocked = self.ir.emit_logical_and(bool_ty, held, outstanding);
            let blocked = self.ir.emit_logical_and(bool_ty, arrived, blocked);
            let free = self.ir.emit_logical_not(bool_ty, blocked);
            runnable = self.ir.emit_logical_and(bool_ty, runnable, free);
        }
        let none = self.cached_const_u32(u32::MAX);
        if words.iter().any(|&inst| partial_warpsync(inst)) {
            let ptr = self.barrier_token(WARPSYNC_MASK);
            let mask = self.ir.emit_load(ty, ptr);
            let offer = self.ir.emit_select(ty, running, mask, none);
            let mut waits = self.ir.emit_constant_false(bool_ty);
            for (lane, [other, _]) in (0..).zip(self.warp_values(offer)) {
                let lane = self.cached_const_u32(lane);
                let member = self.ir.emit_bit_field_u_extract(ty, mask, lane, self.const_u32_1);
                let member = self.ir.emit_inot_equal(bool_ty, member, self.const_u32_0);
                let [same, gone] = [mask, none].map(|v| self.ir.emit_iequal(bool_ty, other, v));
                let ready = self.ir.emit_logical_or(bool_ty, same, gone);
                let late = self.ir.emit_logical_not(bool_ty, ready);
                let late = self.ir.emit_logical_and(bool_ty, member, late);
                waits = self.ir.emit_logical_or(bool_ty, waits, late);
            }
            let kept = self.ir.emit_select(ty, waits, mask, self.const_u32_0);
            self.ir.emit_store(ptr, kept);
            let free = self.ir.emit_logical_not(bool_ty, waits);
            runnable = self.ir.emit_logical_and(bool_ty, runnable, free);
        }
        let bars = guest_barriers(words);
        let mut states = Vec::new();
        for &bar in &bars {
            let ptr = self.barrier_token(BAR_ARRIVALS + bar);
            let state = self.ir.emit_load(ty, ptr);
            let [arrived, parked, pred, op, count] = [(0, 1), (1, 1), (2, 1), (3, 2), (5, 12)].map(|(offset, bits)| {
                let [offset, bits] = [offset, bits].map(|c| self.cached_const_u32(c));
                self.ir.emit_bit_field_u_extract(ty, state, offset, bits)
            });
            let parked = self.ir.emit_iequal(bool_ty, parked, self.const_u32_0);
            runnable = self.ir.emit_logical_and(bool_ty, runnable, parked);
            states.push((ptr, state, arrived, pred, op, count));
        }
        let candidate = self.ir.emit_select(ty, runnable, pc, none);
        let step = self.ir.emit_load(ty, self.var_step);
        let [three, stride] = [3, SCHEDULE_STRIDE].map(|c| self.cached_const_u32(c));
        let [now, next] = [self.const_u32_0, self.const_u32_1].map(|offset| {
            let step = self.ir.emit_iadd(ty, step, offset);
            let slot = self.ir.emit_umod(ty, step, three);
            self.ir.emit_imul(ty, slot, stride)
        });
        let [scope, relaxed] = [spirv::scope::WORKGROUP, spirv::memory_semantics::RELAXED].map(|c| self.cached_const_u32(c));
        let vote = self.schedule_word(now, 0);
        self.ir.emit_atomic_umin(ty, vote, scope, relaxed, candidate);
        self.clear_schedule_slot(next, &bars);
        let [sixteen, twelve] = [16, 12].map(|c| self.cached_const_u32(c));
        let mut tallies = Vec::new();
        for (&bar, &(_, _, arrived, pred, _, count)) in bars.iter().zip(&states) {
            let [tally, terms] = [1, 2].map(|word| self.schedule_word(now, 2 * bar + word));
            let pred = self.ir.emit_shift_left_logical(ty, pred, sixteen);
            let arrival = self.ir.emit_bitwise_or(ty, arrived, pred);
            self.ir.emit_atomic_iadd(ty, tally, scope, relaxed, arrival);
            let away = self.ir.emit_iequal(bool_ty, arrived, self.const_u32_0);
            let owing = self.ir.emit_logical_and(bool_ty, running, away);
            let owing = self.bool_bit(owing);
            let owing = self.ir.emit_shift_left_logical(ty, owing, twelve);
            let term = self.ir.emit_bitwise_or(ty, count, owing);
            self.ir.emit_atomic_or(ty, terms, scope, relaxed, term);
            tallies.push([tally, terms]);
        }
        self.workgroup_barrier();
        let scheduled = self.ir.emit_load(ty, vote);
        self.here = Some(self.ir.emit_iequal(bool_ty, candidate, scheduled));
        let mut released = self.ir.emit_constant_false(bool_ty);
        let result_ptr = self.barrier_token(BAR_RESULT);
        let mut result = self.ir.emit_load(ty, result_ptr);
        for ((ptr, state, arrived, _, op, _), [tally, terms]) in states.into_iter().zip(tallies) {
            let [tally, terms] = [tally, terms].map(|word| self.ir.emit_load(ty, word));
            let [arrivals, preds, count, owing] = [(tally, 0, 16), (tally, 16, 16), (terms, 0, 12), (terms, 12, 1)].map(|(word, offset, bits)| {
                let [offset, bits] = [offset, bits].map(|c| self.cached_const_u32(c));
                self.ir.emit_bit_field_u_extract(ty, word, offset, bits)
            });
            let counted = self.ir.emit_inot_equal(bool_ty, count, self.const_u32_0);
            let enough = self.ir.emit_ugreater_than_equal(bool_ty, arrivals, count);
            let all = self.ir.emit_iequal(bool_ty, owing, self.const_u32_0);
            let met = self.ir.emit_select(bool_ty, counted, enough, all);
            let some = self.ir.emit_inot_equal(bool_ty, arrivals, self.const_u32_0);
            let release = self.ir.emit_logical_and(bool_ty, some, met);
            released = self.ir.emit_logical_or(bool_ty, released, release);
            let mine = self.ir.emit_inot_equal(bool_ty, arrived, self.const_u32_0);
            let mine = self.ir.emit_logical_and(bool_ty, mine, release);
            let kept = self.ir.emit_select(ty, mine, self.const_u32_0, state);
            self.ir.emit_store(ptr, kept);
            let every = self.ir.emit_iequal(bool_ty, preds, arrivals);
            let any = self.ir.emit_inot_equal(bool_ty, preds, self.const_u32_0);
            let [every, any] = [every, any].map(|c| self.bool_bit(c));
            let [popc, and] = [0, 1].map(|c| {
                let c = self.cached_const_u32(c);
                self.ir.emit_iequal(bool_ty, op, c)
            });
            let reduced = self.ir.emit_select(ty, and, every, any);
            let reduced = self.ir.emit_select(ty, popc, preds, reduced);
            result = self.ir.emit_select(ty, mine, reduced, result);
        }
        self.ir.emit_store(result_ptr, result);
        let progress = self.ir.emit_select(ty, released, self.const_u32_0, scheduled);
        [scheduled, progress]
    }

    fn schedule_word(&mut self, slot: u32, word: u32) -> u32 {
        let word = self.cached_const_u32(word);
        let index = self.ir.emit_iadd(self.type_u32[1], slot, word);
        self.workgroup_word(ResourceKind::Schedule, 3 * SCHEDULE_STRIDE, index)
    }

    pub(super) fn clear_schedule_slot(&mut self, slot: u32, bars: &[u32]) {
        let [scope, relaxed, none] = [spirv::scope::WORKGROUP, spirv::memory_semantics::RELAXED, u32::MAX].map(|c| self.cached_const_u32(c));
        let vote = self.schedule_word(slot, 0);
        self.ir.emit_atomic_store(vote, scope, relaxed, none);
        for &bar in bars {
            for word in [1, 2] {
                let word = self.schedule_word(slot, 2 * bar + word);
                self.ir.emit_atomic_store(word, scope, relaxed, self.const_u32_0);
            }
        }
    }

    // A lane holds the step of its BSSY Bx as the Bx token until it passes BSYNC Bx or leaves through BREAK Bx.
    fn store_barrier_token(&mut self, inst: u128, token: u32) {
        assert_eq!(inst >> 87 & 0xf, PT as u128, "{:#x} with a condition predicate", opcode(inst));
        let ptr = self.barrier_token((inst >> 16 & 0xf) as u32);
        self.ir.emit_store(ptr, token);
    }

    // Ra or URa holds a byte address and the pc counts 16-byte instructions.
    fn jump_to_register(&mut self, inst: u128, relative: bool) {
        let ra = if inst >> 91 & 1 != 0 { self.load_ureg((inst >> 24 & 0x3f) as u32) } else { self.load_register((inst >> 24 & 0xff) as u32) };
        let four = self.cached_const_u32(4);
        let ra = self.ir.emit_shift_right_logical(self.type_u32[1], ra, four);
        let offset = branch_offset(inst);
        let base = self.cached_const_u32((offset + if relative { self.pc as i64 + 1 } else { 0 }) as u32);
        let target = self.ir.emit_iadd(self.type_u32[1], base, ra);
        self.ir.emit_store(self.var_pc, target);
    }

    pub fn arrives(&mut self, _inst: u128) {}
    pub fn b2r(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        if inst >> 78 & 3 != 1 {
            return self.store_register(rd, self.const_u32_0);
        }
        let ptr = self.barrier_token(BAR_RESULT);
        let result = self.ir.emit_load(self.type_u32[1], ptr);
        self.store_register(rd, result);
        let nonzero = self.ir.emit_inot_equal(self.type_bool[1], result, self.const_u32_0);
        let bit = self.bool_bit(nonzero);
        self.store_pred_bit(false, (inst >> 81 & 7) as u32, bit);
    }
    // A lane records its arrival and the schedule releases it: bit 0 arrived, 1 parked until release, 2 the BAR.RED
    // predicate, 3..4 the BAR.RED operation, 5.. the thread count, 0 for every running lane of the CTA.
    pub fn bar(&mut self, inst: u128) {
        let ty = self.type_u32[1];
        let (op, parks) = match inst >> 77 & 7 {
            0 | 4 => (0, true),
            1 => (0, false),
            2 => (inst >> 74 & 3, true),
            3 => todo!("BAR.SCAN has no published scan result or per-lane ordering in the SM86 table, Eden, Ryujinx, or NAK"),
            mode => panic!("BAR mode {mode} is invalid"),
        };
        assert!(op != 3, "BAR.RED operation 3 is invalid");
        let [sb, sc] = [inst >> 54 & 0xf, inst >> 42 & 0xfff].map(|v| self.cached_const_u32(v as u32));
        let rb = (inst >> 32 & 0xff) as u32;
        let (bar, count) = match opcode(inst) {
            0xb1d => (sb, sc),
            0x91d => (sb, self.load_register(rb)),
            0x51d => {
                let bars = self.cached_const_u32(0xf);
                let id = self.load_register(rb);
                (self.ir.emit_bitwise_and(ty, id, bars), sc)
            }
            _ => panic!("BAR with barrier and thread count in one register"),
        };
        let pred = if inst >> 77 & 7 == 2 { self.pred_bit(false, (inst >> 87 & 7) as u32, inst >> 90 & 1 != 0) } else { self.const_u32_0 };
        let [two, five, flags] = [2, 5, 1 | (parks as u32) << 1 | (op as u32) << 3].map(|c| self.cached_const_u32(c));
        let pred = self.ir.emit_shift_left_logical(ty, pred, two);
        let count = self.ir.emit_shift_left_logical(ty, count, five);
        let state = self.ir.emit_bitwise_or(ty, flags, pred);
        let state = self.ir.emit_bitwise_or(ty, state, count);
        let first = self.cached_const_u32(BAR_ARRIVALS);
        let index = self.ir.emit_iadd(ty, first, bar);
        let ptr = self.sync_word(index);
        let old = self.ir.emit_load(ty, ptr);
        let state = self.ir.emit_select(ty, self.active, state, old);
        self.ir.emit_store(ptr, state);
    }
    pub fn bpt(&mut self, inst: u128) {
        let cop = (((inst >> 84) & 0x7) << 0);
        panic!("BPT mode {cop} breaks into a debugger, which a compute shader has none of");
    }
    pub fn bra(&mut self, inst: u128) {
        let cond = (((inst >> 32) & 0x3) << 0);
        let upq = (((inst >> 24) & 0x7) << 0) as u32;
        let upq_not = (((inst >> 27) & 0x1) << 0) as u32;
        // .DIV and .CONV branch on warp divergence, which is not tracked, so the warp counts as converged.
        if cond == 2 {
            return;
        }
        let mut target = self.cached_const_u32(branch_target(self.pc, inst).unwrap());
        if opcode(inst) == BRA_UPRED {
            let bit = self.pred_bit(true, upq, upq_not != 0);
            let taken = self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0);
            let next = self.ir.emit_load(self.type_u32[1], self.var_pc);
            target = self.ir.emit_select(self.type_u32[1], taken, target, next);
        }
        self.ir.emit_store(self.var_pc, target);
    }
    pub fn break_(&mut self, inst: u128) {
        self.store_barrier_token(inst, self.const_u32_0);
    }
    pub fn brx(&mut self, inst: u128) {
        self.jump_to_register(inst, true);
    }
    pub fn brxu(&mut self, inst: u128) {
        self.jump_to_register(inst, true);
    }
    pub fn bssy(&mut self, inst: u128) {
        let step = self.ir.emit_load(self.type_u32[1], self.var_step);
        let token = self.ir.emit_iadd(self.type_u32[1], step, self.const_u32_1);
        self.store_barrier_token(inst, token);
    }
    pub fn bsync(&mut self, inst: u128) {
        self.store_barrier_token(inst, self.const_u32_0);
    }
    pub fn call(&mut self, inst: u128) {
        match opcode(inst) {
            CALL_ABS_R | CALL_REL_R | 0x1943 | 0x1944 => self.jump_to_register(inst, matches!(opcode(inst), CALL_REL_R | 0x1944)),
            0xb43 => {
                let offset = self.cached_const_u32(((inst >> 40 & 0x3fff) as u32) << 2);
                let address = self.load_cbuf(ResourceKind::Cbuf((inst >> 54 & 0x1f) as u32), offset);
                let four = self.cached_const_u32(4);
                let target = self.ir.emit_shift_right_logical(self.type_u32[1], address, four);
                self.ir.emit_store(self.var_pc, target);
            }
            _ => {
                let target = branch_target(self.pc, inst).unwrap();
                let target = self.cached_const_u32(target);
                self.ir.emit_store(self.var_pc, target);
            }
        }
    }
    // The host driver owns dependency scoreboarding.
    pub fn depbar(&mut self, _inst: u128) {}
    pub fn exit(&mut self, _inst: u128) {
        let exit = self.cached_const_u32(u32::MAX);
        self.ir.emit_store(self.var_pc, exit);
    }
    pub fn jmp(&mut self, inst: u128) {
        let op = opcode(inst);
        let mut address = match op {
            JMP_ABS | 0x194a | 0x154a => self.cached_const_u32((branch_offset(inst) as u32) << 4),
            0xb4a | 0x1b4a | 0x174a => {
                let offset = self.cached_const_u32(((inst >> 40 & 0x3fff) as u32) << 2);
                self.load_cbuf(ResourceKind::Cbuf((inst >> 54 & 0x1f) as u32), offset)
            }
            _ => unreachable!(),
        };
        if matches!(op, 0x194a | 0x1b4a) {
            let ur = self.load_ureg((inst >> 24 & 0x3f) as u32);
            address = self.ir.emit_iadd(self.type_u32[1], address, ur);
        }
        let four = self.cached_const_u32(4);
        let target = self.ir.emit_shift_right_logical(self.type_u32[1], address, four);
        if matches!(op, 0x154a | 0x174a) {
            let upq = (inst >> 24 & 7) as u32;
            let taken = self.pred_bit(true, upq, inst >> 27 & 1 != 0);
            let taken = self.ir.emit_inot_equal(self.type_bool[1], taken, self.const_u32_0);
            let next = self.ir.emit_load(self.type_u32[1], self.var_pc);
            let target = self.ir.emit_select(self.type_u32[1], taken, target, next);
            return self.ir.emit_store(self.var_pc, target);
        }
        self.ir.emit_store(self.var_pc, target);
    }
    pub fn jmx(&mut self, inst: u128) {
        self.jump_to_register(inst, false);
    }
    pub fn jmxu(&mut self, inst: u128) {
        self.jump_to_register(inst, false);
    }
    pub fn kill(&mut self, inst: u128) {
        // KILL is pixel-only on hardware, so a compute invocation that reaches one just ends like at EXIT.
        self.exit(inst);
    }
    pub fn lepc(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let address = self.cached_const_u32((self.pc + 1) * 16);
        let ty = self.type_u64();
        let address = self.ir.emit_u_convert(ty, address);
        self.store_pair(rd, address);
    }
    pub fn nanosleep(&mut self, inst: u128) {
        const MAX_NS: u32 = 1_000_000;
        const FALLBACK_ITERS: u32 = 4096;
        if matches!(self.timing, Timing::Off) {
            return;
        }
        let timing = std::mem::replace(&mut self.timing, Timing::Off);
        let mut ns = self.alu_src(inst);
        let cap = self.cached_const_u32(MAX_NS);
        let over = self.ir.emit_ugreater_than(self.type_bool[1], ns, cap);
        ns = self.ir.emit_select(self.type_u32[1], over, cap, ns);
        match timing {
            Timing::Off => {}
            Timing::Clock { period, emitted } => {
                if !emitted {
                    self.ir.emit_capability(spirv::capability::SHADER_CLOCK);
                    self.ir.emit_extension("SPV_KHR_shader_clock");
                }
                let scope = self.cached_const_u32(spirv::scope::DEVICE);
                let start = self.ir.emit_read_clock(self.type_u32[2], scope);
                let start = self.ir.emit_composite_extract(self.type_u32[1], start, &[0]);
                let nanos = self.ir.emit_convert_u_to_f(self.type_f32[1], ns);
                let period_id = self.ir.emit_constant_typed(self.type_f32[1], period);
                let ticks = self.ir.emit_fdiv(self.type_f32[1], nanos, period_id);
                let ticks = self.ir.emit_convert_f_to_u(self.type_u32[1], ticks);
                let [header, body, done] = [(); 3].map(|_| self.ir.alloc_id());
                self.ir.emit_branch(header);
                self.ir.emit_label_id(header);
                let now = self.ir.emit_read_clock(self.type_u32[2], scope);
                let now = self.ir.emit_composite_extract(self.type_u32[1], now, &[0]);
                let elapsed = self.ir.emit_isub(self.type_u32[1], now, start);
                let elapsed = self.ir.emit_ugreater_than_equal(self.type_bool[1], elapsed, ticks);
                self.ir.emit_loop_merge(done, body, 0);
                self.ir.emit_branch_conditional(elapsed, done, body);
                self.ir.emit_label_id(body);
                self.ir.emit_branch(header);
                self.ir.emit_label_id(done);
                self.timing = Timing::Clock { period, emitted: true };
            }
            Timing::Fallback => {
                let limit = self.cached_const_u32(FALLBACK_ITERS);
                let over = self.ir.emit_ugreater_than(self.type_bool[1], ns, limit);
                let limit = self.ir.emit_select(self.type_u32[1], over, limit, ns);
                let counter = self.hint_buffer();
                let ptr = self.ssbo_word(counter, self.const_u32_0, PMTRIG_WORDS as u32);
                let [scope, relaxed] = [spirv::scope::DEVICE, spirv::memory_semantics::RELAXED].map(|v| self.cached_const_u32(v));
                let old_step = self.ir.emit_load(self.type_u32[1], self.var_step);
                self.ir.emit_store(self.var_step, self.const_u32_0);
                let [header, body, done] = [(); 3].map(|_| self.ir.alloc_id());
                self.ir.emit_branch(header);
                self.ir.emit_label_id(header);
                let i = self.ir.emit_load(self.type_u32[1], self.var_step);
                let waits = self.ir.emit_uless_than(self.type_bool[1], i, limit);
                self.ir.emit_loop_merge(done, body, 0);
                self.ir.emit_branch_conditional(waits, body, done);
                self.ir.emit_label_id(body);
                self.ir.emit_atomic_iadd(self.type_u32[1], ptr, scope, relaxed, self.const_u32_0);
                let i = self.ir.emit_iadd(self.type_u32[1], i, self.const_u32_1);
                self.ir.emit_store(self.var_step, i);
                self.ir.emit_branch(header);
                self.ir.emit_label_id(done);
                self.ir.emit_store(self.var_step, old_step);
                self.timing = Timing::Fallback;
            }
        }
    }
    pub fn nanotrap(&mut self, inst: u128) {
        self.exit(inst);
    }
    pub fn r2b(&mut self, _inst: u128) {}
    pub fn ret(&mut self, inst: u128) {
        self.jump_to_register(inst, inst >> 85 & 1 == 0);
    }
    pub fn rpcmov(&mut self, inst: u128) {
        let op = opcode(inst);
        if op == 0x353 {
            let value = self.ir.emit_load(self.type_u32[1], self.var_rpc[(inst >> 31 & 1) as usize]);
            return self.store_register((inst >> 16 & 0xff) as u32, value);
        }
        let values = match op {
            0x352 => [self.load_register((inst >> 32 & 0xff) as u32), self.const_u32_0],
            0x952 => [self.cached_const_u32((inst >> 32) as u32), self.const_u32_0],
            0xb52 => {
                let offset = self.cached_const_u32(((inst >> 40 & 0x3fff) as u32) << 2);
                [self.load_cbuf(ResourceKind::Cbuf((inst >> 54 & 0x1f) as u32), offset), self.const_u32_0]
            }
            0x1d52 => [self.load_ureg((inst >> 32 & 0x3f) as u32), self.const_u32_0],
            0x954 => {
                let address = ((inst >> 34) & 0x7fff_ffff_ffff) << 2;
                [self.cached_const_u32(address as u32), self.cached_const_u32((address >> 32) as u32)]
            }
            0xb54 => {
                let offset = ((inst >> 40 & 0x3fff) as u32) << 2;
                let bank = ResourceKind::Cbuf((inst >> 54 & 0x1f) as u32);
                let lo = self.cached_const_u32(offset);
                let hi = self.cached_const_u32(offset + 4);
                [self.load_cbuf(bank, lo), self.load_cbuf(bank, hi)]
            }
            0x1d54 => {
                let ur = (inst >> 32 & 0x3f) as u32;
                [self.load_ureg(ur), self.load_ureg(nth_reg(ur, URZ, 1))]
            }
            0x1b52 | 0x1b54 => {
                let bank = self.load_ureg((inst >> 32 & 0x3f) as u32);
                let offset = ((inst >> 40 & 0x3fff) as u32) << 2;
                let lo = self.cached_const_u32(offset);
                let hi = self.cached_const_u32(offset + 4);
                [self.load_dynamic_cbuf(bank, lo), self.load_dynamic_cbuf(bank, hi)]
            }
            _ => unreachable!(),
        };
        if matches!(op, 0x1b54 | 0xb54 | 0x954 | 0x1d54) {
            self.ir.emit_store(self.var_rpc[0], values[0]);
            self.ir.emit_store(self.var_rpc[1], values[1]);
        } else {
            self.ir.emit_store(self.var_rpc[(inst >> 31 & 1) as usize], values[0]);
        }
    }
    pub fn warpsync(&mut self, inst: u128) {
        assert!(inst >> 86 & 1 == 0 && inst >> 87 & 0xf == PT as u128, "WARPSYNC.EXCLUSIVE or with a condition predicate");
        let mask = self.alu_src(inst);
        let full = self.cached_const_u32(u32::MAX);
        let partial = self.ir.emit_inot_equal(self.type_bool[1], mask, full);
        let waits = self.ir.emit_select(self.type_u32[1], partial, mask, self.const_u32_0);
        let ptr = self.barrier_token(WARPSYNC_MASK);
        self.ir.emit_store(ptr, waits);
    }
    // SPIR-V and Vulkan expose no subgroup yield.
    pub fn yield_(&mut self, _inst: u128) {}
}
