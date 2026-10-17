use crate::gpu::compute::HostBuffer;
use crate::gpu::graphics::{DrawState, GraphicsPipeline, RasterState, RenderTarget, TargetState, VertexLayout};
use crate::gpu::spirv::{BuiltIn, Emitter, capability, decoration, execution_mode, execution_model, storage_class};
use crate::gpu::{VkError, VkState};
use ash::vk;

pub const SIZE: u32 = 64;
// Pixel corners (4, 4), (60, 4) and (4, 60) in normalized device coordinates, coloured red, green and blue.
const VERTICES: [([f32; 2], [u8; 4]); 3] =
    [([-0.875, -0.875], [255, 0, 0, 255]), ([0.875, -0.875], [0, 255, 0, 255]), ([-0.875, 0.875], [0, 0, 255, 255])];

// Vertex: position (x, y, 0, 1) from location 0, colour from location 1 to 0. Fragment: colour from location 0 to 0.
fn pass_through(stage: u32) -> Emitter {
    use storage_class::{INPUT, OUTPUT};
    let vertex = stage == execution_model::VERTEX;
    let mut e = Emitter::new();
    e.emit_header();
    e.emit_capability(capability::SHADER);
    e.emit_memory_model(0, 1);
    let void = e.emit_type_void();
    let main_type = e.emit_type_function(void, &[]);
    let float = e.emit_type_float(32);
    let vec4 = e.emit_type_vector(float, 4);
    let [vec4_in, vec4_out] = [INPUT, OUTPUT].map(|class| e.emit_type_pointer(class, vec4));
    let [colour_in, colour_out] = [(vec4_in, INPUT), (vec4_out, OUTPUT)].map(|(ty, class)| e.emit_variable(ty, class));
    e.emit_decorate(colour_in, decoration::LOCATION, &[u32::from(vertex)]);
    e.emit_decorate(colour_out, decoration::LOCATION, &[0]);
    let main = e.emit_function(void, 0, main_type);
    e.emit_label();
    let colour = e.emit_load(vec4, colour_in);
    e.emit_store(colour_out, colour);
    let mut interface = vec![colour_in, colour_out];
    if vertex {
        let vec2 = e.emit_type_vector(float, 2);
        let vec2_in = e.emit_type_pointer(INPUT, vec2);
        let position_in = e.emit_variable(vec2_in, INPUT);
        let position_out = e.emit_variable(vec4_out, OUTPUT);
        e.emit_decorate(position_in, decoration::LOCATION, &[0]);
        e.emit_decorate(position_out, decoration::BUILTIN, &[BuiltIn::Position as u32]);
        let [zero, one] = [0.0f32, 1.0].map(|v| e.emit_constant_typed(float, v));
        let xy = e.emit_load(vec2, position_in);
        let position = e.emit_composite_construct(vec4, &[xy, zero, one]);
        e.emit_store(position_out, position);
        interface.extend([position_in, position_out]);
    } else {
        e.emit_execution_mode(main, execution_mode::ORIGIN_UPPER_LEFT, &[]);
    }
    e.emit_return();
    e.emit_function_end();
    e.emit_entry_point(stage, main, "main", &interface);
    e.finalize();
    e
}

/// Draws the triangle over a white clear into a SIZE x SIZE target and reads it back, once per call.
pub fn triangle(state: &VkState) -> Result<impl FnMut(&mut [u8]) -> Result<(), VkError> + '_, VkError> {
    let [vertex, fragment] = [execution_model::VERTEX, execution_model::FRAGMENT].map(|stage| pass_through(stage).words());
    let bindings = [vk::VertexInputBindingDescription { binding: 0, stride: 12, input_rate: vk::VertexInputRate::VERTEX }];
    let attributes = [(0, vk::Format::R32G32_SFLOAT, 0), (1, vk::Format::R8G8B8A8_UNORM, 8)]
        .map(|(location, format, offset)| vk::VertexInputAttributeDescription { location, binding: 0, format, offset });
    let topology = vk::PrimitiveTopology::TRIANGLE_LIST;
    let layout = VertexLayout { bindings: &bindings, attributes: &attributes, topology };
    let blend = vk::PipelineColorBlendAttachmentState::default().color_write_mask(vk::ColorComponentFlags::RGBA);
    let pipeline = GraphicsPipeline::new(state, &vertex, None, &fragment, &[], &layout, &TargetState { blend, depth: false })?;
    let target = RenderTarget::new(state, SIZE, SIZE, false)?;
    let mut vertices = HostBuffer::new(state, 36)?;
    let data: Vec<u8> =
        VERTICES.iter().flat_map(|([x, y], rgba)| [&x.to_le_bytes()[..], &y.to_le_bytes(), rgba].concat()).collect();
    vertices.write(&data)?;
    let size = SIZE as f32;
    let raster = RasterState {
        viewport: vk::Viewport { x: 0.0, y: 0.0, width: size, height: size, min_depth: 0.0, max_depth: 1.0 },
        scissor: target.extent.into(),
        cull_face: vk::CullModeFlags::NONE,
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        topology,
        depth_test: false,
        depth_write: false,
        depth_func: vk::CompareOp::ALWAYS,
        blend_const: [0.0; 4],
    };
    Ok(move |out: &mut [u8]| {
        let draw = DrawState {
            vertex_buffers: &[&vertices],
            index_buffer: None,
            vertex_count: 3,
            first_vertex: 0,
            instance_count: 1,
            clear: Some([1.0; 4]),
            raster,
        };
        state.draw(&pipeline, &target, &draw)?;
        target.read(out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::gpu::harness::{spirv_val, vk_state};

    #[test]
    fn triangle_interpolates_vertex_colours_over_the_clear_colour() {
        for stage in [execution_model::VERTEX, execution_model::FRAGMENT] {
            spirv_val(&pass_through(stage));
        }
        let Some(state) = vk_state() else { return };
        let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
        triangle(&state).unwrap()(&mut pixels).unwrap();
        let pixel = |x: u32, y: u32| &pixels[((y * SIZE + x) * 4) as usize..][..4];
        let barycentric = |x: u32, y: u32| {
            let [u, v] = [x, y].map(|p| (p as f32 + 0.5 - 4.0) / 56.0);
            [1.0 - u - v, u, v, 1.0].map(|c| (c * 255.0).round() as u8)
        };
        for (x, y) in [(4, 4), (58, 4), (4, 58), (20, 20)] {
            let (got, want) = (pixel(x, y), barycentric(x, y));
            assert!(got.iter().zip(want).all(|(&g, w)| g.abs_diff(w) <= 2), "({x}, {y}): {got:?} != {want:?}");
        }
        for (x, y) in (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| (x, y))) {
            if x < 4 || y < 4 || x + y >= 64 {
                assert_eq!(pixel(x, y), [255; 4], "({x}, {y})");
            }
        }
    }
}
