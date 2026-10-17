use crate::cpu::UnicornCPU;
use crate::cpu::cpu_manager::CpuManager;
use crate::kernel::{CODE, Error, Kernel, Object, TLS, Timer};
use std::time::Instant;

const DATA: u64 = 0x4000_0000;
const IMAGE: &[u8] = include_bytes!("../../tests/fixtures/threads.bin");

fn setup(entry: u64) -> (CpuManager, Kernel) {
    let cpus = CpuManager::new();
    let cpu = cpus.get_core(0).unwrap();
    cpu.write_mem(CODE.base, IMAGE).unwrap();
    let mut kernel = Kernel::default().with_virtual_time(true);
    kernel.start(cpu, CODE.base + entry, IMAGE.len() as u64);
    (cpus, kernel)
}

fn call(kernel: &mut Kernel, cpu: &UnicornCPU, nr: u32, args: &[u64]) -> [u64; 8] {
    let mut x = [0; 8];
    x[..args.len()].copy_from_slice(args);
    let result = kernel.svc(cpu, nr, x).unwrap();
    assert_eq!(result[0], 0, "svc {nr:x}");
    result
}

fn thread(kernel: &mut Kernel, cpu: &UnicornCPU, entry: u64, arg: u64, priority: u64) -> u64 {
    call(kernel, cpu, 0x08, &[0, CODE.base + entry, arg, 0x2020_0000, priority, 0])[1]
}

fn ping_pong(seed: u64, virtual_time: bool) -> (Vec<(usize, u32, u64)>, u64) {
    let (cpus, mut kernel) = setup(0);
    let cpu = cpus.get_core(0).unwrap();
    cpu.set_virtual_time(virtual_time);
    kernel.scheduler.seed = seed;
    let other = thread(&mut kernel, cpu, 0, 1, 44);
    call(&mut kernel, cpu, 0x09, &[other]);
    let start = Instant::now();
    assert!(
        kernel.run(cpu),
        "pc={:x} counter={} turn={} mutex={} trace={:?}",
        cpu.get_pc(),
        cpu.read_u32(DATA + 12),
        cpu.read_u32(DATA + 8),
        cpu.read_u32(DATA),
        &kernel.scheduler.trace[kernel.scheduler.trace.len().saturating_sub(12)..]
    );
    let elapsed = start.elapsed().as_secs_f64();
    assert_eq!(cpu.read_u32(DATA + 12), 20_000);
    if !virtual_time {
        eprintln!(
            "fixture: {:.3} us/dispatch, {:.0} switches/s, {} switches",
            elapsed * 1e6 / kernel.scheduler.switches as f64,
            kernel.scheduler.switches as f64 / elapsed,
            kernel.scheduler.switches
        );
    }
    (kernel.scheduler.trace, kernel.scheduler.switches)
}

#[test]
fn mutex_condvar_ping_pong_is_exact_after_ten_thousand_rounds() {
    ping_pong(0, false);
    let (cpus, _) = setup(0x1200);
    let cpu = cpus.get_core(0).unwrap();
    cpu.set_virtual_time(false);
    let mut context = cpu.context().unwrap();
    let mut measurements = Vec::new();
    for mode in 0..3 {
        cpu.benchmark_block_hook(mode != 0, CODE.base, CODE.base + IMAGE.len() as u64);
        cpu.set_pc(CODE.base + 0x1200);
        cpu.set_x(9, 2_000_000);
        let start = Instant::now();
        let mut slices = 0;
        if mode == 0 {
            cpu.run();
        } else {
            loop {
                let end = if mode == 1 { u64::MAX } else { cpu.ticks() + 7200 };
                let preempted = cpu.run_slice(end, 0).unwrap();
                slices += 1;
                if !preempted {
                    break;
                }
                cpu.save(&mut context).unwrap();
                cpu.restore(&context).unwrap();
            }
        }
        measurements.push((start.elapsed().as_secs_f64(), slices));
        assert_eq!(cpu.svc_pending(), Some(0x07));
        assert_eq!(cpu.get_x(9), 0);
    }
    let (total, slices) = measurements[2];
    eprintln!(
        "block hook: {:.3} us/slice overhead, {:.3} us/slice total, {:.0} switches/s; baseline {:.6}s, hooked {:.6}s, sliced {:.6}s, {} slices",
        (measurements[1].0 - measurements[0].0) * 1e6 / f64::from(slices),
        total * 1e6 / f64::from(slices),
        f64::from(slices) / total,
        measurements[0].0,
        measurements[1].0,
        total,
        slices
    );
}

#[test]
fn exclusive_store_fails_after_a_switch_even_when_memory_returns_to_its_old_value() {
    let (cpus, mut kernel) = setup(0x400);
    let cpu = cpus.get_core(0).unwrap();
    let other = thread(&mut kernel, cpu, 0x480, 0, 44);
    call(&mut kernel, cpu, 0x09, &[other]);
    assert!(kernel.run(cpu));
    assert_eq!(cpu.read_u32(DATA), 5);
    assert_eq!(cpu.read_u32(DATA + 4), 1);
    assert_eq!(cpu.read_u32(DATA + 12), 1);
}

#[test]
fn synchronization_deadline_has_one_winner_and_preserves_later_signals() {
    for (when, expected) in [(192, 0), (384, 0), (576, Error::TimedOut.code())] {
        let (cpus, mut kernel) = setup(0x600);
        let cpu = cpus.get_core(0).unwrap();
        let handles = call(&mut kernel, cpu, 0x45, &[]);
        cpu.write_u32(DATA, handles[2] as u32);
        let Some(Object::Event(event)) = kernel.process.handles.get(handles[2] as u32) else { panic!() };
        kernel.scheduler.timers.push(Timer { event: event.clone(), deadline: when, period: 0 });
        assert!(kernel.run(cpu));
        assert_eq!(cpu.read_u32(DATA + 8), expected);
        if expected == 0 {
            assert_eq!(cpu.read_u32(DATA + 12), 0);
        }
        assert_eq!(cpu.read_u32(DATA + 16), 0);
        assert_eq!(cpu.read_u32(DATA + 20), Error::TimedOut.code());
    }
}

#[test]
fn mutex_waiter_boosts_holder_past_medium_priority_and_restores_priority() {
    let (cpus, mut kernel) = setup(0x800);
    let cpu = cpus.get_core(0).unwrap();
    call(&mut kernel, cpu, 0x0d, &[1, 50]);
    let high = thread(&mut kernel, cpu, 0x900, 0, 10);
    let medium = thread(&mut kernel, cpu, 0xa00, 0, 30);
    cpu.write_u32(DATA + 4, high as u32);
    cpu.write_u32(DATA + 8, medium as u32);
    assert!(kernel.run(cpu));
    assert_eq!(cpu.read_u32(DATA + 12), 10);
    assert_eq!(cpu.read_u32(DATA + 20), 50);
    assert_eq!(cpu.read_u32(DATA + 24), 1);
}

#[test]
fn seeded_block_preemption_replays_the_same_guest_svc_trace() {
    let first = ping_pong(0x1234_5678, true);
    let second = ping_pong(0x1234_5678, true);
    assert_eq!(first, second);
}

#[test]
fn context_switch_preserves_general_simd_flags_tls_and_system_registers() {
    let (cpus, mut kernel) = setup(0xc00);
    let cpu = cpus.get_core(0).unwrap();
    let other = thread(&mut kernel, cpu, 0x1000, 0, 44);
    call(&mut kernel, cpu, 0x09, &[other]);
    call(&mut kernel, cpu, 0x0f, &[1, u64::MAX, 0xff]);
    assert!(kernel.run(cpu));
    let output = DATA + 0x2000;
    for n in 0..31 {
        assert_eq!(cpu.read_u64(output + n * 8), 0x100 + n, "x{n}");
    }
    assert_eq!(cpu.read_u64(output + 248), output);
    for n in 0..32 {
        let mut bytes = [0; 16];
        cpu.read_mem(output + 256 + n * 16, &mut bytes).unwrap();
        assert_eq!(bytes, [n as u8; 16], "v{n}");
    }
    for (offset, expected) in [
        (768, 0xa000_0000),
        (776, 0x0100_0000),
        (784, 0x0800_0000),
        (792, 0x1234),
        (800, TLS),
        (808, 19_200_000),
        (848, 0x3c0),
        (856, 4),
        (864, 1),
    ] {
        assert_eq!(cpu.read_u64(output + offset), expected, "offset {offset}");
    }
    assert!(cpu.read_u64(output + 816) > 0);
    assert!(cpu.read_u64(DATA + 0x2348) >= cpu.read_u64(output + 816));
    assert_eq!(cpu.read_u64(DATA + 0x1010), TLS + 0x200);
    let core = cpu.read_u64(DATA + 0x2340);
    assert!(core < 8);
    assert_eq!(cpu.read_u64(output + 824) & 0xff, core);
    assert!(kernel.scheduler.switches >= 3);
}
