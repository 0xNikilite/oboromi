use oboromi::cpu::CPU;
use oboromi::memory::Memory;

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
}
