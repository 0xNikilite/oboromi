use oboromi_core::cpu::cpu_manager::{BackendError, CORE_COUNT, CpuManager};
use rapid_arm_emu::{address_space::PAGE_SIZE, handler::Exception, state::ExecState};
use std::{
    fs::File,
    io::{self, Write},
};

fn code_page(words: &[u32]) -> io::Result<File> {
    let mut image = tempfile::tempfile()?;
    image.set_len(PAGE_SIZE)?;
    for word in words {
        image.write_all(&word.to_le_bytes())?;
    }
    Ok(image)
}

#[test]
fn console_cpu_has_eight_cores() -> Result<(), BackendError> {
    let cpu = CpuManager::new()?;
    assert_eq!(cpu.core_count().get(), CORE_COUNT);
    Ok(())
}

#[test]
fn rapid_cores_share_guest_ram() -> Result<(), BackendError> {
    let mut cpu = CpuManager::with_cores(std::num::NonZero::new(2).unwrap())?;
    // Writer: STLR W0, [X1]; BRK. Reader: LDAR W0, [X1]; CBZ W0, reader; BRK.
    cpu.map_code(
        0x1000,
        code_page(&[
            0x889ffc20_u32,
            0xd4200000,
            0x88dffc20,
            0x34ffffe0,
            0xd4200000,
        ])?,
    )?;
    cpu.map_ram(0x2000, PAGE_SIZE)?;
    let mut state = ExecState::initial();
    state.pc = 0x1008;
    state.x[1] = 0x2000;
    let reader = cpu.start(1, state.clone())?;
    state.pc = 0x1000;
    state.x[0] = 0xdeadbeef;
    let timeout = std::time::Duration::from_secs(5);
    let write = reader.spawn_thread(0, state)?.wait_timeout(timeout)?;
    assert!(matches!(write.exception, Exception::Breakpoint(0)));
    let read = reader.wait_timeout(timeout)?;
    assert!(matches!(read.exception, Exception::Breakpoint(0)));
    assert_eq!(read.state.x[0], 0xdeadbeef);
    Ok(())
}

#[test]
fn rapid_resumes_the_same_task_while_another_core_runs() -> Result<(), BackendError> {
    use std::time::Duration;
    let mut cpu = CpuManager::with_cores(std::num::NonZero::new(2).unwrap())?;
    // Core 0 spins until the host edits X0. Core 1 starts directly at ADD.
    // CBZ X0, .; ADD X0, X0, #1; BRK #0
    cpu.map_code(
        0x1000,
        code_page(&[0xb4000000_u32, 0x91000400, 0xd4200000])?,
    )?;
    let mut initial = ExecState::initial();
    initial.pc = 0x1000;
    let first = cpu.start(0, initial.clone())?;
    initial.pc += 4;
    initial.x[0] = 99;
    let second = cpu.start(1, initial)?;
    let timeout = Duration::from_secs(5);
    let other = second.wait_timeout(timeout)?;
    assert!(matches!(other.exception, Exception::Breakpoint(0)));
    assert_eq!(other.state.x[0], 100);
    first.with_paused_state(|state| {
        assert_eq!(state.pc, 0x1000);
        state.x[0] = 41;
    })?;
    let resumed = first.wait_timeout(timeout)?;
    assert!(matches!(resumed.exception, Exception::Breakpoint(0)));
    assert_eq!((resumed.state.pc, resumed.state.x[0]), (0x1008, 42));
    Ok(())
}

#[test]
fn rapid_timeout_releases_a_busy_core() -> Result<(), BackendError> {
    use std::{sync::mpsc::RecvTimeoutError, time::Duration};
    let mut cpu = CpuManager::with_cores(std::num::NonZeroUsize::MIN)?;
    // B .; BRK #0
    cpu.map_code(0x1000, code_page(&[0x14000000_u32, 0xd4200000])?)?;
    let mut initial = ExecState::initial();
    initial.pc = 0x1000;
    let error = cpu
        .start(0, initial.clone())?
        .wait_timeout(Duration::from_millis(20))
        .err()
        .expect("loop should time out");
    assert_eq!(
        error.downcast_ref::<RecvTimeoutError>(),
        Some(&RecvTimeoutError::Timeout)
    );
    initial.pc += 4;
    let next = cpu
        .start(0, initial)?
        .wait_timeout(Duration::from_secs(5))?;
    assert!(matches!(next.exception, Exception::Breakpoint(0)));
    Ok(())
}

#[test]
fn rapid_reports_syscall_and_memory_faults() -> Result<(), BackendError> {
    let mut cpu = CpuManager::with_cores(std::num::NonZeroUsize::MIN)?;
    // SVC #7; LDR X0, [X1]
    cpu.map_code(0x1000, code_page(&[0xd400_00e1, 0xf940_0020])?)?;
    cpu.map_ram(0x2000, PAGE_SIZE)?;

    let mut state = ExecState::initial();
    state.pc = 0x1000;
    state.x[1] = 0x3000;
    let timeout = std::time::Duration::from_secs(5);
    let svc = cpu.start(0, state)?.wait_timeout(timeout)?;
    assert!(matches!(svc.exception, Exception::Svc(7)));
    assert_eq!(svc.state.pc, 0x1004);

    let load = cpu.start(0, svc.state)?.wait_timeout(timeout)?;
    assert!(matches!(
        load.exception,
        Exception::DataAbort { vaddr: 0x3000 }
    ));
    assert_eq!(load.state.pc, 0x1004);

    let mut state = load.state;
    state.pc = 0x2000;
    let fetch = cpu.start(0, state)?.wait_timeout(timeout)?;
    assert!(matches!(
        fetch.exception,
        Exception::InstructionAbort { vaddr: 0x2000 }
    ));
    assert_eq!(fetch.state.pc, 0x2000);
    Ok(())
}
