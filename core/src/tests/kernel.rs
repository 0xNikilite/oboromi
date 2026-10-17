use crate::cpu::cpu_manager::CpuManager;
use crate::guest::load_flat_image;
use crate::kernel::{ALIAS, CODE, CURRENT_PROCESS, Error, HEAP, Kernel, MAIN_STACK, STACK, TLS};
use crate::nn::hipc::{Request, Response, Service};
use crate::tests::run::arm64::*;
use std::cell::RefCell;
use std::rc::Rc;

struct Echo;

impl Service for Echo {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        match cmd {
            0 => Response { data: request.args.clone(), ..Response::default() },
            _ => Response::unknown_command(cmd),
        }
    }
}

#[test]
fn guest_calls_svcs_and_round_trips_ipc() {
    const DATA: u16 = 0x100;
    const HELLO: &[u8] = b"hello from the guest";
    let program = [
        mrs_tpidrro(19),
        movz(9, (CODE.base >> 16) as u16, 16),
        add_imm(9, 9, DATA),
        add_imm(0, 9, 0),
        movz(1, HELLO.len() as u16, 0),
        svc(0x27),
        mov_reg(20, 0),
        movz(1, 0x40, 16),
        svc(0x01),
        mov_reg(21, 0),
        mov_reg(22, 1),
        movz(1, 7, 0),
        movz(2, CURRENT_PROCESS as u16, 0),
        movk(2, (CURRENT_PROCESS >> 16) as u16, 16),
        movz(3, 0, 0),
        svc(0x29),
        mov_reg(23, 0),
        mov_reg(24, 1),
        add_imm(1, 9, 0x20),
        svc(0x1f),
        mov_reg(25, 0),
        mov_reg(26, 1),
        add_imm(1, 9, 0x30),
        ldp(2, 3, 1, 0),
        stp(2, 3, 19, 0),
        ldp(2, 3, 1, 16),
        stp(2, 3, 19, 16),
        ldp(2, 3, 1, 32),
        stp(2, 3, 19, 32),
        mov_reg(0, 26),
        svc(0x21),
        mov_reg(27, 0),
        svc(0x7f),
        mov_reg(28, 0),
        svc(0x07),
    ];
    // libnx cmifMakeRequest for command 0 with two u32 arguments: 16 bytes padding + SFCI + 8.
    let request = [4, 10, 0, 0, u32::from_le_bytes(*b"SFCI"), 0, 0, 0, 0x1111, 0x2222, 0, 0];
    let mut image: Vec<u8> = program.iter().flat_map(|w| w.to_le_bytes()).collect();
    image.resize(DATA.into(), 0);
    image.extend(HELLO);
    image.resize(usize::from(DATA) + 0x20, 0);
    image.extend(b"test\0");
    image.resize(usize::from(DATA) + 0x30, 0);
    image.extend(request.iter().flat_map(|w: &u32| w.to_le_bytes()));

    let cpus = CpuManager::new();
    let cpu = cpus.get_core(0).unwrap();
    let mut kernel = Kernel::default();
    kernel.ports.insert("test".into(), Rc::new(RefCell::new(Echo)));
    let entry = load_flat_image(&cpus.ram, &image, CODE.base).unwrap();
    kernel.start(cpu, entry, image.len() as u64);
    assert!(kernel.run(cpu), "guest stopped at {:#x} without ExitProcess", cpu.get_pc());

    assert_eq!(kernel.debug_output, [String::from_utf8_lossy(HELLO)]);
    let x = |i| cpu.get_x(i);
    assert_eq!([x(20), x(21), x(22), x(23), x(25), x(27)], [0, 0, HEAP.base, 0, 0, 0]);
    assert_eq!(x(24), 0x1000 + MAIN_STACK.size + 0x40_0000);
    assert_ne!(x(26), 0);
    assert_eq!(x(28), Error::NotImplemented.code().into());
    let reply: Vec<u32> = (0..10).map(|i| cpu.read_u32(TLS + 4 * i)).collect();
    assert_eq!(reply[4..10], [u32::from_le_bytes(*b"SFCO"), 0, 0, 0, 0x1111, 0x2222]);
}

#[test]
fn svcs_follow_switchbrew() {
    let cpus = CpuManager::new();
    let cpu = cpus.get_core(0).unwrap();
    let mut kernel = Kernel::default();
    kernel.ports.insert("test".into(), Rc::new(RefCell::new(Echo)));
    let mut svc = |nr, args: &[u64]| {
        let mut x = [0; 8];
        x[..args.len()].copy_from_slice(args);
        kernel.svc(cpu, nr, x).unwrap()
    };
    // switchbrew SVC: SendSyncRequest 0x21 closes the session on any error [S2];
    // SetHeapSize 0x01 takes 2 MiB multiples up to 4 GiB, 0xCA01 otherwise;
    // MapPhysicalMemory 0x2c gives 0xCC01 unaligned and 0xDC01 outside the alias region.
    for region in [CODE, STACK, HEAP, ALIAS] {
        assert_eq!((region.base | region.size) % 0x20_0000, 0);
    }
    assert_eq!(svc(0x01, &[0, 0x10_0000])[0], 0xCA01);
    assert_eq!(svc(0x01, &[0, 0x1_0020_0000])[0], 0xCA01);
    assert_eq!(svc(0x01, &[0, 0x20_0000])[..2], [0, HEAP.base]);
    assert_eq!(svc(0x2c, &[ALIAS.base + 1, 0x1000])[0], 0xCC01);
    assert_eq!(svc(0x2c, &[HEAP.base, 0x1000])[0], 0xDC01);
    assert_eq!(svc(0x2c, &[ALIAS.base, 0x2000])[0], 0);
    assert_eq!(svc(0x2c, &[ALIAS.base + 0x1000, 0x2000])[0], 0);
    assert_eq!(svc(0x29, &[0, 7, CURRENT_PROCESS.into(), 0])[1], MAIN_STACK.size + 0x20_0000 + 0x3000);

    cpu.write_u64(HEAP.base, 0xC0FFEE);
    assert_eq!(svc(0x04, &[STACK.base, HEAP.base, 0x1000])[0], 0);
    assert_eq!(cpu.read_u64(STACK.base), 0xC0FFEE);
    cpu.write_u64(STACK.base, 0xBEEF);
    assert_eq!(svc(0x05, &[STACK.base, HEAP.base, 0x1000])[0], 0);
    assert_eq!(cpu.read_u64(HEAP.base), 0xBEEF);

    let transfer = svc(0x15, &[0, ALIAS.base, 0x3000, 3]);
    assert_eq!(transfer[0], 0);
    assert_eq!(svc(0x16, &[transfer[1]])[0], 0);
    assert_eq!(svc(0x16, &[transfer[1]])[0], 0xE401);

    cpu.write_mem(HEAP.base, b"test\0").unwrap();
    let session = svc(0x1f, &[0, HEAP.base])[1];
    cpu.write_u32(TLS, 16);
    assert_eq!(svc(0x21, &[session])[0], Error::NotImplemented.code().into());
    assert_eq!(svc(0x21, &[session])[0], 0xE401);
}
