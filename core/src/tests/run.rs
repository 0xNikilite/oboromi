//! A64 programs run by the core tests and the GUI.

use crate::cpu::cpu_manager::{BackendError, CpuManager, Stop};
use rapid_arm_emu::address_space::PAGE_SIZE;
use rapid_arm_emu::handler::Exception;
use rapid_arm_emu::state::ExecState;
use std::io::Write;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

const CODE_BASE: u64 = 0x1000;
const STACK_TOP: u64 = 0x8000;
const BRK: u32 = 0xd420_0000;
const TIMEOUT: Duration = Duration::from_secs(5);

struct Case {
    name: &'static str,
    words: &'static [u32],
    registers: &'static [(usize, u64)],
    expected: &'static [(usize, u64)],
    stop_word: usize,
}

const CASES: &[Case] = &[
    Case {
        name: "NOP",
        words: &[0xd503_201f],
        registers: &[],
        expected: &[],
        stop_word: 1,
    },
    Case {
        name: "ADD immediate",
        words: &[0x9100_0821],
        registers: &[(1, 5)],
        expected: &[(1, 7)],
        stop_word: 1,
    },
    Case {
        name: "SUB immediate",
        words: &[0xd100_0442],
        registers: &[(2, 10)],
        expected: &[(2, 9)],
        stop_word: 1,
    },
    Case {
        name: "ADD register",
        words: &[0x8b01_0000],
        registers: &[(0, 7), (1, 3)],
        expected: &[(0, 10)],
        stop_word: 1,
    },
    Case {
        name: "MOV register",
        words: &[0xaa04_03e3],
        registers: &[(4, 0xdead_beef)],
        expected: &[(3, 0xdead_beef)],
        stop_word: 1,
    },
    Case {
        name: "Stack store and reload",
        // SUB SP, SP, #16; STR X0, [SP]; LDR X1, [SP]; ADD SP, SP, #16
        words: &[0xd100_43ff, 0xf900_03e0, 0xf940_03e1, 0x9100_43ff],
        registers: &[(0, 0x0123_4567_89ab_cdef)],
        expected: &[(1, 0x0123_4567_89ab_cdef)],
        stop_word: 4,
    },
    Case {
        name: "RET to link register",
        words: &[0xd65f_03c0, BRK],
        registers: &[(30, CODE_BASE + 8)],
        expected: &[],
        stop_word: 2,
    },
    Case {
        name: "Counted branch loop",
        // SUBS X0, X0, #1; B.NE to the SUBS
        words: &[0xf100_0400, 0x54ff_ffe1],
        registers: &[(0, 3)],
        expected: &[(0, 0)],
        stop_word: 2,
    },
    Case {
        name: "Multiple arithmetic ops",
        words: &[0x9100_1400, 0xd100_0c21, 0x8b01_0000],
        registers: &[(0, 10), (1, 20)],
        expected: &[(0, 32), (1, 17)],
        stop_word: 3,
    },
];

fn run_case(case: &Case) -> Result<Stop, BackendError> {
    let mut image = tempfile::tempfile()?;
    image.set_len(PAGE_SIZE)?;
    for word in case.words.iter().copied().chain([BRK]) {
        image.write_all(&word.to_le_bytes())?;
    }

    let mut state = ExecState::initial();
    state.pc = CODE_BASE;
    state.sp = STACK_TOP;
    for &(reg, value) in case.registers {
        state.x[reg] = value;
    }

    let mut cpu = CpuManager::with_cores(NonZeroUsize::MIN)?;
    cpu.map_code(CODE_BASE, image)?;
    cpu.map_ram(STACK_TOP - PAGE_SIZE, PAGE_SIZE)?;
    cpu.start(0, state)?.wait_timeout(TIMEOUT)
}

fn verify(case: &Case, stop: Stop) -> Result<(), BackendError> {
    let pc = CODE_BASE + case.stop_word as u64 * 4;
    if !matches!(stop.exception, Exception::Breakpoint(0)) || stop.state.pc != pc {
        return Err(format!(
            "expected BRK #0 at {pc:#x}, got {:?} at {:#x}",
            stop.exception, stop.state.pc
        )
        .into());
    }
    if stop.state.sp != STACK_TOP {
        return Err(format!("expected SP = {STACK_TOP:#x}, got {:#x}", stop.state.sp).into());
    }
    for &(reg, expected) in case.expected {
        let actual = stop.state.x[reg];
        if actual != expected {
            return Err(format!("expected X{reg} = {expected:#x}, got {actual:#x}").into());
        }
    }
    Ok(())
}

pub fn run_tests() -> Vec<String> {
    let start = Instant::now();
    let mut lines = Vec::with_capacity(CASES.len() + 2);
    lines.push("ARM64 tests with rapid".to_owned());
    let mut passed = 0;
    for case in CASES {
        let begin = Instant::now();
        let result = run_case(case).and_then(|stop| verify(case, stop));
        let elapsed = begin.elapsed();
        let name = case.name;
        match result {
            Ok(()) => {
                passed += 1;
                lines.push(format!("Y {name} - PASS ({elapsed:?})"));
            }
            Err(error) => lines.push(format!("N {name} - FAIL: {error} ({elapsed:?})")),
        }
    }
    let total = CASES.len();
    let failed = total - passed;
    lines.push(format!(
        "Total: {total} ({passed} passed, {failed} failed) time {:?}",
        start.elapsed()
    ));
    lines
}
