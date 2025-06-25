// src/main.rs

use oboromi::cpu::{CPU, Flags};
use oboromi::memory::Memory;

#[allow(dead_code)]
/// Helper to extract basic fields from an ARM64 opcode.
fn decode_arm64_fields(opcode: u32) -> (u8, u8, u8, u8) {
    let sf  = ((opcode >> 31) & 1) as u8;
    let opc = ((opcode >> 29) & 0x3) as u8;
    let rn  = ((opcode >> 5) & 0x1F) as u8;
    let rd  = (opcode & 0x1F) as u8;
    (sf, opc, rn, rd)
}

fn main() {
    // 1) Memory
    let mut mem = Memory::new(64 * 1024 * 1024);
    mem.write_byte(10, 0xAB);
    assert_eq!(mem.read_byte(10), 0xAB);
    mem.write_byte(100, 0x11);
    mem.write_byte(101, 0x22);
    mem.write_byte(102, 0x33);
    mem.write_byte(103, 0x44);
    assert_eq!(mem.read_u32(100), 0x4433_2211);
    println!("✅ Memory OK");

    // 2) NOP
    let mut cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    let nop = 0xD503201F_u32.to_le_bytes();
    for i in 0..4 { cpu.memory.write_byte(i, nop[i]); }
    cpu.step();
    assert_eq!(cpu.regs.pc, 4);
    println!("✅ NOP OK");

    // 3) ADD immediate
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[1] = 5;
    let addi = 0x9100_0821_u32.to_le_bytes(); // ADD X1, X1, #0x2
    for i in 0..4 { cpu.memory.write_byte(i, addi[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[1], 7);
    println!("✅ ADDI OK");

    // 4) SUB immediate
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[2] = 10;
    let subi = 0xD100_0442_u32.to_le_bytes(); // SUB X2, X2, #0x1
    for i in 0..4 { cpu.memory.write_byte(i, subi[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[2], 9);
    println!("✅ SUBI OK");

    // 5) ADD register
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[0] = 7;
    cpu.regs.x[1] = 3;
    let addr = 0x8B01_0000_u32.to_le_bytes(); // ADD X0, X0, X1
    for i in 0..4 { cpu.memory.write_byte(i, addr[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[0], 10);
    println!("✅ ADDR OK");

    // 6) SUB register
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[3] = 8;
    cpu.regs.x[4] = 2;
    let subr = 0xCB04_0063_u32.to_le_bytes(); // SUB X3, X3, X4
    for i in 0..4 { cpu.memory.write_byte(i, subr[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[3], 6);
    println!("✅ SUBR OK");

    // 7) AND register
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[5] = 0b1010;
    cpu.regs.x[6] = 0b1100;
    let andr = 0x8A06_00A5_u32.to_le_bytes(); // AND X5, X5, X6
    for i in 0..4 { cpu.memory.write_byte(i, andr[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[5], 0b1000);
    println!("✅ AND OK");

    // 8) CMP (SUBS XZR, X7, X8)
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[7] = 3;
    cpu.regs.x[8] = 3;
    let cmp = 0xEB08_00FF_u32.to_le_bytes(); // CMP X7, X8
    for i in 0..4 { cpu.memory.write_byte(i, cmp[i]); }
    cpu.step();
    assert!(cpu.regs.flags.contains(Flags::ZERO));
    println!("✅ CMP OK");

    // 9) LDR/STR immediate (64-bit)
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[9] = 100;
    cpu.regs.x[10] = 0x1234_5678;
    
    // STR X10, [X9, #16]
    let stri = 0xF900092A_u32.to_le_bytes();
    for i in 0..4 { cpu.memory.write_byte(i, stri[i]); }
    cpu.step();
    
    // LDR X11, [X9, #16]
    let ldri = 0xF940092B_u32.to_le_bytes();
    for i in 0..4 { cpu.memory.write_byte(i + 4, ldri[i]); }
    cpu.step();
    
    assert_eq!(cpu.regs.x[11], 0x1234_5678);
    println!("✅ LDR/STR OK");

    // 10) Branch
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    let b = 0x1400_0002_u32.to_le_bytes(); // B +2
    for i in 0..4 { cpu.memory.write_byte(i, b[i]); }
    cpu.step();
    assert_eq!(cpu.regs.pc, 8);
    println!("✅ B OK");

    // RET
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[30] = 16;
    let ret = 0xD65F_03C0_u32.to_le_bytes();
    for i in 0..4 { cpu.memory.write_byte(i, ret[i]); }
    cpu.step();
    assert_eq!(cpu.regs.pc, 16);
    println!("✅ RET OK");
}