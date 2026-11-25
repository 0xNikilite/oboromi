use super::{AddressSpace, Channel, Fault, SubmitError};

fn submit(
    channel: &mut Channel,
    memory: &mut AddressSpace,
    words: &[u32],
) -> Result<(), SubmitError> {
    let bytes = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    memory.map(0x1000, bytes).unwrap();
    let result = channel.submit(memory, &[0x1000 | (words.len() as u64) << 42]);
    memory.unmap(0x1000).unwrap();
    result
}

#[test]
fn registration_checks_both_neighbours_and_address_limit() {
    let mut memory = AddressSpace::default();
    memory.map(0x10, vec![1; 4]).unwrap();
    memory.map(0x0c, vec![2; 4]).unwrap();
    for (base, size) in [
        (0x10, 4),
        (0x13, 1),
        (0xb, 2),
        (0xb, 10),
        (1 << 49, 1),
        (0, 0),
        (u64::MAX, 2),
    ] {
        assert_eq!(memory.map(base, vec![0; size]), Err(Fault::InvalidMapping));
    }
    assert_eq!(memory.unmap(0x10), Some(vec![1; 4]));
    assert!(memory.read(0x10, 1).is_err());
    memory.map(0x10, vec![3]).unwrap();
    assert_eq!(memory.read(0x10, 1).unwrap(), [3]);
}

#[test]
fn packet_modes_and_registers_persist_across_submissions() {
    let mut channel = Channel::default();
    let mut ram = AddressSpace::default();
    ram.map(0x2000, b"abcd....".to_vec()).unwrap();
    // Increment-once writes high, low, low. Non-incrementing rewrites line length.
    submit(
        &mut channel,
        &mut ram,
        &[
            0,
            0x2001_0000,
            0xc7b5,
            0xa003_0100,
            0,
            0x1111,
            0x2000,
            0x2002_0102,
            0,
            0x2004,
            0x6002_0106,
            1,
            4,
        ],
    )
    .unwrap();
    submit(&mut channel, &mut ram, &[0x8181_00c0]).unwrap();
    assert_eq!(ram.read(0x2000, 8).unwrap(), b"abcdabcd");
}

#[test]
fn invalid_final_row_and_semaphore_do_not_commit_a_launch() {
    let mut channel = Channel::default();
    let mut ram = AddressSpace::default();
    ram.map(0x2000, vec![1; 8]).unwrap();
    ram.map(0x3000, vec![2; 7]).unwrap();
    ram.map(0x4000, vec![3; 4]).unwrap();
    submit(
        &mut channel,
        &mut ram,
        &[
            0x2001_0000,
            0xc7b5,
            0x2003_0090,
            0,
            0x4000,
            9,
            0x2008_0100,
            0,
            0x2000,
            0,
            0x3000,
            4,
            4,
            4,
            2,
        ],
    )
    .unwrap();
    let error = submit(&mut channel, &mut ram, &[0x838a_00c0]).unwrap_err();
    assert_eq!(
        error.fault,
        Fault::Unmapped {
            va: 0x3000,
            size: 8
        }
    );
    assert_eq!(ram.read(0x3000, 7).unwrap(), [2; 7]);
    assert_eq!(ram.read(0x4000, 4).unwrap(), [3; 4]);
    ram.unmap(0x4000);
    let error = submit(&mut channel, &mut ram, &[0x818a_00c0]).unwrap_err();
    assert_eq!(
        error.fault,
        Fault::Unmapped {
            va: 0x4000,
            size: 4
        }
    );
    assert_eq!(ram.read(0x3000, 7).unwrap(), [2; 7]);
}

#[test]
fn fault_identifies_the_first_unsupported_word() {
    for (words, pc, fault) in [
        (
            vec![0, 0xe000_0000],
            0x1004,
            Fault::UnsupportedHeader(0xe000_0000),
        ),
        (
            vec![0x2001_0000, 0xb0b5],
            0x1004,
            Fault::UnsupportedClass(0xb0b5),
        ),
        (vec![0x8181_40c0], 0x1000, Fault::UnboundSubchannel(2)),
        (
            vec![0x2002_0000, 0xc7b5],
            0x1008,
            Fault::TruncatedPacket { remaining: 1 },
        ),
        (
            vec![0x2001_0000, 0xc7b5, 0x878a_00c0],
            0x1008,
            Fault::UnsupportedLaunch(0x78a),
        ),
    ] {
        let mut channel = Channel::default();
        let mut ram = AddressSpace::default();
        assert_eq!(
            submit(&mut channel, &mut ram, &words),
            Err(SubmitError {
                entry: 0,
                pushbuffer_va: pc,
                fault
            })
        );
    }
}

#[test]
fn control_entries_and_discontinuous_packets_fail_explicitly() {
    let mut channel = Channel::default();
    let mut ram = AddressSpace::default();
    ram.map(0x1000, 0x2001_0000u32.to_le_bytes().to_vec())
        .unwrap();
    let first = 0x1000 | (1 << 42);
    let truncated = channel.submit(&mut ram, &[first, 0]).unwrap_err();
    assert_eq!(truncated.entry, 0);
    assert_eq!(truncated.pushbuffer_va, 0x1004);
    for flag in [1, 2, 1 << 40, 1 << 41, 1 << 63] {
        let entry = first | flag;
        assert_eq!(
            channel.submit(&mut ram, &[entry]).unwrap_err().fault,
            Fault::UnsupportedEntry(entry)
        );
    }
    let error = channel
        .submit(&mut ram, &[first, 0, first + 8])
        .unwrap_err();
    assert_eq!(
        error,
        SubmitError {
            entry: 2,
            pushbuffer_va: 0x1008,
            fault: Fault::DiscontinuousPacket { expected: 0x1004 }
        }
    );
}

#[test]
fn semaphore_only_and_empty_transfers_need_no_source() {
    let mut ram = AddressSpace::default();
    let mut channel = Channel::default();
    ram.map(0x5000, vec![0; 4]).unwrap();
    submit(
        &mut channel,
        &mut ram,
        &[
            0x2001_0000,
            0xc7b5,
            0x2003_0090,
            0,
            0x5000,
            0x1234,
            0x8008_00c0,
        ],
    )
    .unwrap();
    assert_eq!(ram.read(0x5000, 4).unwrap(), 0x1234u32.to_le_bytes());
    submit(&mut channel, &mut ram, &[0x8000_0092, 0x8189_00c0]).unwrap();
    assert_eq!(ram.read(0x5000, 4).unwrap(), [0; 4]);
}

#[test]
fn overlaps_and_unsupported_pitches_leave_the_image_intact() {
    let mut ram = AddressSpace::default();
    let mut channel = Channel::default();
    ram.map(0x2000, vec![0; 16]).unwrap();
    ram.write(0x2000, b"abcdefgh").unwrap();
    submit(
        &mut channel,
        &mut ram,
        &[
            0x2001_0000,
            0xc7b5,
            0x2008_0100,
            0,
            0x2000,
            0,
            0x2002,
            4,
            4,
            4,
            2,
        ],
    )
    .unwrap();
    assert_eq!(
        submit(&mut channel, &mut ram, &[0x8381_00c0])
            .unwrap_err()
            .fault,
        Fault::OverlappingRows
    );
    assert_eq!(
        submit(&mut channel, &mut ram, &[0x8001_0104, 0x8381_00c0])
            .unwrap_err()
            .fault,
        Fault::UnsupportedPitch
    );
    assert_eq!(ram.read(0x2000, 8).unwrap(), b"abcdefgh");
    submit(&mut channel, &mut ram, &[0x8181_00c0]).unwrap();
    assert_eq!(ram.read(0x2000, 8).unwrap(), b"ababcdgh");
}

#[test]
fn pitched_rectangles_can_share_padding_in_one_mapping() {
    let mut channel = Channel::default();
    let mut ram = AddressSpace::default();
    ram.map(0x2000, b"ab..cd..ef..".to_vec()).unwrap();
    let setup = [
        0x2001_0000,
        0xc7b5,
        0x2008_0100,
        0,
        0x2000,
        0,
        0x2002,
        4,
        4,
        2,
        3,
    ];
    submit(&mut channel, &mut ram, &setup).unwrap();
    submit(&mut channel, &mut ram, &[0x8381_00c0]).unwrap();
    assert_eq!(ram.read(0x2000, 12).unwrap(), b"ababcdcdefef");
}
