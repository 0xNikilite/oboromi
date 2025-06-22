use oboromi::cpu::CPU;
use oboromi::memory::Memory;

/// Simple helper to decode basic ARM64 instruction fields from a 32-bit opcode.
/// This example just extracts some common fields by bitmask and shift.
/// Returns a tuple (sf, opc, rn, rd).
fn decode_arm64_fields(opcode: u32) -> (u8, u8, u8, u8) {
    // sf: bit 31 (1 bit)
    let sf = ((opcode >> 31) & 0x1) as u8;
    // opc: bits 29-30 (2 bits)
    let opc = ((opcode >> 29) & 0x3) as u8;
    // rn: bits 5-9 (5 bits)
    let rn = ((opcode >> 5) & 0x1F) as u8;
    // rd: bits 0-4 (5 bits)
    let rd = (opcode & 0x1F) as u8;

    (sf, opc, rn, rd)
}

fn main() {
    // 1) Test basic Memory read/write
    let mut mem = Memory::new(64 * 1024 * 1024);             // 64 MiB RAM
    mem.write_byte(10, 0xAB);
    assert_eq!(mem.read_byte(10), 0xAB);
    // Test read_u32: write 4 bytes and read the 32-bit word
    mem.write_byte(100, 0x11);
    mem.write_byte(101, 0x22);
    mem.write_byte(102, 0x33);
    mem.write_byte(103, 0x44);
    let word = mem.read_u32(100);
    assert_eq!(word, 0x44332211);

    println!("✅ Memory tests passed");

    // 2) Test CPU–Memory integration with NOP instruction
    let mut cpu = CPU::new(1024);            // 1 KiB RAM for testing
    // Load ARM64 NOP opcode (0xD503201F) into memory at address 0
    let nop = 0xD503201F_u32.to_le_bytes();
    for i in 0..4 {
        cpu.memory.write_byte(i, nop[i]);
    }
    // Initial PC should be 0
    assert_eq!(cpu.regs.pc, 0);
    // Execute one CPU cycle
    cpu.step();
    // After NOP, PC should advance by 4 bytes
    assert_eq!(cpu.regs.pc, 4);

    println!("✅ CPU–Memory integration test passed");

    // 3) Decode and print the fields of the NOP instruction
    let opcode = 0xD503201F;
    let (sf, opc, rn, rd) = decode_arm64_fields(opcode);
    println!("Decoded ARM64 instruction 0x{:08X}:", opcode);
    println!("  sf  (bit 31): {}", sf);
    println!("  opc (bits 29-30): {}", opc);
    println!("  rn  (bits 5-9): {}", rn);
    println!("  rd  (bits 0-4): {}", rd);
    println!("  Note: NOP means 'No Operation', this instruction does nothing but advance the PC.");
}
