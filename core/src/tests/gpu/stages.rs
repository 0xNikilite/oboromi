#[cfg(test)]
mod tests {
    use crate::gpu::compute::{HostBuffer, HostImage};
    use crate::gpu::graphics::{DrawState, GraphicsPipeline, RasterState, RenderTarget, TargetState, VertexLayout};
    use crate::gpu::sm86::{CBUF_BINDING, GLOBAL_BINDING, ResourceKind, SUPPORT_BINDING, Stage, TEXTURE_BINDING, emit_module};
    use crate::gpu::spirv::Emitter;
    use crate::gpu::texture::TexturePool;
    use crate::gpu::VkState;
    use crate::gpu::spirv::BuiltIn as B;
    use crate::gpu::threed::PrimitiveTopology;
    use crate::tests::gpu::graphics::{SIZE, opaque};
    use crate::tests::gpu::harness::{GLOBAL_BASE, spirv_val, vk_state};
    use crate::tests::gpu_test::inst::*;
    use ash::vk::{self, DescriptorType};

    const SCALE: u32 = 8;

    fn vertex_program() -> Vec<u128> {
        vec![ald(0, 0x80, 4), ald(4, 0x90, 4), mov_cbuf(SCALE, 0, 0), fmul(0, 0, SCALE), fmul(1, 1, SCALE), ast(0x70, 0, 4), ast(0x80, 4, 4), exit()]
    }

    fn fragment_program() -> Vec<u128> {
        let varying = (0..4).map(|c| ipa(c, 0x80 + 4 * c, 0));
        varying.chain([mov_cbuf(SCALE, 0, 0), fmul(0, 0, SCALE), fmul(1, 1, SCALE), fmul(2, 2, SCALE), exit()]).collect()
    }

    fn stage_interface(stage: Stage, program: &[u128]) -> Vec<ResourceKind> {
        let mut interface = Vec::new();
        let module = emit_module(None, stage, |decoder| {
            decoder.translate_program(program);
            interface = decoder.resources.iter().map(|r| r.kind).filter(|kind| !matches!(kind, ResourceKind::Cbuf(_))).collect();
        });
        spirv_val(&module);
        interface
    }

    #[test]
    fn vertex_program_reads_inputs_and_writes_position_and_varying() {
        use ResourceKind::*;
        let position = BuiltIn(crate::gpu::spirv::BuiltIn::Position as u32);
        let expected = [VertexInput { location: 0 }, VertexInput { location: 1 }, position, Varying { location: 0, flat: false }];
        assert_eq!(stage_interface(Stage::Vertex { uint: 0, sint: 0 }, &vertex_program()), expected);
    }

    #[test]
    fn fragment_program_reads_varying_and_writes_colour() {
        let frag_coord = ResourceKind::BuiltIn(crate::gpu::spirv::BuiltIn::FragCoord as u32);
        let expected = [ResourceKind::Varying { location: 0, flat: false }, frag_coord, ResourceKind::FragmentOutput { location: 0 }];
        assert_eq!(stage_interface(Stage::Fragment { flat: 0, screen_linear: 0, depth: false }, &fragment_program()), expected);
    }

    macro_rules! driver_shaders {
        ($($name:literal),*) => { [$(($name, include_bytes!(concat!("../../../tests/fixtures/driver-shaders/", $name, ".bin")).as_slice())),*] };
    }

    const DRIVER_SHADERS: [(&str, &[u8]); 16] = driver_shaders!(
        "triangle.vert", "triangle.frag", "particles.vert", "particles.frag", "particles_update.comp", "pbr.vert", "pbr.frag",
        "post.vert", "post.frag", "sprite2d.vert", "sprite2d.frag", "quad.tesc", "quad.tese", "dynquad.tesc", "msaa.frag",
        "points.geom"
    );

    // The code of a graphics stage follows its 0x80-byte program header, whose byte 11 is THREADS_PER_INPUT_PRIMITIVE
    // (clc797sph.h:58), byte 15 OUTPUT_TOPOLOGY (:60) and byte 16 MAX_OUTPUT_VERTEX_COUNT (:64); compute has none.
    fn driver_module(device: Option<&VkState>, name: &str) -> (Emitter, Vec<ResourceKind>) {
        let bytes = DRIVER_SHADERS.iter().find(|shader| shader.0 == name).unwrap().1;
        let (stage, code) = match &name[name.len() - 4..] {
            "vert" => (Stage::Vertex { uint: 0, sint: 0 }, &bytes[0x80..]),
            "frag" => (Stage::Fragment { flat: 0, screen_linear: 0, depth: false }, &bytes[0x80..]),
            "tesc" => (Stage::TessControl { vertices: bytes[11].into() }, &bytes[0x80..]),
            "tese" => (Stage::TessEval { mode: 2, spacing: 0, order: 3 }, &bytes[0x80..]),
            "geom" => (Stage::Geometry { input: PrimitiveTopology::Points, output: bytes[15].into(), max_vertices: bytes[16].into(), invocations: bytes[11].into() }, &bytes[0x80..]),
            _ => (Stage::Compute { local_size: [256, 1, 1] }, bytes),
        };
        let program: Vec<u128> = code.chunks(16).map(|word| u128::from_le_bytes(word.try_into().unwrap())).collect();
        let mut resources = Vec::new();
        let module = emit_module(device, stage, |decoder| {
            decoder.translate_program(&program);
            resources = decoder.resources.iter().map(|r| r.kind).collect();
        });
        spirv_val(&module);
        (module, resources)
    }

    #[test]
    fn driver_compiled_shaders_translate() {
        for (name, _) in DRIVER_SHADERS {
            eprintln!("{name}: {:?}", driver_module(None, name).1);
        }
    }

    // Draws `count` vertices over a white clear into a SIZE x SIZE target and reads it back.
    fn draw(state: &VkState, shaders: [&str; 2], layout: &VertexLayout, vertices: &[u8], count: u32, buffers: &[(u32, &HostBuffer)], images: &[(u32, &HostImage)]) -> Vec<u8> {
        let [vertex, fragment] = shaders.map(|name| driver_module(Some(state), name).0.words());
        let uniform = |binding| if binding == GLOBAL_BINDING { DescriptorType::STORAGE_BUFFER } else { DescriptorType::UNIFORM_BUFFER };
        let vertex_bindings: Vec<_> = buffers.iter().map(|&(b, _)| (b, uniform(b))).collect();
        let fragment_bindings: Vec<_> = vertex_bindings.iter().copied().chain(images.iter().map(|&(b, _)| (b, DescriptorType::COMBINED_IMAGE_SAMPLER))).collect();
        let blend = vk::PipelineColorBlendAttachmentState::default().color_write_mask(vk::ColorComponentFlags::RGBA);
        let mut pipeline = GraphicsPipeline::new(state, &vertex, None, &fragment, [&vertex_bindings, &fragment_bindings], layout, &TargetState { blend, depth: None }).unwrap();
        for descriptors in &mut pipeline.descriptors {
            descriptors.bind(buffers);
        }
        pipeline.descriptors[1].bind_images(images);
        let target = RenderTarget::new(state, SIZE, SIZE, None).unwrap();
        let mut buffer = HostBuffer::new(state, vertices.len() as u64).unwrap();
        buffer.write(vertices).unwrap();
        let raster = RasterState {
            viewport: vk::Viewport { x: 0.0, y: 0.0, width: SIZE as f32, height: SIZE as f32, min_depth: 0.0, max_depth: 1.0 },
            scissor: target.extent.into(),
            cull_face: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            topology: layout.topology,
            depth_test: false,
            depth_write: false,
            depth_func: vk::CompareOp::ALWAYS,
            blend_const: [0.0; 4],
            blend,
        };
        let draw = DrawState {
            vertex_buffers: &[(&buffer, layout.bindings[0].stride)],
            index_buffer: None,
            vertex_count: count,
            first_vertex: 0,
            first_instance: 0,
            instance_count: 1,
            clear: Some([1.0; 4]),
            clear_depth: None,
            load: false,
            raster,
        };
        state.draw(&pipeline, &target, &draw).unwrap();
        let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
        target.read(&mut pixels).unwrap();
        pixels
    }

    fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
        pixels[((y * SIZE + x) * 4) as usize..][..4].try_into().unwrap()
    }

    fn bytes(floats: &[f32]) -> Vec<u8> {
        floats.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    #[test]
    fn driver_compiled_triangle_draws_its_varying() {
        let Some(state) = vk_state() else { return };
        // The VS writes position.xy and colour position.zwxy * 0.5 + 0.25, so z and w make red and green barycentric.
        let vertices = bytes(&[-0.875, -0.875, 1.5, -0.5, 0.875, -0.875, -0.5, 1.5, -0.875, 0.875, -0.5, -0.5]);
        let bindings = [vk::VertexInputBindingDescription { binding: 0, stride: 16, input_rate: vk::VertexInputRate::VERTEX }];
        let attributes = [vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32B32A32_SFLOAT, offset: 0 }];
        let layout = VertexLayout { bindings: &bindings, attributes: &attributes, topology: vk::PrimitiveTopology::TRIANGLE_LIST };
        let pixels = draw(&state, ["triangle.vert", "triangle.frag"], &layout, &vertices, 3, &[], &[]);
        for (x, y) in [(4, 4), (58, 4), (4, 58), (20, 20)] {
            let [u, v] = [x, y].map(|p| (p as f32 + 0.5 - 4.0) / 56.0);
            let [ndc_x, ndc_y] = [x, y].map(|p| (p as f32 + 0.5) / 32.0 - 1.0);
            let want = [1.0 - u - v, u, ndc_x * 0.5 + 0.25, ndc_y * 0.5 + 0.25].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
            let got = pixel(&pixels, x, y);
            assert!(got.iter().zip(want).all(|(&g, w)| g.abs_diff(w) <= 2), "({x}, {y}): {got:?} != {want:?}");
        }
        for (x, y) in (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| (x, y))) {
            if x < 4 || y < 4 || x + y >= 64 {
                assert_eq!(pixel(&pixels, x, y), [255; 4], "({x}, {y})");
            }
        }
    }

    #[test]
    fn driver_compiled_sprite_samples_its_texture() {
        let Some(state) = vk_state() else { return };
        // Both stages read GLSL binding 1 through the bindless cbuf handle at c[0x1d][0x8], after the texture handle at
        // c[0x1d][0x0]; bank 0x1d is the desktop driver's pointer table and NVN's may differ. As the VS camera, 1.0 maps
        // pixel p to NDC p - 1, and as the FS tint it keeps the texel.
        let handle = [0, 0, (GLOBAL_BASE >> 4) as u32, (GLOBAL_BASE >> 36) as u32 | 16 >> 4 << 13];
        let [pointers, ubo, support] = [bytes(&handle.map(f32::from_bits)), bytes(&[1.0; 4]), GLOBAL_BASE.to_le_bytes().to_vec()].map(|data| {
            let mut buffer = HostBuffer::new(&state, data.len() as u64).unwrap();
            buffer.write(&data).unwrap();
            buffer
        });
        // One instance: centre (1, 1), half size 0.5, the whole texture, no parallax or rotation, alpha 1.
        let instance = bytes(&[1.0, 1.0, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let bindings = [vk::VertexInputBindingDescription { binding: 0, stride: 48, input_rate: vk::VertexInputRate::INSTANCE }];
        let attributes = [0, 1, 2].map(|location| vk::VertexInputAttributeDescription { location, binding: 0, format: vk::Format::R32G32B32A32_SFLOAT, offset: 16 * location });
        let layout = VertexLayout { bindings: &bindings, attributes: &attributes, topology: vk::PrimitiveTopology::TRIANGLE_STRIP };
        const RGBA8_UNORM: u32 = 8 | 0b010_010_010_010 << 7;
        let tic = [RGBA8_UNORM | 0o5432 << 19, 0, 2 << 21, 1, 1 | 1 << 23, 1 | 1 << 31, 0, 0];
        let tsc = [2 | 2 << 3, 1 | 1 << 4, 0, 0, 0, 0, 0, 0];
        let [tic, tsc] = [tic, tsc].map(|entry| entry.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>());
        let pool = TexturePool { tic: &tic, tsc: &tsc };
        let texels = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 0, 255]];
        let rows: Vec<u8> = texels.chunks(2).flat_map(|row| [row.concat(), vec![0; 24]].concat()).collect();
        let image = HostImage::new(&state, &pool.texture(0).unwrap(), &rows, &pool.sampler(0).unwrap()).unwrap();
        let buffers = [(SUPPORT_BINDING, &support), (CBUF_BINDING + 0x1d, &pointers), (GLOBAL_BINDING, &ubo)];
        let pixels = draw(&state, ["sprite2d.vert", "sprite2d.frag"], &layout, &instance, 4, &buffers, &[(TEXTURE_BINDING, &image)]);
        for ((x, y), texel) in [(20, 20), (44, 20), (20, 44), (44, 44)].into_iter().zip(texels) {
            assert_eq!(pixel(&pixels, x, y), texel, "({x}, {y})");
        }
        for (x, y) in [(4, 4), (15, 32), (48, 32), (32, 15), (32, 48)] {
            assert_eq!(pixel(&pixels, x, y), [255; 4], "({x}, {y})");
        }
    }

    #[test]
    fn geometry_program_reads_its_point_and_emits_a_strip() {
        use ResourceKind::*;
        let program = [isberd(1, 255, 0), ald_vertex(4, 0x70, 4, 1), ald_vertex(8, 0x80, 4, 1), ald(12, 0x60, 1), ast(0x64, 12, 1)];
        let strip = [ast(0x80, 8, 4), ast(0x70, 4, 4), out(2, 2, OUT_EMIT), fadd_imm(4, 4, 0.5), ast(0x70, 4, 4), out(2, 2, OUT_EMIT), out(2, 2, OUT_CUT), exit()];
        let stage = Stage::Geometry { input: PrimitiveTopology::Points, output: 7, max_vertices: 2, invocations: 1 };
        let builtin = |b: B| BuiltIn(b as u32);
        let expected = [PerVertex { attr: 0x70, output: false }, PerVertex { attr: 0x80, output: false }, builtin(B::PrimitiveId), builtin(B::Layer), Varying { location: 0, flat: false }, builtin(B::Position)];
        assert_eq!(stage_interface(stage, &[&program[..], &strip].concat()), expected);
    }

    #[test]
    fn tess_control_program_copies_its_vertex_and_writes_the_patch() {
        use ResourceKind::*;
        let program = [s2r(1, 17), isberd(2, 1, 0), ald_vertex(4, 0x70, 4, 2), ast(0x70, 4, 4), ast(0x0, 4, 4) | ATTR_PATCH, ast(0x10, 4, 2) | ATTR_PATCH, ast(0x20, 4, 1) | ATTR_PATCH, exit()];
        let builtin = |b: B| BuiltIn(b as u32);
        let expected = [builtin(B::InvocationId), PerVertex { attr: 0x70, output: false }, PerVertex { attr: 0x70, output: true }, builtin(B::TessLevelOuter), builtin(B::TessLevelInner), Patch { location: 0 }];
        assert_eq!(stage_interface(Stage::TessControl { vertices: 3 }, &program), expected);
    }

    #[test]
    fn tess_eval_program_reads_the_coordinate_and_patch_and_writes_position() {
        use ResourceKind::*;
        let program = [s2r(0, 0), ald_vertex(2, 0x2f0, 2, 0) | ATTR_OUTPUT, isberd(1, 255, 2), ald_vertex(4, 0x70, 4, 1), ald(8, 0x0, 4) | ATTR_PATCH, ald(12, 0x20, 1) | ATTR_PATCH, ffma(4, 2, 8, 4), ast(0x70, 4, 4), exit()];
        let builtin = |b: B| BuiltIn(b as u32);
        let expected = [builtin(B::TessCoord), PerVertex { attr: 0x70, output: false }, builtin(B::TessLevelOuter), Patch { location: 0 }, builtin(B::Position)];
        assert_eq!(stage_interface(Stage::TessEval { mode: 1, spacing: 0, order: 3 }, &program), expected);
    }

    #[test]
    fn geometry_program_turns_a_point_into_a_triangle() {
        let corners = [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5)].map(|(x, y)| [fadd_imm(0, 4, x), fadd_imm(1, 5, y), ast(0x70, 0, 2), out(2, 2, OUT_EMIT)]);
        let geometry = [&[isberd(1, 255, 0), ald_vertex(4, 0x70, 4, 1), ast(0x78, 6, 2)][..], &corners.concat(), &[out(2, 2, OUT_CUT), exit()]].concat();
        let fragment = [mov_imm(0, 1.0f32.to_bits()), mov_imm(1, 0), mov_imm(2, 0), mov_imm(3, 1.0f32.to_bits()), exit()];
        let stages = [
            (Stage::Vertex { uint: 0, sint: 0 }, &[ald(0, 0x80, 4), ast(0x70, 0, 4), exit()][..]),
            (Stage::Geometry { input: PrimitiveTopology::Points, output: 7, max_vertices: 3, invocations: 1 }, &geometry),
            (Stage::Fragment { flat: 0, screen_linear: 0, depth: false }, &fragment),
        ];
        for (stage, program) in stages {
            stage_interface(stage, program);
        }
        let Some(state) = vk_state() else { return };
        let [vertex, geometry, fragment] = stages.map(|(stage, program)| emit_module(Some(&state), stage, |decoder| decoder.translate_program(program)).words());
        let bindings = [vk::VertexInputBindingDescription { binding: 0, stride: 8, input_rate: vk::VertexInputRate::VERTEX }];
        let attributes = [vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 }];
        let topology = vk::PrimitiveTopology::POINT_LIST;
        let layout = VertexLayout { bindings: &bindings, attributes: &attributes, topology };
        let pipeline = GraphicsPipeline::new(&state, &vertex, Some(&geometry), &fragment, [&[], &[]], &layout, &TargetState { blend: opaque(), depth: None }).unwrap();
        let target = RenderTarget::new(&state, SIZE, SIZE, None).unwrap();
        let mut point = HostBuffer::new(&state, 8).unwrap();
        point.write(&[0u8; 8]).unwrap();
        let raster = RasterState {
            viewport: vk::Viewport { x: 0.0, y: 0.0, width: SIZE as f32, height: SIZE as f32, min_depth: 0.0, max_depth: 1.0 },
            scissor: target.extent.into(),
            cull_face: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            topology,
            depth_test: false,
            depth_write: false,
            depth_func: vk::CompareOp::ALWAYS,
            blend_const: [0.0; 4],
            blend: opaque(),
        };
        let draw = DrawState {
            vertex_buffers: &[(&point, 8)],
            index_buffer: None,
            vertex_count: 1,
            first_vertex: 0,
            first_instance: 0,
            instance_count: 1,
            clear: Some([1.0; 4]),
            clear_depth: None,
            load: false,
            raster,
        };
        state.draw(&pipeline, &target, &draw).unwrap();
        let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
        target.read(&mut pixels).unwrap();
        let corners = [(17, 17), (17, 45), (45, 17), (40, 40)].map(|(x, y)| pixel(&pixels, x, y));
        assert_eq!(corners, [[255, 0, 0, 255], [255, 0, 0, 255], [255, 0, 0, 255], [255; 4]]);
    }
}
