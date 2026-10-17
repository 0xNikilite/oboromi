use crate::audio::audout::AudioOut;

#[test]
fn appended_buffers_release_once_their_frames_play() {
    let frames = |count: usize, value: i16| -> Vec<u8> {
        (0..count * 2).flat_map(|_| value.to_le_bytes()).collect()
    };
    let mut out = AudioOut::open(0, 0).unwrap();
    out.append(&frames(100, 1000), 7).unwrap();
    out.append(&frames(200, -1000), 9).unwrap();
    out.start();
    let first = out.step().to_vec();
    assert_eq!((first[199], first[200]), (1000, -1000));
    assert_eq!(out.released(), [7]);
    let second = out.step().to_vec();
    assert_eq!((second[119], second[120]), (-1000, 0));
    assert_eq!(out.released(), [9]);
}
