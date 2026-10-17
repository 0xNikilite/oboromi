//! Test frame: a compute shader writes a gradient into a storage buffer that is presented as is.
use super::compute::{ComputePipeline, HostBuffer};
use super::spirv::{BuiltIn, Emitter, decoration as deco, storage_class as sc, *};
use super::{VkError, device};
use ash::vk;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const GROUP: u32 = 16;
const PIXELS_BINDING: u32 = 9;

fn gradient_shader() -> Vec<u32> {
    let mut e = Emitter::new();
    e.emit_header();
    e.emit_capability(capability::SHADER);
    e.emit_memory_model(0, 1);
    let void = e.emit_type_void();
    let bool_ty = e.emit_type_bool();
    let u32_ty = e.emit_type_int(32, 0);
    let uvec3 = e.emit_type_vector(u32_ty, 3);
    let gid_ptr = e.emit_type_pointer(sc::INPUT, uvec3);
    let gid = e.emit_variable(gid_ptr, sc::INPUT);
    let array = e.emit_type_runtime_array(u32_ty);
    let block = e.emit_type_struct(&[array]);
    let block_ptr = e.emit_type_pointer(sc::STORAGE_BUFFER, block);
    let pixels = e.emit_variable(block_ptr, sc::STORAGE_BUFFER);
    let pixel_ptr = e.emit_type_pointer(sc::STORAGE_BUFFER, u32_ty);
    // The u32 is stored little-endian, so r is byte 0 and a is byte 3 of each RGBA8 pixel.
    let [zero, max, width, height, eight, blue_alpha] =
        [0, 255, WIDTH, HEIGHT, 8, 0xff80_0000].map(|v: u32| e.emit_constant_typed(u32_ty, v));
    let fn_ty = e.emit_type_function(void, &[]);
    let main = e.emit_function(void, 0, fn_ty);
    e.emit_label();
    let id = e.emit_load(uvec3, gid);
    let x = e.emit_composite_extract(u32_ty, id, &[0]);
    let y = e.emit_composite_extract(u32_ty, id, &[1]);
    let x_inside = e.emit_uless_than(bool_ty, x, width);
    let y_inside = e.emit_uless_than(bool_ty, y, height);
    let inside = e.emit_logical_and(bool_ty, x_inside, y_inside);
    let (body, merge) = (e.alloc_id(), e.alloc_id());
    e.emit_selection_merge(merge, 0);
    e.emit_branch_conditional(inside, body, merge);
    e.emit_label_id(body);
    let x_scaled = e.emit_imul(u32_ty, x, max);
    let r = e.emit_udiv(u32_ty, x_scaled, width);
    let y_scaled = e.emit_imul(u32_ty, y, max);
    let g = e.emit_udiv(u32_ty, y_scaled, height);
    let g = e.emit_shift_left_logical(u32_ty, g, eight);
    let rg = e.emit_bitwise_or(u32_ty, r, g);
    let pixel = e.emit_bitwise_or(u32_ty, rg, blue_alpha);
    let row = e.emit_imul(u32_ty, y, width);
    let index = e.emit_iadd(u32_ty, row, x);
    let dst = e.emit_access_chain(pixel_ptr, pixels, &[zero, index]);
    e.emit_store(dst, pixel);
    e.emit_branch(merge);
    e.emit_label_id(merge);
    e.emit_return();
    e.emit_function_end();
    e.emit_entry_point(execution_model::GLCOMPUTE, main, "main", &[gid, pixels]);
    e.emit_execution_mode(main, execution_mode::LOCAL_SIZE, &[GROUP, GROUP, 1]);
    // GlobalInvocationId is WorkgroupId * LocalSize + LocalInvocationId, i.e. the pixel coordinate.
    e.emit_decorate(gid, deco::BUILTIN, &[BuiltIn::GlobalInvocationId as u32]);
    e.emit_decorate(array, deco::ARRAY_STRIDE, &[4]);
    e.emit_decorate(block, deco::BLOCK, &[]);
    e.emit_member_decorate(block, 0, deco::OFFSET, &[0]);
    e.emit_decorate(pixels, deco::DESCRIPTOR_SET, &[0]);
    e.emit_decorate(pixels, deco::BINDING, &[PIXELS_BINDING]);
    e.finalize();
    e.words()
}

pub fn render_test_frame() -> Result<(), VkError> {
    let state = device()?;
    let size = u64::from(WIDTH * HEIGHT * 4);
    let buffer = HostBuffer::new(state, size)?;
    let bindings = [(PIXELS_BINDING, vk::DescriptorType::STORAGE_BUFFER)];
    let mut pipeline = ComputePipeline::new(state, &gradient_shader(), &bindings)?;
    pipeline.bind(&[(PIXELS_BINDING, &buffer)]);
    // Round up so 1080 rows get a partial last group; the shader guard drops the overhang.
    let groups = [WIDTH.div_ceil(GROUP), HEIGHT.div_ceil(GROUP), 1];
    state.dispatch(&pipeline, groups)?;
    let mut pixels = vec![0; size as usize];
    buffer.read(&mut pixels)?;
    crate::display::present(WIDTH, HEIGHT, &pixels);
    Ok(())
}
