#![cfg(test)]

use super::engine::bytes;
use scene::host::SET_OBJECT;
use scene::threed::*;
use scene::{entry, immediate, inc, non_inc, one_inc};
use crate::gpu::{
    fifo::{Event, Pusher},
    threed::{MemoryLayout, PrimitiveTopology},
};

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
        )
        .unwrap_err();
    assert!(error.contains("unbound subchannel 0"), "{error}");

    words.clear();
    inc(SET_OBJECT, &[0xdead], &mut words);
    immediate(0x100, 1, &mut words);
    let memory = bytes(&words);
    let error = Pusher::default()
        .submit(
            [entry(0x1000, words.len())],
            |_, _| Ok(memory.as_slice()),
        )
        .unwrap_err();
    assert!(
        error.contains("unsupported class method: class 0xdead, method 0x0100"),
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
        )
        .unwrap();
    assert_eq!(pusher.threed().raw((SET_VERTEX_ATTRIBUTE_A / 4) as u16), 1);
    assert_eq!(
        pusher.threed().raw((SET_VERTEX_ATTRIBUTE_A / 4 + 1) as u16),
        3
    );
}

#[test]
fn subsequent_instances_of_one_draw_merge_into_an_instanced_draw() {
    let mut words = triangle_pushbuffer();
    non_inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT, &[(4 << 28) | (3 << 16) | 2; 2], &mut words);
    let events = submit(&words, None);
    let [Event::Draw(draw)] = events.as_slice() else { panic!("{events:?}") };
    assert_eq!((draw.first_instance, draw.instance_count), (0, 3));
}
