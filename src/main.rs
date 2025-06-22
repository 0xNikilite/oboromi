use oboromi::cpu::CPU;
use oboromi::memory::Memory;

/// Simple helper to decode basic ARM64 instruction fields from a 32-bit opcode.
/// Returns a tuple (sf, opc, rn, rd).
fn decode_arm64_fields(opcode: u32) -> (u8, u8, u8, u8) {
    let sf  = ((opcode >> 31) & 0x1)  as u8; // sign flag width
    let opc = ((opcode >> 29) & 0x3)  as u8; // operation code
    let rn  = ((opcode >> 5)  & 0x1F) as u8; // first source register
    let rd  = ( opcode        & 0x1F) as u8; // destination register
    (sf, opc, rn, rd)
}

fn main() {
    // 1) Test basic Memory read/write
    let mut mem = Memory::new(64 * 1024 * 1024); // 64 MiB RAM
    mem.write_byte(10, 0xAB);
    assert_eq!(mem.read_byte(10), 0xAB);
    mem.write_byte(100, 0x11);
    mem.write_byte(101, 0x22);
    mem.write_byte(102, 0x33);
    mem.write_byte(103, 0x44);
    assert_eq!(mem.read_u32(100), 0x44332211);
    println!("✅ Memory tests passed");

    // 2) Test CPU–Memory integration with NOP
    let mut cpu = CPU::new(1024); // 1 KiB RAM
    let nop = 0xD503201F_u32.to_le_bytes();
    for i in 0..4 {
        cpu.memory.write_byte(i, nop[i]);
    }
    assert_eq!(cpu.regs.pc, 0);
    cpu.step();
    assert_eq!(cpu.regs.pc, 4);
    println!("✅ NOP executed correctly, PC = {}", cpu.regs.pc);

    // 4) Test dynamic MOV Xd, #imm decoding
    // MOV X5, #0x2A  => ORR X5, XZR, #0x2A
    let mut cpu2 = CPU::new(1024);
    cpu2.regs.pc = 0;
    // build opcode: sf=1 (bit31), opc=01 (bits30-29), imm12=0x2A at bits[21:10], rd=5 at bits[4:0]
    let mov_5_2a = (1u32 << 31)
        | (0b01 << 29)
        | (0x2A << 10)
        | 5;
    let mov_bytes = mov_5_2a.to_le_bytes();
    for i in 0..4 {
        cpu2.memory.write_byte(i, mov_bytes[i]);
    }
    cpu2.step();
    assert_eq!(cpu2.regs.x[5], 0x2A);
    println!("✅ MOV X5, #0x2A executed correctly, X5 = {}", cpu2.regs.x[5]);

    // 5) Decode and print fields of all tested opcodes
    for &opcode in &[0xD503201F, 0xD2802674, mov_5_2a] {
        let (sf, opc, rn, rd) = decode_arm64_fields(opcode);
        println!("Decoded 0x{:08X}: sf={}, opc={}, rn={}, rd={}", opcode, sf, opc, rn, rd);
    }

    // 5) Test istruzione reale: ADD X1, X1, X2 (opcode: 0x8B020021)
    let add_real = 0x8B020021_u32.to_le_bytes();
    for i in 0..4 {
        cpu.memory.write_byte(i, add_real[i]);
    }
    cpu.regs.x[1] = 10;
    cpu.regs.x[2] = 32;
    cpu.regs.pc = 0;
    cpu.step();
    assert_eq!(cpu.regs.x[1], 42);
    println!("✅ Real ADD executed correctly: X1 = {}", cpu.regs.x[1]);
}

