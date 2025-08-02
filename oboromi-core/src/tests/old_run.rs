use crate::cpu::{CPU, Flags};
use crate::memory::Memory;

pub fn run_tests() -> Vec<String> {
    let mut results = Vec::new();

    // 1) Memory subsystem test
    let mut mem = Memory::new(64 * 1024 * 1024);
    mem.write_byte(10, 0xAB);
    assert_eq!(mem.read_byte(10), 0xAB);
    mem.write_byte(100, 0x11);
    mem.write_byte(101, 0x22);
    mem.write_byte(102, 0x33);
    mem.write_byte(103, 0x44);
    assert_eq!(mem.read_u32(100), 0x4433_2211);
    println!("✅ Memory OK");

    // 2) NOP instruction test
    let mut cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    let nop = 0xD503201F_u32.to_le_bytes();
    for i in 0..4 { cpu.memory.write_byte(i, nop[i]); }
    cpu.step();
    assert_eq!(cpu.regs.pc, 4);
    println!("✅ NOP OK");

    // 3) ADD immediate test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[1] = 5;
    let addi = 0x9100_0821_u32.to_le_bytes(); // ADD X1, X1, #0x2
    for i in 0..4 { cpu.memory.write_byte(i, addi[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[1], 7);
    println!("✅ ADDI OK");

    // 4) SUB immediate test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[2] = 10;
    let subi = 0xD100_0442_u32.to_le_bytes(); // SUB X2, X2, #0x1
    for i in 0..4 { cpu.memory.write_byte(i, subi[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[2], 9);
    println!("✅ SUBI OK");

    // 5) ADD register test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[0] = 7;
    cpu.regs.x[1] = 3;
    let addr = 0x8B01_0000_u32.to_le_bytes(); // ADD X0, X0, X1
    for i in 0..4 { cpu.memory.write_byte(i, addr[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[0], 10);
    println!("✅ ADDR OK");

    // 6) SUB register test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[3] = 8;
    cpu.regs.x[4] = 2;
    let subr = 0xCB04_0063_u32.to_le_bytes(); // SUB X3, X3, X4
    for i in 0..4 { cpu.memory.write_byte(i, subr[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[3], 6);
    println!("✅ SUBR OK");

    // 7) AND register test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[5] = 0b1010;
    cpu.regs.x[6] = 0b1100;
    let andr = 0x8A06_00A5_u32.to_le_bytes(); // AND X5, X5, X6
    for i in 0..4 { cpu.memory.write_byte(i, andr[i]); }
    cpu.step();
    assert_eq!(cpu.regs.x[5], 0b1000);
    println!("✅ AND OK");

    // 8) CMP (SUBS XZR, X7, X8) test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[7] = 3;
    cpu.regs.x[8] = 3;
    let cmp = 0xEB08_00FF_u32.to_le_bytes(); // CMP X7, X8
    for i in 0..4 { cpu.memory.write_byte(i, cmp[i]); }
    cpu.step();
    assert!(cpu.regs.flags.contains(Flags::ZERO));
    println!("✅ CMP OK");

    // 9) LDR/STR immediate (64-bit) test
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

    // 10) Branch test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    let b = 0x1400_0002_u32.to_le_bytes(); // B +2
    for i in 0..4 { cpu.memory.write_byte(i, b[i]); }
    cpu.step();
    assert_eq!(cpu.regs.pc, 8);
    println!("✅ B OK");

    // 11) RET test
    cpu = CPU::new(1024);
    cpu.regs.pc = 0;
    cpu.regs.x[30] = 16;
    let ret = 0xD65F_03C0_u32.to_le_bytes();
    for i in 0..4 { cpu.memory.write_byte(i, ret[i]); }
    cpu.step();
    assert_eq!(cpu.regs.pc, 16);
    println!("✅ RET OK");

    // 12) MMU Basic Functionality Test
    println!("\nTesting MMU Basic Functionality...");
    let mut cpu = CPU::new(4096); // 4KB memory for MMU test

    let vaddr = 0x10;
    let paddr = cpu.memory.mmu.translate(vaddr).expect("Translation failed");
    println!("MMU: 0x{:x} → 0x{:x}", vaddr, paddr);
    assert_eq!(vaddr, paddr);

    // Test: write through MMU
    cpu.memory.write_byte(vaddr as usize, 0xAB);
    let value = cpu.memory.ram[paddr as usize];
    println!("Wrote 0xAB to vaddr 0x{:x} (paddr 0x{:x}), read back: 0x{:x}", 
        vaddr, paddr, value);
    assert_eq!(value, 0xAB);

    // Test: read through MMU
    let read_value = cpu.memory.read_byte(vaddr as usize);
    println!("Read from vaddr 0x{:x}: 0x{:x}", vaddr, read_value);
    assert_eq!(read_value, 0xAB);

    println!("✅ MMU Basic Functionality OK");
    results.push("All tests passed".to_string());
    results
}