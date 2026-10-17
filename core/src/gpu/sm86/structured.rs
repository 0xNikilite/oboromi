// Emits a structured plan as nested SPIR-V selections and loops inside one outer loop, whose merge every exit
// reaches. SPIR-V 2.11: a branch out of a construct reaches at most the innermost loop's merge or continue target,
// so an exit inside guest loops leaves the pc at u32::MAX and breaks them one at a time.
use super::cfg::{Ast, Cond, Pred};
use super::*;

impl Decoder<'_> {
    pub(super) fn translate_structured(&mut self, words: &[u128], plan: &[Ast]) {
        self.locals = Some(Locals { vars: BTreeMap::new(), entry: self.entry, start: self.ir.function_len() });
        self.ir.emit_store(self.var_pc, self.const_u32_0);
        let [header, body, cont, merge] = [(); 4].map(|_| self.ir.alloc_id());
        let live = self.ir.emit_load(self.type_u32[1], self.var_live);
        let live = self.ir.emit_inot_equal(self.type_bool[1], live, self.const_u32_0);
        self.ir.emit_branch(header);
        self.ir.emit_label_id(header);
        self.ir.emit_loop_merge(merge, cont, 0);
        self.ir.emit_branch_conditional(live, body, merge);
        self.ir.emit_label_id(body);
        if !self.emit_sequence(words, plan, [merge, cont]) {
            self.ir.emit_branch(merge);
        }
        self.ir.emit_label_id(cont);
        self.ir.emit_branch(header);
        self.ir.emit_label_id(merge);
        for (slot, var) in self.locals.take().unwrap().vars {
            let value = self.ir.emit_load(self.type_u32[1], var);
            let ptr = self.aggregate_slot(slot);
            self.ir.emit_store(ptr, value);
        }
    }

    // Returns whether the sequence ended in a branch out of the block it leaves the emitter in.
    fn emit_sequence(&mut self, words: &[u128], seq: &[Ast], [merge, cont]: [u32; 2]) -> bool {
        for ast in seq {
            match ast {
                Ast::Code(range) => {
                    for pc in range.clone() {
                        self.pc = pc;
                        self.translate_guarded(words[pc as usize]);
                    }
                }
                Ast::If(_, then, otherwise) if then.is_empty() && otherwise.is_empty() => {}
                Ast::If(cond, then, otherwise) => {
                    let cond = match cond {
                        Cond::All(preds) => self.holds(preds),
                        Cond::Exited => {
                            let pc = self.ir.emit_load(self.type_u32[1], self.var_pc);
                            let exited = self.cached_const_u32(u32::MAX);
                            self.ir.emit_iequal(self.type_bool[1], pc, exited)
                        }
                    };
                    // SPIR-V 2.11: each selection header names a merge block of its own, and one no path reaches only
                    // holds OpUnreachable.
                    let after = self.ir.alloc_id();
                    let labels = [then, otherwise].map(|branch| if branch.is_empty() { after } else { self.ir.alloc_id() });
                    self.ir.emit_selection_merge(after, 0);
                    self.ir.emit_branch_conditional(cond, labels[0], labels[1]);
                    let mut ended = true;
                    for (branch, label) in [then, otherwise].into_iter().zip(labels).filter(|(branch, _)| !branch.is_empty()) {
                        self.ir.emit_label_id(label);
                        if !self.emit_sequence(words, branch, [merge, cont]) {
                            self.ir.emit_branch(after);
                            ended = false;
                        }
                    }
                    self.ir.emit_label_id(after);
                    if ended && !then.is_empty() && !otherwise.is_empty() {
                        self.ir.emit_unreachable();
                        return true;
                    }
                }
                Ast::Loop(body) => {
                    // SPIR-V 2.11: OpLoopMerge directly precedes the header's branch and only the continue target
                    // branches back to the header.
                    let [header, start, cont, merge] = [(); 4].map(|_| self.ir.alloc_id());
                    self.ir.emit_branch(header);
                    self.ir.emit_label_id(header);
                    self.ir.emit_loop_merge(merge, cont, 0);
                    self.ir.emit_branch(start);
                    self.ir.emit_label_id(start);
                    if !self.emit_sequence(words, body, [merge, cont]) {
                        self.ir.emit_branch(cont);
                    }
                    self.ir.emit_label_id(cont);
                    self.ir.emit_branch(header);
                    self.ir.emit_label_id(merge);
                }
                Ast::Break => {
                    self.ir.emit_branch(merge);
                    return true;
                }
                Ast::Continue => {
                    self.ir.emit_branch(cont);
                    return true;
                }
                Ast::Exit => {
                    self.exit(0);
                    self.ir.emit_branch(merge);
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn holds(&mut self, preds: &[Pred]) -> u32 {
        let bits: Vec<u32> = preds.iter().map(|pred| {
            let bit = self.pred_bit(pred.uniform, pred.index, pred.not);
            self.ir.emit_inot_equal(self.type_bool[1], bit, self.const_u32_0)
        }).collect();
        bits.into_iter().reduce(|a, b| self.ir.emit_logical_and(self.type_bool[1], a, b)).unwrap()
    }
}
