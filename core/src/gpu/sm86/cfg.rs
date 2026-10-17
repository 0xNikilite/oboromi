// The structured plan: the program's basic blocks and their terminators, dominators and post-dominators, natural
// loops, and from those a tree of sequences, if/else, loops, breaks, continues and exits that follows every guest
// path. A program the tree cannot express exactly stays on the dispatcher, with the reason.
use super::*;
use std::ops::Range;

const BSSY: u32 = 0x945;
const BREAK: u32 = 0x942;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Translation {
    ForceDispatcher,
    #[default]
    Auto,
    ForceStructured,
}

#[derive(Clone, Copy, Debug)]
pub struct Pred {
    pub uniform: bool,
    pub index: u32,
    pub not: bool,
}

#[derive(Debug)]
pub enum Cond {
    All(Vec<Pred>),
    Exited,
}

#[derive(Debug)]
pub enum Ast {
    Code(Range<u32>),
    If(Cond, Vec<Ast>, Vec<Ast>),
    Loop(Vec<Ast>),
    Break,
    Continue,
    Exit,
}

enum Term {
    Goto(usize),
    Branch(Vec<Pred>, usize, usize),
}

struct Tree {
    idom: Vec<Option<usize>>,
    number: Vec<usize>,
}

impl Tree {
    fn new(succs: &[Vec<usize>], entry: usize) -> Self {
        let mut order = Vec::new();
        let mut number = vec![usize::MAX; succs.len()];
        let mut stack = vec![(entry, 0)];
        number[entry] = 0;
        while let Some((v, i)) = stack.pop() {
            match succs[v].get(i) {
                Some(&w) => {
                    stack.push((v, i + 1));
                    if number[w] == usize::MAX {
                        number[w] = 0;
                        stack.push((w, 0));
                    }
                }
                None => {
                    number[v] = order.len();
                    order.push(v);
                }
            }
        }
        let mut preds = vec![Vec::new(); succs.len()];
        for &v in &order {
            for &w in &succs[v] {
                preds[w].push(v);
            }
        }
        let mut tree = Self { idom: vec![None; succs.len()], number };
        tree.idom[entry] = Some(entry);
        let mut changed = true;
        while changed {
            changed = false;
            for &v in order.iter().rev().skip(1) {
                let idom = preds[v].iter().filter(|&&p| tree.idom[p].is_some()).fold(None, |acc, &p| Some(acc.map_or(p, |q| tree.common(p, q))));
                changed |= idom != tree.idom[v];
                tree.idom[v] = idom;
            }
        }
        tree
    }

    fn common(&self, mut a: usize, mut b: usize) -> usize {
        while a != b {
            while self.number[a] < self.number[b] {
                a = self.idom[a].unwrap();
            }
            while self.number[b] < self.number[a] {
                b = self.idom[b].unwrap();
            }
        }
        a
    }

    fn dominates(&self, a: usize, b: usize) -> bool {
        self.idom[b].is_some() && self.common(a, b) == a
    }
}

struct Builder {
    starts: Vec<u32>,
    code: Vec<Range<u32>>,
    terms: Vec<Term>,
    dom: Tree,
    postdom: Tree,
    merges: Vec<Option<usize>>,
    visited: Vec<bool>,
}

fn condition(inst: u128) -> Option<Pred> {
    if opcode(inst) == BRA_UPRED {
        return Some(Pred { uniform: true, index: (inst >> 24 & 7) as u32, not: inst >> 27 & 1 != 0 });
    }
    (matches!(opcode(inst), BRA | BRA_UREG) && inst >> 87 & 0xf != PT as u128).then(|| Pred { uniform: is_uniform(inst), index: (inst >> 87 & 7) as u32, not: inst >> 90 & 1 != 0 })
}

fn ends_block(inst: u128) -> bool {
    let branch = matches!(opcode(inst), BRA | BRA_UPRED | BRA_UREG) && inst >> 32 & 3 != 2 || matches!(opcode(inst), CALL_ABS | CALL_REL | JMP_ABS);
    inst >> 12 & 0xf != 0xf && (branch || exits(inst))
}

fn exits(inst: u128) -> bool {
    matches!(opcode(inst), EXIT | KILL | 0x95a | 0xb5a | 0x35a | 0x1b5a | 0x1d5a)
}

fn unsupported(pc: usize, inst: u128) -> Option<String> {
    let op = opcode(inst);
    let reason = if exchanges(inst) || partial_warpsync(inst) || matches!(op, 0x198c | 0x1f8c | 0x38c | 0x38d) {
        "collective"
    } else if dynamic_target(inst) || matches!(op, RET | 0x1950) {
        "dynamic branch target"
    } else if matches!(op, 0x1478 | 0x1878 | 0x1a78 | 0x1c78) {
        "indexed register"
    } else if op == BREAK {
        "BREAK"
    } else {
        return None;
    };
    Some(format!("{reason} {op:#x} at pc {pc}"))
}

pub fn plan(words: &[u128]) -> Result<Vec<Ast>, String> {
    if let Some(reason) = (0..).zip(words).find_map(|(pc, &inst)| unsupported(pc, inst)) {
        return Err(reason);
    }
    let len = words.len() as u32;
    let mut starts = vec![0, len];
    for (pc, &inst) in (0..).zip(words).filter(|&(_, &inst)| ends_block(inst)) {
        starts.push(pc + 1);
        starts.extend(branch_target(pc, inst));
    }
    starts.sort_unstable();
    starts.dedup();
    if starts.last() != Some(&len) {
        return Err("branch target outside the program".into());
    }
    let exit = starts.len() - 1;
    let node = |pc: u32| starts.binary_search(&pc).unwrap();
    let mut terms = Vec::new();
    let mut code = Vec::new();
    for window in starts.windows(2) {
        let (start, end) = (window[0], window[1]);
        let last = words[end as usize - 1];
        let fall = node(end);
        let (term, code_end) = if !ends_block(last) {
            (Term::Goto(fall), end)
        } else {
            let target = if exits(last) { exit } else { node(branch_target(end - 1, last).unwrap()) };
            let preds: Vec<Pred> = guard_pred(last).into_iter().chain(condition(last)).collect();
            (if preds.is_empty() { Term::Goto(target) } else { Term::Branch(preds, target, fall) }, end - 1)
        };
        terms.push(term);
        code.push(start..code_end);
    }
    let mut succs: Vec<Vec<usize>> = terms.iter().map(|term| match *term {
        Term::Goto(t) => vec![t],
        Term::Branch(_, t, f) => vec![t, f],
    }).collect();
    succs.push(Vec::new());
    let dom = Tree::new(&succs, 0);
    let mut reverse = vec![Vec::new(); succs.len()];
    let mut latches = vec![Vec::new(); succs.len()];
    for v in (0..exit).filter(|&v| dom.idom[v].is_some()) {
        for &w in &succs[v] {
            reverse[w].push(v);
            if dom.number[w] >= dom.number[v] {
                if !dom.dominates(w, v) {
                    return Err(format!("irreducible loop into pc {}", starts[w]));
                }
                latches[w].push(v);
            }
        }
    }
    let postdom = Tree::new(&reverse, exit);
    if let Some(v) = (0..exit).find(|&v| dom.idom[v].is_some() && postdom.idom[v].is_none()) {
        return Err(format!("no exit from pc {}", starts[v]));
    }
    let mut merges = vec![None; succs.len()];
    for (h, latches) in latches.iter().enumerate().filter(|(_, latches)| !latches.is_empty()) {
        let mut body = vec![false; succs.len()];
        body[h] = true;
        let mut work = latches.clone();
        while let Some(v) = work.pop() {
            if !std::mem::replace(&mut body[v], true) {
                work.extend(&reverse[v]);
            }
        }
        let targets = (0..exit).filter(|&v| body[v]).flat_map(|v| succs[v].iter().copied()).filter(|&w| !body[w] && w != exit);
        merges[h] = Some(targets.reduce(|a, b| postdom.common(a, b)).unwrap_or(exit));
    }
    let mut builder = Builder { starts, code, terms, dom, postdom, merges, visited: vec![false; exit + 1] };
    builder.check_tokens(words)?;
    builder.walk(0, None, None, false)
}

impl Builder {
    fn block(&self, pc: u32) -> usize {
        self.starts.partition_point(|&start| start <= pc) - 1
    }

    // A BSSY Bx and the BSYNC Bx at its target or just before it bracket a single-entry, single-exit region.
    fn check_tokens(&self, words: &[u128]) -> Result<(), String> {
        let token = |pc: u32| (words[pc as usize] >> 16 & 0xf) as u32;
        let reachable = |pc: u32| self.dom.idom[self.block(pc)].is_some();
        let at = |op| (0..words.len() as u32).filter(move |&pc| opcode(words[pc as usize]) == op);
        let mut paired = Vec::new();
        for pc in at(BSSY).filter(|&pc| reachable(pc)) {
            let target = (pc as i64 + 1 + branch_offset(words[pc as usize])) as u32;
            let simple = |q: u32| (q as usize) < words.len() && opcode(words[q as usize]) == BSYNC && token(q) == token(pc) && q > pc;
            let sync = [target, target.wrapping_sub(1)].into_iter().find(|&q| simple(q));
            let unguarded = |pc: u32| words[pc as usize] >> 12 & 0xf == PT as u128;
            match sync {
                Some(q) if unguarded(pc) && unguarded(q) && self.dom.dominates(self.block(pc), self.block(q)) && self.postdom.dominates(self.block(q), self.block(pc)) && !paired.contains(&q) => paired.push(q),
                _ => return Err(format!("BSSY B{} at pc {pc} without a simple BSYNC pair", token(pc))),
            }
        }
        match at(BSYNC).find(|&q| reachable(q) && !paired.contains(&q)) {
            Some(q) => Err(format!("BSYNC B{} at pc {q} without its BSSY", token(q))),
            None => Ok(()),
        }
    }

    // The innermost loop's header continues and its merge breaks; a selection merges at its immediate post-dominator,
    // or when that lies outside the loop, at a successor that stays inside, preferring the fall-through.
    fn walk(&mut self, mut cur: usize, stop: Option<usize>, ctx: Option<(usize, usize)>, mut entering: bool) -> Result<Vec<Ast>, String> {
        let exit = self.terms.len();
        let mut seq = Vec::new();
        loop {
            if !entering {
                let special = match ctx {
                    Some((h, _)) if cur == h => Some(Ast::Continue),
                    Some((_, m)) if cur == m => Some(Ast::Break),
                    _ if cur == exit => Some(Ast::Exit),
                    _ => None,
                };
                if let Some(ast) = special {
                    seq.push(ast);
                    return Ok(seq);
                }
                if Some(cur) == stop {
                    return Ok(seq);
                }
                if let Some(m) = self.merges[cur] {
                    let body = self.walk(cur, None, Some((cur, m)), true)?;
                    let exits = exits_from(&body);
                    seq.push(Ast::Loop(body));
                    if exits && m != exit {
                        seq.push(Ast::If(Cond::Exited, vec![Ast::Exit], Vec::new()));
                    }
                    cur = m;
                    continue;
                }
            }
            entering = false;
            let straight = matches!(self.terms[cur], Term::Goto(_)) && self.merges[cur].is_none();
            if std::mem::replace(&mut self.visited[cur], true) && !straight {
                return Err(format!("unstructured join at pc {}", self.starts[cur]));
            }
            if !self.code[cur].is_empty() {
                seq.push(Ast::Code(self.code[cur].clone()));
            }
            let (preds, t, f) = match &self.terms[cur] {
                Term::Goto(t) => {
                    cur = *t;
                    continue;
                }
                Term::Branch(preds, t, f) => (preds.clone(), *t, *f),
            };
            let outside = |n: usize| n == exit || ctx.is_some_and(|(h, m)| n == h || n == m || !self.dom.dominates(h, n) || m != exit && self.dom.dominates(m, n));
            let merge = match self.postdom.idom[cur].unwrap() {
                m if !outside(m) => Some(m),
                _ if !outside(f) => Some(f),
                _ if !outside(t) => Some(t),
                _ => None,
            };
            let then = self.walk(t, merge, ctx, false)?;
            let otherwise = self.walk(f, merge, ctx, false)?;
            seq.push(Ast::If(Cond::All(preds), then, otherwise));
            match merge {
                Some(m) => cur = m,
                None => return Ok(seq),
            }
        }
    }
}

fn exits_from(seq: &[Ast]) -> bool {
    seq.iter().any(|ast| match ast {
        Ast::Exit => true,
        Ast::If(_, then, otherwise) => exits_from(then) || exits_from(otherwise),
        _ => false,
    })
}
