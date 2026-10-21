//! Backend-neutral ARM64 instruction conformance tests.
//!
//! A CPU implementation can run these vectors by implementing
//! [`Arm64TestBackend`]. The harness deliberately has no dependency on the
//! current emulator backend.

use std::time::{Duration, Instant};

pub const TEST_CODE_BASE: u64 = 0x1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedProgramCounter {
    Sequential,
    Value(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct InstructionTestCase {
    pub name: &'static str,
    pub instructions: &'static [u32],
    pub initial_registers: &'static [(u8, u64)],
    pub expected_registers: &'static [(u8, u64)],
    pub expected_pc: ExpectedProgramCounter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionTestResult {
    pub name: &'static str,
    pub passed: bool,
    pub message: String,
    pub duration: Duration,
}

/// The minimal interface an ARM64 CPU implementation needs to run this suite.
pub trait Arm64TestBackend {
    fn reset(&mut self);
    fn write_u32(&mut self, address: u64, value: u32);
    fn set_register(&mut self, register: u8, value: u64);
    fn get_register(&self, register: u8) -> u64;
    fn set_pc(&mut self, value: u64);
    fn get_pc(&self) -> u64;
    fn step(&mut self) -> Result<(), String>;
}

const NOP: u32 = 0xD503_201F;
const ADD_X1_X1_2: u32 = 0x9100_0821;
const SUB_X2_X2_1: u32 = 0xD100_0442;
const ADD_X0_X0_X1: u32 = 0x8B01_0000;
const MOV_X3_X4: u32 = 0xAA04_03E3;
const RET: u32 = 0xD65F_03C0;
const BRANCH_FORWARD_2: u32 = 0x1400_0002;
const ADD_X0_X0_5: u32 = 0x9100_1400;
const SUB_X1_X1_3: u32 = 0xD100_0C21;

const NO_REGISTERS: &[(u8, u64)] = &[];
const ADD_IMMEDIATE_SETUP: &[(u8, u64)] = &[(1, 5)];
const ADD_IMMEDIATE_EXPECTED: &[(u8, u64)] = &[(1, 7)];
const SUB_IMMEDIATE_SETUP: &[(u8, u64)] = &[(2, 10)];
const SUB_IMMEDIATE_EXPECTED: &[(u8, u64)] = &[(2, 9)];
const ADD_REGISTER_SETUP: &[(u8, u64)] = &[(0, 7), (1, 3)];
const ADD_REGISTER_EXPECTED: &[(u8, u64)] = &[(0, 10)];
const MOV_REGISTER_SETUP: &[(u8, u64)] = &[(4, 0xDEAD_BEEF)];
const MOV_REGISTER_EXPECTED: &[(u8, u64)] = &[(3, 0xDEAD_BEEF)];
const RET_SETUP: &[(u8, u64)] = &[(30, 0x2000)];
const MULTI_OP_SETUP: &[(u8, u64)] = &[(0, 10), (1, 20)];
const MULTI_OP_EXPECTED: &[(u8, u64)] = &[(0, 32), (1, 17)];

const INSTRUCTION_CASES: &[InstructionTestCase] = &[
    InstructionTestCase {
        name: "NOP",
        instructions: &[NOP],
        initial_registers: NO_REGISTERS,
        expected_registers: NO_REGISTERS,
        expected_pc: ExpectedProgramCounter::Sequential,
    },
    InstructionTestCase {
        name: "ADD immediate",
        instructions: &[ADD_X1_X1_2],
        initial_registers: ADD_IMMEDIATE_SETUP,
        expected_registers: ADD_IMMEDIATE_EXPECTED,
        expected_pc: ExpectedProgramCounter::Sequential,
    },
    InstructionTestCase {
        name: "SUB immediate",
        instructions: &[SUB_X2_X2_1],
        initial_registers: SUB_IMMEDIATE_SETUP,
        expected_registers: SUB_IMMEDIATE_EXPECTED,
        expected_pc: ExpectedProgramCounter::Sequential,
    },
    InstructionTestCase {
        name: "ADD register",
        instructions: &[ADD_X0_X0_X1],
        initial_registers: ADD_REGISTER_SETUP,
        expected_registers: ADD_REGISTER_EXPECTED,
        expected_pc: ExpectedProgramCounter::Sequential,
    },
    InstructionTestCase {
        name: "MOV register alias",
        instructions: &[MOV_X3_X4],
        initial_registers: MOV_REGISTER_SETUP,
        expected_registers: MOV_REGISTER_EXPECTED,
        expected_pc: ExpectedProgramCounter::Sequential,
    },
    InstructionTestCase {
        name: "Multiple arithmetic instructions",
        instructions: &[ADD_X0_X0_5, SUB_X1_X1_3, ADD_X0_X0_X1],
        initial_registers: MULTI_OP_SETUP,
        expected_registers: MULTI_OP_EXPECTED,
        expected_pc: ExpectedProgramCounter::Sequential,
    },
    InstructionTestCase {
        name: "RET",
        instructions: &[RET],
        initial_registers: RET_SETUP,
        expected_registers: NO_REGISTERS,
        expected_pc: ExpectedProgramCounter::Value(0x2000),
    },
    InstructionTestCase {
        name: "Unconditional branch",
        instructions: &[BRANCH_FORWARD_2],
        initial_registers: NO_REGISTERS,
        expected_registers: NO_REGISTERS,
        expected_pc: ExpectedProgramCounter::Value(TEST_CODE_BASE + 8),
    },
];

pub fn instruction_cases() -> &'static [InstructionTestCase] {
    INSTRUCTION_CASES
}

pub fn run_instruction_tests<B: Arm64TestBackend>(backend: &mut B) -> Vec<InstructionTestResult> {
    INSTRUCTION_CASES
        .iter()
        .map(|test_case| run_case(backend, test_case))
        .collect()
}

fn run_case<B: Arm64TestBackend>(
    backend: &mut B,
    test_case: &InstructionTestCase,
) -> InstructionTestResult {
    let start = Instant::now();
    backend.reset();
    backend.set_pc(TEST_CODE_BASE);

    for (index, instruction) in test_case.instructions.iter().enumerate() {
        backend.write_u32(TEST_CODE_BASE + (index as u64 * 4), *instruction);
    }
    for &(register, value) in test_case.initial_registers {
        backend.set_register(register, value);
    }

    for (index, _) in test_case.instructions.iter().enumerate() {
        if let Err(error) = backend.step() {
            return failure(
                test_case.name,
                format!("step {} failed: {error}", index + 1),
                start.elapsed(),
            );
        }
    }

    for &(register, expected) in test_case.expected_registers {
        let actual = backend.get_register(register);
        if actual != expected {
            return failure(
                test_case.name,
                format!("X{register}: expected {expected:#018x}, got {actual:#018x}"),
                start.elapsed(),
            );
        }
    }

    let expected_pc = match test_case.expected_pc {
        ExpectedProgramCounter::Sequential => {
            TEST_CODE_BASE + (test_case.instructions.len() as u64 * 4)
        }
        ExpectedProgramCounter::Value(value) => value,
    };
    let actual_pc = backend.get_pc();
    if actual_pc != expected_pc {
        return failure(
            test_case.name,
            format!("PC: expected {expected_pc:#018x}, got {actual_pc:#018x}"),
            start.elapsed(),
        );
    }

    InstructionTestResult {
        name: test_case.name,
        passed: true,
        message: "PASS".to_string(),
        duration: start.elapsed(),
    }
}

fn failure(name: &'static str, message: String, duration: Duration) -> InstructionTestResult {
    InstructionTestResult {
        name,
        passed: false,
        message: format!("FAIL: {message}"),
        duration,
    }
}

#[cfg(test)]
mod tests {
    use super::{Arm64TestBackend, TEST_CODE_BASE, instruction_cases, run_instruction_tests};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct ReferenceBackend {
        registers: [u64; 31],
        pc: u64,
        memory: BTreeMap<u64, u32>,
    }

    impl Arm64TestBackend for ReferenceBackend {
        fn reset(&mut self) {
            self.registers = [0; 31];
            self.pc = 0;
            self.memory.clear();
        }

        fn write_u32(&mut self, address: u64, value: u32) {
            self.memory.insert(address, value);
        }

        fn set_register(&mut self, register: u8, value: u64) {
            self.registers[register as usize] = value;
        }

        fn get_register(&self, register: u8) -> u64 {
            self.registers[register as usize]
        }

        fn set_pc(&mut self, value: u64) {
            self.pc = value;
        }

        fn get_pc(&self) -> u64 {
            self.pc
        }

        fn step(&mut self) -> Result<(), String> {
            let instruction = self
                .memory
                .get(&self.pc)
                .copied()
                .ok_or_else(|| format!("no instruction at {:#x}", self.pc))?;
            match instruction {
                0xD503_201F => self.pc += 4,
                0x9100_0821 => {
                    self.registers[1] = self.registers[1].wrapping_add(2);
                    self.pc += 4;
                }
                0xD100_0442 => {
                    self.registers[2] = self.registers[2].wrapping_sub(1);
                    self.pc += 4;
                }
                0x8B01_0000 => {
                    self.registers[0] = self.registers[0].wrapping_add(self.registers[1]);
                    self.pc += 4;
                }
                0xAA04_03E3 => {
                    self.registers[3] = self.registers[4];
                    self.pc += 4;
                }
                0x1400_0002 => self.pc += 8,
                0x9100_1400 => {
                    self.registers[0] = self.registers[0].wrapping_add(5);
                    self.pc += 4;
                }
                0xD100_0C21 => {
                    self.registers[1] = self.registers[1].wrapping_sub(3);
                    self.pc += 4;
                }
                0xD65F_03C0 => self.pc = self.registers[30],
                _ => return Err(format!("unsupported test opcode {instruction:#010x}")),
            }
            Ok(())
        }
    }

    #[test]
    fn instruction_cases_cover_expected_vectors() {
        let cases = instruction_cases();
        assert_eq!(cases.len(), 8);
        assert!(cases.iter().all(|case| !case.instructions.is_empty()));
        assert!(cases.iter().all(|case| {
            (0..case.instructions.len()).all(|index| (TEST_CODE_BASE + index as u64 * 4) & 3 == 0)
        }));
        let words: Vec<_> = cases
            .iter()
            .flat_map(|case| case.instructions.iter().copied())
            .collect();
        assert_eq!(
            words,
            [
                0xD503_201F,
                0x9100_0821,
                0xD100_0442,
                0x8B01_0000,
                0xAA04_03E3,
                0x9100_1400,
                0xD100_0C21,
                0x8B01_0000,
                0xD65F_03C0,
                0x1400_0002,
            ]
        );
    }

    #[test]
    fn reference_backend_passes_instruction_suite() {
        let results = run_instruction_tests(&mut ReferenceBackend::default());
        assert_eq!(results.len(), instruction_cases().len());
        assert!(results.iter().all(|result| result.passed), "{results:#?}");
    }

    #[derive(Default)]
    struct UnsupportedBackend;

    impl Arm64TestBackend for UnsupportedBackend {
        fn reset(&mut self) {}
        fn write_u32(&mut self, _address: u64, _value: u32) {}
        fn set_register(&mut self, _register: u8, _value: u64) {}
        fn get_register(&self, _register: u8) -> u64 {
            0
        }
        fn set_pc(&mut self, _value: u64) {}
        fn get_pc(&self) -> u64 {
            0
        }
        fn step(&mut self) -> Result<(), String> {
            Err("instruction is not implemented".to_string())
        }
    }

    #[test]
    fn backend_errors_are_reported_per_test_case() {
        let results = run_instruction_tests(&mut UnsupportedBackend);
        assert_eq!(results.len(), instruction_cases().len());
        assert!(results.iter().all(|result| !result.passed));
        assert!(
            results
                .iter()
                .all(|result| result.message.contains("instruction is not implemented"))
        );
    }

    #[derive(Default)]
    struct NoOpBackend {
        pc: u64,
    }

    impl Arm64TestBackend for NoOpBackend {
        fn reset(&mut self) {
            self.pc = 0;
        }
        fn write_u32(&mut self, _address: u64, _value: u32) {}
        fn set_register(&mut self, _register: u8, _value: u64) {}
        fn get_register(&self, _register: u8) -> u64 {
            0
        }
        fn set_pc(&mut self, value: u64) {
            self.pc = value;
        }
        fn get_pc(&self) -> u64 {
            self.pc
        }
        fn step(&mut self) -> Result<(), String> {
            self.pc += 4;
            Ok(())
        }
    }

    #[test]
    fn register_mismatches_are_reported_as_failures() {
        let results = run_instruction_tests(&mut NoOpBackend::default());
        let add_test = results
            .iter()
            .find(|result| result.name == "ADD immediate")
            .unwrap();
        assert!(!add_test.passed);
        assert!(add_test.message.contains("X1"));
        assert!(add_test.message.contains("expected 0x0000000000000007"));
    }
}
