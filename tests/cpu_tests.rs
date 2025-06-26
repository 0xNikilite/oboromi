// tests/cpu_tests.rs

use oboromi::cpu::{CPU, Flags};
use oboromi::memory::Memory;

#[test]
fn memory_read_write() {
    let mut mem = Memory::new(1024);
    mem.write_byte(0, 0xAA);
    assert_eq!(mem.read_byte(0), 0xAA);
    mem.write_u32(4, 0xDEADBEEF);
    assert_eq!(mem.read_u32(4), 0xDEADBEEF);
}

#[test]
fn cpu_nop_and_addi() {
    let mut cpu = CPU::new(64);
    // NOP
    cpu.memory.write_u32(0, 0xD503201F);
    cpu.step();
    assert_eq!(cpu.regs.pc, 4);

    // ADDI X0, X0, #5
    cpu.reset();
    cpu.regs.x[0] = 10;
    cpu.memory.write_u32(0, 0x91001400);
    cpu.step();
    assert_eq!(cpu.regs.x[0], 15);
    assert!(!cpu.regs.flags.contains(Flags::ZERO));
}

#[test]
fn cpu_branch_and_ret() {
    let mut cpu = CPU::new(64);
    cpu.memory.write_u32(0, 0x14000001); // B +1
    cpu.step();
    assert_eq!(cpu.regs.pc, 4);

    cpu.reset();
    cpu.regs.x[30] = 0x20;
    cpu.memory.write_u32(0, 0xD65F03C0); // RET
    cpu.step();
    assert_eq!(cpu.regs.pc, 0x20);
}