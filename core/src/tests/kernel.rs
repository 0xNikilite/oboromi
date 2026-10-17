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
    const DATA: u16 = 0x300;
    const OUTPUT: &[u8] = b"pointer 0x8000 echo 0x1111 0x2222";
    let mut program = vec![
        mrs_tpidrro(19),
        movz(20, (CODE.base >> 16) as u16, 16),
        add_imm(20, 20, DATA),
        mov_reg(1, 20),
        svc(0x1f),
        mov_reg(21, 1),
    ];
    let copy = |program: &mut Vec<u32>, offset| {
        program.push(add_imm(9, 20, offset));
        for at in [0, 16, 32, 48] {
            program.extend([ldp(2, 3, 9, at), stp(2, 3, 19, at)]);
        }
        program.extend([mov_reg(0, 21), svc(0x21)]);
    };
    copy(&mut program, 0x80);
    program.extend([add_imm(9, 19, 32), ldr(22, 9)]);
    copy(&mut program, 0xc0);
    program.extend([add_imm(9, 19, 32), ldr(23, 9)]);
    copy(&mut program, 0x100);
    program.extend([add_imm(9, 19, 12), ldr(24, 9)]);
    copy(&mut program, 0x140);
    program.extend([add_imm(9, 19, 48), ldr_post(25, 9, 4), ldr(26, 9)]);
    copy(&mut program, 0x180);
    program.extend([
        add_imm(0, 20, 0x20),
        movz(1, OUTPUT.len() as u16, 0),
        svc(0x27),
        svc(0x7f),
        svc(0x07),
    ]);
    let control = |command| [5, 10, 0, 0, u32::from_le_bytes(*b"SFCI"), 0, command, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let domain = [4, 14, 0, 0, 1 | 24 << 16, 1, 0, 0, u32::from_le_bytes(*b"SFCI"), 0, 0, 0, 0x1111, 0x2222, 0, 0];
    let close = [4, 8, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut image: Vec<u8> = program.iter().flat_map(|w| w.to_le_bytes()).collect();
    image.resize(DATA.into(), 0);
    image.extend(b"test\0");
    image.resize(usize::from(DATA) + 0x20, 0);
    image.extend(OUTPUT);
    image.resize(usize::from(DATA) + 0x80, 0);
    for message in [control(3), control(0), control(2), domain, close] {
        image.extend(message.iter().flat_map(|word| word.to_le_bytes()));
    }

    let cpus = CpuManager::new();
    let cpu = cpus.get_core(0).unwrap();
    let mut kernel = Kernel::default();
    kernel.ports.insert("test".into(), Rc::new(RefCell::new(Echo)));
    let entry = load_flat_image(&cpus.ram, &image, CODE.base).unwrap();
    kernel.start(cpu, entry, image.len() as u64);
    assert!(kernel.run(cpu), "guest stopped at {:#x} without ExitProcess", cpu.get_pc());

    assert_eq!(kernel.debug_output, [String::from_utf8_lossy(OUTPUT)]);
    let x = |i| cpu.get_x(i);
    assert_eq!([x(22), x(23), x(25), x(26)], [0x8000, 1, 0x1111, 0x2222]);
    assert_ne!(x(21), 0);
    assert_ne!(x(24), 0);
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
