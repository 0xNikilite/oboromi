//! Wire-format fixture shared by the Qt GPU diagnostics and `cargo test`.
use super::gpu_test::TestResult;
use crate::gpu::State;
use std::time::Instant;

fn pitched_copy() -> Result<(), String> {
    let mut gpu = State::new();
    let commands_va = 0x01_0000_1000;
    let source_va = 0x22_0000_2000;
    let dest_va = 0x33_0000_3000;
    let signal_va = 0x44_0000_4000;
    // Subchannel 4, incrementing SET_OBJECT, then three semaphore registers,
    // eight copy registers and an immediate pitched, multiline LAUNCH_DMA.
    let commands: [u32; 16] = [
        0x2001_8000,
        0xc7b5,
        0x2003_8090,
        0x44,
        0x4000,
        0xcafe_f00d,
        0x2008_8100,
        0x22,
        0x2000,
        0x33,
        0x3001,
        4,
        5,
        3,
        3,
        0x838a_80c0,
    ];
    for (va, bytes) in [
        (
            commands_va,
            commands.iter().flat_map(|w| w.to_le_bytes()).collect(),
        ),
        (source_va, vec![1, 2, 3, 0xff, 4, 5, 6, 0xff, 7, 8, 9]),
        (dest_va, vec![0xcc; 16]),
        (signal_va, vec![0; 4]),
    ] {
        gpu.address_space
            .map(va, bytes)
            .map_err(|e| format!("{e:?}"))?;
    }
    // Split in the middle of the copy register packet.
    let entries = [commands_va | (10 << 42), (commands_va + 40) | (6 << 42)];
    gpu.submit_gpfifo(&entries).map_err(|e| format!("{e:?}"))?;
    let expected = [
        0xcc, 1, 2, 3, 0xcc, 0xcc, 4, 5, 6, 0xcc, 0xcc, 7, 8, 9, 0xcc, 0xcc,
    ];
    let read = |va, size| {
        gpu.address_space
            .read(va, size)
            .map_err(|e| format!("{e:?}"))
    };
    let output = read(dest_va, expected.len())?;
    if output != expected {
        return Err(format!("DMA readback {output:02x?}"));
    }
    if read(signal_va, 4)? != 0xcafe_f00du32.to_le_bytes() {
        return Err("completion payload mismatch".into());
    }
    Ok(())
}

pub(super) fn diagnostic() -> TestResult {
    let start = Instant::now();
    let result = pitched_copy();
    TestResult {
        name: "Ampere GPFIFO pitched DMA + completion".into(),
        passed: result.is_ok(),
        message: result
            .err()
            .map_or_else(|| "PASS".into(), |e| format!("FAIL: {e}")),
        duration: start.elapsed(),
    }
}
