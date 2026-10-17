#![cfg(test)]

use crate::gpu::{
    fifo::{Event, Pusher},
    threed::{MemoryLayout, PrimitiveTopology},
};

const SET_OBJECT: u32 = 0x0000;
const SET_VERTEX_STREAM_SIZE_A: u32 = 0x0600;
const SET_COLOR_TARGET_A: u32 = 0x0800;
const SET_VIEWPORT_SCALE_X: u32 = 0x0a00;
const SET_VIEWPORT_CLIP_HORIZONTAL: u32 = 0x0c00;
const SET_SCISSOR_ENABLE: u32 = 0x0e00;
const SET_VERTEX_ATTRIBUTE_A: u32 = 0x1160;
const DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST: u32 = 0x1214;
const SET_CT_WRITE: u32 = 0x1a00;
const SET_VERTEX_STREAM_A_FORMAT: u32 = 0x1c00;
const SET_PIPELINE_SHADER: u32 = 0x2000;

fn header(op: u32, method: u32, count: u32) -> u32 {
    op << 29 | count << 16 | method / 4
}

fn inc(method: u32, values: &[u32], words: &mut Vec<u32>) {
    words.push(header(1, method, values.len() as u32));
    words.extend(values);
}

fn non_inc(method: u32, values: &[u32], words: &mut Vec<u32>) {
    words.push(header(3, method, values.len() as u32));
    words.extend(values);
}

fn immediate(method: u32, value: u32, words: &mut Vec<u32>) {
    words.push(header(4, method, value));
}

fn one_inc(method: u32, values: &[u32], words: &mut Vec<u32>) {
    words.push(header(5, method, values.len() as u32));
    words.extend(values);
}

fn entry(address: u64, words: usize) -> [u8; 8] {
    let lo = address as u32;
    let hi = ((address >> 32) as u32 & 0xff) | (words as u32) << 10;
    [lo.to_le_bytes(), hi.to_le_bytes()]
        .concat()
        .try_into()
        .unwrap()
}

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn submit(words: &[u32], split: Option<usize>) -> Vec<Event> {
    let memory = bytes(words);
    let entries = match split {
        Some(at) => vec![
            entry(0x1000, at),
            entry(0x1000 + at as u64 * 4, words.len() - at),
        ],
        None => vec![entry(0x1000, words.len())],
    };
    Pusher::default()
        .submit(
            entries,
            |address, len| {
                let start = (address - 0x1000) as usize;
                Ok(&memory[start..start + len])
            },
            |_, _| Ok(()),
        )
        .unwrap()
}

fn triangle_pushbuffer() -> Vec<u32> {
    let mut words = Vec::new();
    inc(SET_OBJECT, &[0xc797], &mut words);
    inc(SET_VERTEX_STREAM_SIZE_A, &[0, 0x400], &mut words);
    inc(
        SET_VERTEX_STREAM_A_FORMAT,
        &[0x1000 | 24, 1, 0x2345_6780, 0],
        &mut words,
    );
    inc(
        SET_VERTEX_ATTRIBUTE_A,
        &[
            0 | (0 << 7) | (2 << 21) | (7 << 27),
            0 | (12 << 7) | (4 << 21) | (7 << 27) | (1 << 31),
        ],
        &mut words,
    );
    inc(
        SET_COLOR_TARGET_A,
        &[2, 0x3456_7800, 640, 480, 0xd5, 0x1000],
        &mut words,
    );
    inc(
        SET_VIEWPORT_SCALE_X,
        &[
            320.0f32.to_bits(),
            240.0f32.to_bits(),
            0.5f32.to_bits(),
            320.0f32.to_bits(),
            240.0f32.to_bits(),
            0.5f32.to_bits(),
        ],
        &mut words,
    );
    inc(
        SET_VIEWPORT_CLIP_HORIZONTAL,
        &[640 << 16, 480 << 16, 0.0f32.to_bits(), 1.0f32.to_bits()],
        &mut words,
    );
    inc(SET_SCISSOR_ENABLE, &[1, 640 << 16, 480 << 16], &mut words);
    inc(SET_PIPELINE_SHADER, &[1], &mut words);
    inc(SET_PIPELINE_SHADER + 0x14, &[3, 0x1234_0000], &mut words);
    inc(SET_PIPELINE_SHADER + 5 * 64, &[1], &mut words);
    inc(
        SET_PIPELINE_SHADER + 5 * 64 + 0x14,
        &[4, 0x5678_0000],
        &mut words,
    );
    inc(SET_CT_WRITE, &[0x1111], &mut words);
    inc(
        DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST,
        &[(4 << 28) | (3 << 16) | 2],
        &mut words,
    );
    words
}

#[test]
fn decodes_triangle_draw_snapshot() {
    let events = submit(&triangle_pushbuffer(), None);
    let Event::Draw(draw) = &events[0] else {
        panic!()
    };
    assert_eq!(events.len(), 1);
    assert_eq!(
        (
            draw.topology,
            draw.vertex_count,
            draw.first_vertex,
            draw.instance_count
        ),
        (PrimitiveTopology::Triangles, 3, 2, 1)
    );
    assert_eq!(
        (
            draw.vertex_streams[0].address,
            draw.vertex_streams[0].size,
            draw.vertex_streams[0].stride,
            draw.vertex_streams[0].enabled
        ),
        (0x1_2345_6780, 0x400, 24, true)
    );
    assert_eq!(
        (
            draw.vertex_attributes[0].stream,
            draw.vertex_attributes[0].offset,
            draw.vertex_attributes[0].size,
            draw.vertex_attributes[0].kind,
            draw.vertex_attributes[0].bgra
        ),
        (0, 0, 2, 7, false)
    );
    assert_eq!(
        (
            draw.vertex_attributes[1].offset,
            draw.vertex_attributes[1].size,
            draw.vertex_attributes[1].bgra
        ),
        (12, 4, true)
    );
    assert_eq!(
        (
            draw.colour_target.address,
            draw.colour_target.width,
            draw.colour_target.height,
            draw.colour_target.format,
            draw.colour_target.layout
        ),
        (0x2_3456_7800, 640, 480, 0xd5, MemoryLayout::Pitch)
    );
    assert_eq!(
        (draw.viewport.scale, draw.viewport.offset),
        ([320.0, 240.0, 0.5], [320.0, 240.0, 0.5])
    );
    assert_eq!(
        (
            draw.viewport.width,
            draw.viewport.height,
            draw.viewport.min_depth,
            draw.viewport.max_depth
        ),
        (640, 480, 0.0, 1.0)
    );
    assert_eq!(
        (draw.scissor.enabled, draw.scissor.x_max, draw.scissor.y_max),
        (true, 640, 480)
    );
    assert_eq!(
        (
            draw.shader_programs[0].address,
            draw.shader_programs[5].address
        ),
        (0x3_1234_0000, 0x4_5678_0000)
    );
    assert_eq!(draw.colour_write_mask, 0xf);
}

#[test]
fn packet_split_across_entries_matches() {
    let words = triangle_pushbuffer();
    assert_eq!(submit(&words, None), submit(&words, Some(1)));
}

#[test]
fn names_unbound_subchannel_and_unsupported_class() {
    let mut words = Vec::new();
    immediate(0x100, 1, &mut words);
    let memory = bytes(&words);
    let error = Pusher::default()
        .submit(
            [entry(0x1000, words.len())],
            |_, _| Ok(memory.as_slice()),
            |_, _| Ok(()),
        )
        .unwrap_err();
    assert!(error.contains("unbound subchannel 0"), "{error}");

    words.clear();
    inc(SET_OBJECT, &[0xc7c0], &mut words);
    immediate(0x100, 1, &mut words);
    let memory = bytes(&words);
    let error = Pusher::default()
        .submit(
            [entry(0x1000, words.len())],
            |_, _| Ok(memory.as_slice()),
            |_, _| Ok(()),
        )
        .unwrap_err();
    assert!(
        error.contains("unsupported class method: class 0xc7c0, method 0x0100"),
        "{error}"
    );
}

#[test]
fn one_increment_advances_only_first_word() {
    let mut words = Vec::new();
    inc(SET_OBJECT, &[0xc797], &mut words);
    one_inc(SET_VERTEX_ATTRIBUTE_A, &[1, 2, 3], &mut words);
    non_inc(SET_CT_WRITE, &[0x1111], &mut words);
    let memory = bytes(&words);
    let mut pusher = Pusher::default();
    pusher
        .submit(
            [entry(0x1000, words.len())],
            |_, _| Ok(memory.as_slice()),
            |_, _| Ok(()),
        )
        .unwrap();
    assert_eq!(pusher.threed().raw((SET_VERTEX_ATTRIBUTE_A / 4) as u16), 1);
    assert_eq!(
        pusher.threed().raw((SET_VERTEX_ATTRIBUTE_A / 4 + 1) as u16),
        3
    );
}
