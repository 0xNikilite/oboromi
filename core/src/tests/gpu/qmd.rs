#![cfg(test)]

use crate::gpu::{
    compute_class::{ConstantBuffer, ReleaseSemaphore},
    fifo::{Event, Pusher},
};

const SET_OBJECT: u32 = 0x0000;
const SEND_PCAS_A: u32 = 0x02b4;
const SEND_SIGNALING_PCAS_B: u32 = 0x02bc;

fn set(words: &mut [u32; 64], bit: usize, width: usize, value: u32) {
    let mask = if width == 32 {
        u32::MAX
    } else {
        (1 << width) - 1
    };
    words[bit / 32] |= (value & mask) << (bit % 32);
}

fn qmd() -> [u32; 64] {
    let mut words = [0; 64];
    set(&mut words, 576, 4, 3);
    set(&mut words, 580, 4, 2);
    set(&mut words, 384, 32, 11);
    set(&mut words, 416, 16, 12);
    set(&mut words, 448, 16, 13);
    set(&mut words, 592, 16, 32);
    set(&mut words, 608, 16, 4);
    set(&mut words, 624, 16, 2);
    set(&mut words, 544, 18, 0x1800);
    set(&mut words, 640, 8, (1 << 1) | (1 << 6));
    for (slot, address, size) in [(1, 0x12_3456_7800u64, 0x120), (6, 0x65_4321_0000, 0x340)] {
        let base = 1024 + slot * 64;
        set(&mut words, base, 32, address as u32);
        set(&mut words, base + 32, 17, (address >> 32) as u32);
        set(&mut words, base + 51, 13, size >> 4);
    }
    set(&mut words, 138, 1, 1);
    set(&mut words, 736, 32, 0x2345_6780);
    set(&mut words, 768, 8, 3);
    set(&mut words, 800, 32, 0xfeed_beef);
    set(&mut words, 955, 5, 7);
    set(&mut words, 984, 8, 48);
    set(&mut words, 1016, 8, 0x86);
    set(&mut words, 1536, 32, 0x89ab_c000);
    set(&mut words, 1568, 17, 0x123);
    words
}

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn submit(qmd: [u32; 64]) -> Result<Vec<Event>, String> {
    let pushbuffer = bytes(&[
        1 << 29 | 1 << 16 | SET_OBJECT / 4,
        0xc7c0,
        1 << 29 | 1 << 16 | SEND_PCAS_A / 4,
        0x20,
        1 << 29 | 1 << 16 | SEND_SIGNALING_PCAS_B / 4,
        2,
    ]);
    let qmd = bytes(&qmd);
    Pusher::default().submit(
        [entry(0x1000, pushbuffer.len() / 4)],
        |address, len| match address {
            0x1000 => Ok(&pushbuffer[..len]),
            0x2000 => Ok(&qmd[..len]),
            _ => Err(format!("unknown address {address:#x}")),
        },
        |_, _| Ok(()),
    )
}

fn entry(address: u64, words: usize) -> [u8; 8] {
    let hi = ((address >> 32) as u32 & 0xff) | (words as u32) << 10;
    [address as u32, hi]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

#[test]
fn dispatches_qmd_v02_03() {
    let events = submit(qmd()).unwrap();
    let [Event::Dispatch(dispatch)] = events.as_slice() else {
        panic!()
    };
    assert_eq!(dispatch.grid, [11, 12, 13]);
    assert_eq!(dispatch.block, [32, 4, 2]);
    assert_eq!(dispatch.program_address, 0x123_89ab_c000);
    assert_eq!(dispatch.shared_memory_size, 0x1800);
    assert_eq!(
        dispatch.constant_buffers,
        [
            None,
            Some(ConstantBuffer {
                address: 0x12_3456_7800,
                size: 0x120
            }),
            None,
            None,
            None,
            None,
            Some(ConstantBuffer {
                address: 0x65_4321_0000,
                size: 0x340
            }),
            None,
        ]
    );
    assert_eq!(
        dispatch.release_semaphores[0],
        Some(ReleaseSemaphore {
            address: 0x3_2345_6780,
            payload: 0xfeed_beef
        })
    );
    assert_eq!(
        (
            dispatch.barrier_count,
            dispatch.register_count,
            dispatch.sass_version
        ),
        (7, 48, 0x86)
    );
}

#[test]
fn names_unsupported_qmd_version() {
    let mut qmd = qmd();
    qmd[18] = qmd[18] & !0xff | 0x24;
    let error = submit(qmd).unwrap_err();
    assert!(
        error.contains("unsupported C7C0 QMD version QMDV02_04"),
        "{error}"
    );
}
