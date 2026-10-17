//! HDR dungeon materials and effects using the existing Vulkan GLSL pipeline path.
use super::*;
use crate::gpu::{
    compute::{AccelerationStructure, ComputePipeline, HostBuffer, HostImage, Triangles},
    graphics::{DrawState, GraphicsPipeline, RasterState, RenderTarget, TargetState, VertexLayout},
    texture::TexturePool,
};
use ash::vk;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub static SHOWCASE: AtomicBool = AtomicBool::new(true);
pub static DISABLED: AtomicU32 = AtomicU32::new(0);
pub static PROFILE: AtomicBool = AtomicBool::new(false);
pub static RAYS: AtomicBool = AtomicBool::new(true);
fn named(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
fn code(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect()
}
fn pack(vertices: &[Vertex]) -> Vec<u8> {
    vertices
        .iter()
        .flat_map(|v| {
            floats(&[
                v.pos[0],
                v.pos[1],
                v.pos[2],
                v.normal[0],
                v.normal[1],
                v.normal[2],
                v.uv[0],
                v.uv[1],
                v.tint[0],
                v.tint[1],
                v.tint[2],
                v.material,
            ])
        })
        .collect()
}
const HDR: vk::Format = vk::Format::R16G16B16A16_SFLOAT;
const DEPTH: vk::Format = vk::Format::D32_SFLOAT;

pub(super) struct Renderer {
    state: &'static VkState,
    timestamps: vk::QueryPool,
    cones: GraphicsPipeline<'static>,
    _cone_instances: HostBuffer<'static>,
    cone_count: u32,
    opaque: GraphicsPipeline<'static>,
    sprites: GraphicsPipeline<'static>,
    particles_step: ComputePipeline<'static>,
    fog: ComputePipeline<'static>,
    _fog_buffer: HostBuffer<'static>,
    post: ComputePipeline<'static>,
    target: RenderTarget<'static>,
    reflection: RenderTarget<'static>,
    uniforms: HostBuffer<'static>,
    particle_time: HostBuffer<'static>,
    vertices: HostBuffer<'static>,
    indices: HostBuffer<'static>,
    particles: HostBuffer<'static>,
    _lights: HostBuffer<'static>,
    shadows_ready: bool,
    shadow_maps: HostBuffer<'static>,
    _geometry: HostBuffer<'static>,
    _textures: Vec<HostImage<'static>>,
    _structure: Option<AccelerationStructure<'static>>,
    output: HostBuffer<'static>,
    count: u32,
    water_indices: u32,
    water_vertex: usize,
    grid: [usize; 2],
    emitter_count: usize,
    extra_particles: usize,
    pub showcase: bool,
    pub rays: bool,
}
impl Renderer {
    pub(super) fn new(state: &'static VkState, world: &World, mesh: &Builder) -> Result<Self, String> {
        use vk::DescriptorType as D;
        let timestamps = unsafe {
            state.device.create_query_pool(
                &vk::QueryPoolCreateInfo::default().query_type(vk::QueryType::TIMESTAMP).query_count(2),
                None,
            )
        }
        .map_err(named)?;
        let showcase = SHOWCASE.load(Ordering::Relaxed);
        let rays = state.acceleration_structure.is_some();
        let mut vertices = mesh.vertices.clone();
        let water_vertex = vertices.len();
        let grid = water_grid();
        vertices.resize(water_vertex + grid[0] * grid[1], Vertex::default());
        let mut indices = mesh.indices.clone();
        let count = indices.len() as u32;
        for y in 0..grid[1] - 1 {
            for x in 0..grid[0] - 1 {
                let a = (water_vertex + y * grid[0] + x) as u32;
                let row = grid[0] as u32;
                indices.extend([a, a + row, a + 1, a + 1, a + row, a + row + 1]);
            }
        }
        let water_indices = indices.len() as u32 - count;
        let mut vb = HostBuffer::new(state, (vertices.len() * 48) as u64).map_err(named)?;
        vb.write(&pack(&vertices)).map_err(named)?;
        let mut ib = HostBuffer::new(state, (indices.len() * 4) as u64).map_err(named)?;
        ib.write(&indices.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()).map_err(named)?;
        let mut ray_vertices = mesh.vertices.clone();
        let mut ray_indices = mesh.indices.clone();
        let mut cones_data = Vec::new();
        for prop in &world.props {
            let (origin, height, radius) = match *prop {
                Prop::Stalactite(p, h, r) => (p, -h, r),
                Prop::Stalagmite(p, h, r) => (p, h, r),
                _ => continue,
            };
            let tilt = [(origin[0] * 3.).sin() * 0.17, (origin[2] * 2.).cos() * 0.17];
            cones_data.extend([origin[0], origin[1], origin[2], height, radius, tilt[0], tilt[1], 0.]);
            let first = ray_vertices.len() as u32;
            let point = |u: f32, t: f32| {
                let a = u * TAU;
                let r = ((1. - t).powf(1.4) + 0.006) * (1. + 0.08 * (a * 5. + t * 8.).sin()) * radius;
                add(origin, [r * a.cos() + t * tilt[0], t * height, r * a.sin() + t * tilt[1]])
            };
            for y in 0..=12 {
                for x in 0..=24 {
                    let (u, t) = (x as f32 / 24., y as f32 / 12.);
                    let (normal, _) = frame(&point, u, t);
                    ray_vertices.push(Vertex {
                        pos: point(u, t),
                        normal: scale(normal, -height.signum()),
                        uv: [u, t],
                        tint: [0.65, 0.69, 0.62],
                        material: 1.,
                    });
                }
            }
            for y in 0..12 {
                for x in 0..24 {
                    let a = first + y * 25 + x;
                    ray_indices.extend([a, a + 1, a + 25, a + 1, a + 26, a + 25]);
                }
            }
        }
        let cone_count = cones_data.len() as u32 / 8;
        let mut cone_instances = HostBuffer::new(state, (cones_data.len() * 4) as u64).map_err(named)?;
        cone_instances.write(&floats(&cones_data)).map_err(named)?;
        let mut geo = HostBuffer::new(state, (ray_indices.len() * 48) as u64).map_err(named)?;
        geo.write(
            &ray_indices
                .iter()
                .flat_map(|&i| {
                    let v = ray_vertices[i as usize];
                    floats(&[
                        v.pos[0],
                        v.pos[1],
                        v.pos[2],
                        v.material,
                        v.normal[0],
                        v.normal[1],
                        v.normal[2],
                        0.,
                        v.tint[0],
                        v.tint[1],
                        v.tint[2],
                        0.,
                    ])
                })
                .collect::<Vec<_>>(),
        )
        .map_err(named)?;
        let structure = if rays {
            let positions: Vec<_> = ray_vertices.iter().map(|v| v.pos).collect();
            Some(
                AccelerationStructure::new(
                    state,
                    &[Triangles {
                        positions: &positions,
                        indices: &ray_indices,
                        transform: [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]],
                        opaque: true,
                    }],
                )
                .map_err(named)?,
            )
        } else {
            None
        };
        let lights_data: Vec<_> = world
            .lights
            .iter()
            .flat_map(|l| [l.pos[0], l.pos[1], l.pos[2], l.flame, l.colour[0], l.colour[1], l.colour[2], 0.])
            .collect();
        let mut lights = HostBuffer::new(state, (lights_data.len() * 4) as u64).map_err(named)?;
        lights.write(&floats(&lights_data)).map_err(named)?;
        let uniforms = HostBuffer::new(state, 256).map_err(named)?;
        let particle_time = HostBuffer::new(state, 16).map_err(named)?;
        let per_emitter = if showcase { 96 } else { 32 };
        let extra_particles = world.drips.len() * 2
            + world.props.iter().filter(|p| matches!(p, Prop::Web(..) | Prop::Lantern(..))).count()
            + 112;
        let particles = HostBuffer::new(state, ((world.lights.len() * per_emitter + extra_particles) * 48) as u64)
            .map_err(named)?;
        let target = RenderTarget::with_format(state, WIDTH, HEIGHT, Some(DEPTH), None, HDR).map_err(named)?;
        let reflection = RenderTarget::with_format(state, WIDTH, HEIGHT, Some(DEPTH), None, HDR).map_err(named)?;
        let output = HostBuffer::new(state, u64::from(WIDTH * HEIGHT * 4)).map_err(named)?;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../demos/assets/dungeon");
        let mut textures = Vec::new();
        for asset in ["PavingStones142", "Rock035", "Metal055C"] {
            for slot in ["colour", "normal", "surface"] {
                let bytes = std::fs::read(root.join(format!("{asset}-{slot}.rgba8")))
                    .map_err(|e| format!("dungeon textures: {e}; run demos/assets/fetch.sh"))?;
                let tic = [
                    8 | 0o2222 << 7 | 0o5432 << 19,
                    0,
                    3 << 21,
                    1 << 3 | 11 << 28,
                    2047 | 1 << 23,
                    2047 | 1 << 31,
                    0,
                    0,
                ];
                let tic: Vec<u8> = tic.iter().flat_map(|v: &u32| v.to_le_bytes()).collect();
                let header = TexturePool { tic: &tic, tsc: &[] }.texture(0)?;
                let sampler = vk::SamplerCreateInfo::default()
                    .mag_filter(vk::Filter::LINEAR)
                    .min_filter(vk::Filter::LINEAR)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .max_lod(11.);
                textures.push(HostImage::new(state, &header, &bytes, &sampler).map_err(named)?);
            }
        }
        let attributes: Vec<_> = [
            (0, vk::Format::R32G32B32_SFLOAT, 0),
            (1, vk::Format::R32G32B32_SFLOAT, 12),
            (2, vk::Format::R32G32_SFLOAT, 24),
            (3, vk::Format::R32G32B32_SFLOAT, 32),
            (4, vk::Format::R32_SFLOAT, 44),
        ]
        .into_iter()
        .map(|(location, format, offset)| vk::VertexInputAttributeDescription { location, binding: 0, format, offset })
        .collect();
        let bindings =
            [vk::VertexInputBindingDescription { binding: 0, stride: 48, input_rate: vk::VertexInputRate::VERTEX }];
        let layout = VertexLayout {
            bindings: &bindings,
            attributes: &attributes,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        };
        let blend = vk::PipelineColorBlendAttachmentState::default().color_write_mask(vk::ColorComponentFlags::RGBA);
        let shadow_maps = HostBuffer::new(state, (world.lights.len() * 6 * 256 * 256 * 8) as u64).map_err(named)?;
        let mut fragment = vec![(0, D::UNIFORM_BUFFER), (1, D::STORAGE_BUFFER)];
        fragment.extend((2..11).map(|b| (b, D::COMBINED_IMAGE_SAMPLER)));
        fragment.extend((11..14).map(|b| (b, D::STORAGE_BUFFER)));
        fragment.push((15, D::STORAGE_BUFFER));
        if rays {
            fragment.push((14, D::ACCELERATION_STRUCTURE_KHR));
        }
        let shader = if rays {
            include_bytes!("../../../demos/shaders/dungeon_rt.frag.spv").as_slice()
        } else {
            include_bytes!("../../../demos/shaders/dungeon.frag.spv").as_slice()
        };
        let mut opaque = GraphicsPipeline::with_format(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon.vert.spv")),
            None,
            &code(shader),
            [&[(0, D::UNIFORM_BUFFER)], &fragment],
            &layout,
            &TargetState { blend, depth: Some(DEPTH) },
            HDR,
        )
        .map_err(named)?;
        opaque.descriptors[0].bind(&[(0, &uniforms)]);
        opaque.descriptors[1].bind(&[
            (0, &uniforms),
            (1, &lights),
            (11, &reflection.staging),
            (12, &target.staging),
            (13, &geo),
            (15, &shadow_maps),
        ]);
        opaque.descriptors[1]
            .bind_images(&textures.iter().enumerate().map(|(i, t)| (i as u32 + 2, t)).collect::<Vec<_>>());
        if let Some(s) = &structure {
            opaque.descriptors[1].bind_acceleration_structure(14, s);
        }
        let mut cones = GraphicsPipeline::with_format(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_cones.vert.spv")),
            None,
            &code(shader),
            [&[(0, D::UNIFORM_BUFFER), (1, D::STORAGE_BUFFER)], &fragment],
            &VertexLayout { bindings: &[], attributes: &[], topology: vk::PrimitiveTopology::TRIANGLE_LIST },
            &TargetState { blend, depth: Some(DEPTH) },
            HDR,
        )
        .map_err(named)?;
        cones.descriptors[0].bind(&[(0, &uniforms), (1, &cone_instances)]);
        cones.descriptors[1].bind(&[
            (0, &uniforms),
            (1, &lights),
            (11, &reflection.staging),
            (12, &target.staging),
            (13, &geo),
            (15, &shadow_maps),
        ]);
        cones.descriptors[1]
            .bind_images(&textures.iter().enumerate().map(|(i, t)| (i as u32 + 2, t)).collect::<Vec<_>>());
        if let Some(s) = &structure {
            cones.descriptors[1].bind_acceleration_structure(14, s);
        }
        let mut sprites = GraphicsPipeline::with_format(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_sprite.vert.spv")),
            None,
            &code(include_bytes!("../../../demos/shaders/dungeon_sprite.frag.spv")),
            [
                &[(0, D::UNIFORM_BUFFER), (1, D::STORAGE_BUFFER)],
                &[(0, D::UNIFORM_BUFFER), (1, D::STORAGE_BUFFER), (12, D::STORAGE_BUFFER)],
            ],
            &VertexLayout { bindings: &[], attributes: &[], topology: vk::PrimitiveTopology::TRIANGLE_LIST },
            &TargetState { blend, depth: Some(DEPTH) },
            HDR,
        )
        .map_err(named)?;
        sprites.descriptors[0].bind(&[(0, &uniforms), (1, &particles)]);
        sprites.descriptors[1].bind(&[(0, &uniforms), (1, &lights), (12, &target.staging)]);
        let mut particles_step = ComputePipeline::new(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_particles.comp.spv")),
            &[(0, D::STORAGE_BUFFER), (1, D::STORAGE_BUFFER), (2, D::UNIFORM_BUFFER)],
        )
        .map_err(named)?;
        particles_step.descriptors.bind(&[(0, &lights), (1, &particles), (2, &particle_time)]);
        let fog_buffer = HostBuffer::new(state, u64::from(WIDTH / 4 * (HEIGHT / 4) * 16)).map_err(named)?;
        let mut fog = ComputePipeline::new(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_fog.comp.spv")),
            &[(0, D::STORAGE_BUFFER), (1, D::STORAGE_BUFFER), (2, D::UNIFORM_BUFFER), (3, D::STORAGE_BUFFER)],
        )
        .map_err(named)?;
        fog.descriptors.bind(&[(0, &target.staging), (1, &fog_buffer), (2, &uniforms), (3, &lights)]);
        let mut post = ComputePipeline::new(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_post.comp.spv")),
            &[
                (0, D::STORAGE_BUFFER),
                (1, D::STORAGE_BUFFER),
                (2, D::UNIFORM_BUFFER),
                (3, D::STORAGE_BUFFER),
                (4, D::STORAGE_BUFFER),
            ],
        )
        .map_err(named)?;
        post.descriptors.bind(&[(0, &target.staging), (1, &output), (2, &uniforms), (3, &lights), (4, &fog_buffer)]);
        let mut renderer = Self {
            state,
            timestamps,
            cones,
            _cone_instances: cone_instances,
            cone_count,
            opaque,
            sprites,
            particles_step,
            fog,
            _fog_buffer: fog_buffer,
            post,
            target,
            reflection,
            uniforms,
            particle_time,
            vertices: vb,
            indices: ib,
            particles,
            _lights: lights,
            shadows_ready: false,
            shadow_maps,
            _geometry: geo,
            _textures: textures,
            _structure: structure,
            output,
            count,
            water_indices,
            water_vertex,
            grid,
            emitter_count: world.lights.len(),
            extra_particles,
            showcase,
            rays,
        };
        renderer.rays = RAYS.load(Ordering::Relaxed) && rays;
        if !renderer.rays { renderer.bake_shadows(world)?; }
        Ok(renderer)
    }
    fn bake_shadows(&mut self, world: &World) -> Result<(), String> {
        let target = RenderTarget::with_format(self.state, 256, 256, Some(DEPTH), None, HDR).map_err(named)?;
        let projection = [[1., 0., 0., 0.], [0., -1., 0., 0.], [0., 0., -30. / 29.95, -1.], [0., 0., -1.5 / 29.95, 0.]];
        for (i, light) in world.lights.iter().enumerate() {
            for (face, dir) in [[1., 0., 0.], [-1., 0., 0.], [0., 1., 0.], [0., -1., 0.], [0., 0., 1.], [0., 0., -1.]]
                .into_iter()
                .enumerate()
            {
                let right = unit(cross(dir, if dir[1].abs() > 0.5 { [0., 0., 1.] } else { UP }));
                let up = cross(right, dir);
                let axes = [right, up, scale(dir, -1.)];
                let view = std::array::from_fn(|c| {
                    std::array::from_fn(|r| {
                        if r == 3 {
                            if c == 3 { 1. } else { 0. }
                        } else if c == 3 {
                            -dot(axes[r], light.pos)
                        } else {
                            axes[r][c]
                        }
                    })
                });
                let vp = multiply(projection, view);
                self.uniforms(vp, vp, light.pos, 0., right, up, world, true, 0.)?;
                self.uniforms.write_at(216, &2f32.to_le_bytes()).map_err(named)?;
                self.draw(&target, 0, self.count, true, false)?;
                self.draw(&target, u32::MAX, 24 * 12 * 6, false, false)?;
                target.readback().map_err(named)?;
                self.shadow_maps
                    .write_at(
                        ((i * 6 + face) * 256 * 256 * 8) as u64,
                        target.staging.view(0..256 * 256 * 8).map_err(named)?,
                    )
                    .map_err(named)?;
            }
        }
        self.shadows_ready=true;
        Ok(())
    }
    fn uniforms(
        &mut self,
        vp: Matrix,
        reflected: Matrix,
        eye: V3,
        seconds: f32,
        right: V3,
        up: V3,
        world: &World,
        reflection: bool,
        plane: f32,
    ) -> Result<(), String> {
        let mut u = vp.as_flattened().to_vec();
        u.extend(reflected.as_flattened());
        u.extend([
            eye[0],
            eye[1],
            eye[2],
            seconds,
            right[0],
            right[1],
            right[2],
            self.showcase as u8 as f32,
            up[0],
            up[1],
            up[2],
            (self.rays && !reflection) as u8 as f32,
            world.river.lo[0],
            world.river.lo[2],
            world.river.hi[0],
            world.river.hi[2],
        ]);
        u.extend(world.grate.centre());
        u.push(DISABLED.load(Ordering::Relaxed) as f32);
        u.extend([WIDTH as f32, HEIGHT as f32, reflection as u8 as f32, plane]);
        let mut nearest: Vec<_> = (0..world.lights.len()).collect();
        nearest.sort_by(|&a, &b| {
            let score = |i: usize| {
                world.lights[i].colour[0] / (1. + dot(sub(world.lights[i].pos, eye), sub(world.lights[i].pos, eye)))
            };
            score(b).total_cmp(&score(a))
        });
        u.extend(nearest.iter().take(4).map(|&i| i as f32));
        self.uniforms.write(&floats(&u)).map_err(named)
    }
    fn draw(&self, target: &RenderTarget, first: u32, count: u32, clear: bool, sprites: bool) -> Result<(), String> {
        let cones = first == u32::MAX;
        let disabled=DISABLED.load(Ordering::Relaxed);
        if !sprites && !cones && first>0 && disabled&2!=0 { return Ok(()); }
        let blend = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(sprites)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .src_alpha_blend_factor(vk::BlendFactor::ZERO)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE);
        let raster = RasterState {
            viewport: vk::Viewport {
                x: 0.,
                y: 0.,
                width: target.extent.width as f32,
                height: target.extent.height as f32,
                min_depth: 0.,
                max_depth: 1.,
            },
            scissor: target.extent.into(),
            cull_face: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            depth_test: true,
            depth_write: !sprites && (first == 0 || cones),
            depth_func: vk::CompareOp::LESS_OR_EQUAL,
            blend_const: [0.; 4],
            blend,
        };
        let vb = [(&self.vertices, 48)];
        self.state
            .draw_profiled(
                if sprites {
                    &self.sprites
                } else if cones {
                    &self.cones
                } else {
                    &self.opaque
                },
                target,
                &DrawState {
                    vertex_buffers: if sprites || cones { &[] } else { &vb },
                    index_buffer: if sprites || cones { None } else { Some((&self.indices, vk::IndexType::UINT32)) },
                    vertex_count: count,
                    first_vertex: if cones { 0 } else { first },
                    first_instance: 0,
                    instance_count: if sprites {
                        (self.particles.size / 48) as u32
                    } else if cones {
                        self.cone_count
                    } else {
                        1
                    },
                    clear: clear.then_some(if target.extent.width == 256 {
                        [30.; 4]
                    } else {
                        [0.008, 0.012, 0.018, 40.]
                    }),
                    clear_depth: clear.then_some(1.),
                    load: false,
                    raster,
                },
                (PROFILE.load(Ordering::Relaxed) && target.extent.width > 256).then_some(self.timestamps),
            )
            .map_err(named)?;
        if target.extent.width == 256 {
            return Ok(());
        }
        self.report(if cones {
            "cones"
        } else if sprites {
            "sprites"
        } else if first > 0 {
            "water"
        } else if std::ptr::eq(target, &self.reflection) {
            "reflection"
        } else {
            "opaque"
        })
    }
    pub(super) fn query_pool(&self)->Option<vk::QueryPool> { PROFILE.load(Ordering::Relaxed).then_some(self.timestamps) }
    pub(super) fn report(&self, pass: &str) -> Result<(), String> {
        if PROFILE.load(Ordering::Relaxed) {
            let mut ticks = [0u64; 2];
            unsafe {
                self.state.device.get_query_pool_results(
                    self.timestamps,
                    0,
                    &mut ticks,
                    vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                )
            }
            .map_err(named)?;
            let period = unsafe { self.state.instance.get_physical_device_properties(self.state.physical_device) }
                .limits
                .timestamp_period;
            eprintln!("dungeon_gpu,{pass},{:.4}", (ticks[1] - ticks[0]) as f64 * period as f64 / 1e6);
        }
        Ok(())
    }
    pub(super) fn frame(
        &mut self,
        world: &World,
        eye: V3,
        view: Matrix,
        projection: Matrix,
        seconds: f32,
        right: V3,
        up: V3,
        waves: &[[f32; 4]],
    ) -> Result<(), String> {
        if !self.rays && !self.shadows_ready {self.bake_shadows(world)?;}
        let grid = self.grid;
        let water: Vec<_> = waves
            .iter()
            .enumerate()
            .map(|(i, &[h, sx, sz, foam])| {
                let uv = [(i % grid[0]) as f32 / (grid[0] - 1) as f32, (i / grid[0]) as f32 / (grid[1] - 1) as f32];
                let x = world.river.lo[0] + uv[0] * (world.river.hi[0] - world.river.lo[0]);
                let z = world.river.lo[2] + uv[1] * (world.river.hi[2] - world.river.lo[2]);
                let a = x * 3.2 + z * 1.3 - seconds * 1.4;
                let b = x * 1.7 - z * 4.2 + seconds * 1.1;
                let h = h + 0.012 * a.sin() + 0.008 * b.sin();
                let sx = sx + 0.0384 * a.cos() + 0.0136 * b.cos();
                let sz = sz + 0.0156 * a.cos() - 0.0336 * b.cos();
                Vertex {
                    pos: [
                        world.river.lo[0] + uv[0] * (world.river.hi[0] - world.river.lo[0]),
                        WATER + h,
                        world.river.lo[2] + uv[1] * (world.river.hi[2] - world.river.lo[2]),
                    ],
                    normal: unit([-sx, 1., -sz]),
                    uv,
                    tint: [foam; 3],
                    material: 4.,
                    ..Default::default()
                }
            })
            .collect();
        self.vertices.write_at((self.water_vertex * 48) as u64, &pack(&water)).map_err(named)?;
        let per_emitter = if self.showcase { 96 } else { 32 };
        self.particle_time.write(&floats(&[seconds, per_emitter as f32, 0., 0.])).map_err(named)?;
        self.state.dispatch_profiled(&self.particles_step, [(self.emitter_count * per_emitter).div_ceil(64) as u32, 1, 1],self.query_pool()).map_err(named)?;
        self.report("particles")?;
        let mut extras = Vec::new();
        let mut particle = |p: V3, kind: f32, size: [f32; 2], seed: f32, colour: [f32; 4]| {
            extras.extend([
                p[0], p[1], p[2], kind, size[0], size[1], seed, 0., colour[0], colour[1], colour[2], colour[3],
            ]);
        };
        for (i, &(tip, floor)) in world.drips.iter().enumerate() {
            let period = 2.4 + (i % 5) as f32 * 0.3;
            let fall = ((tip[1] - floor) / 4.9).sqrt();
            let age = (seconds + i as f32 * 0.71).rem_euclid(period);
            particle(
                [tip[0], tip[1] - 4.9 * age.min(fall).powi(2), tip[2]],
                4.,
                [0.013, 0.05],
                i as f32,
                [0.4, 0.65, 0.8, if age < fall { 0.8 } else { 0. }],
            );
            let splash = ((age - fall) / 0.7).clamp(0., 1.);
            particle(
                [tip[0], floor + 0.015, tip[2]],
                5.,
                [0.02 + splash * 0.3; 2],
                i as f32,
                [0.35, 0.55, 0.6, if age >= fall { (1. - splash) * 0.55 } else { 0. }],
            );
        }
        let mut rng = Rng(117);
        for i in 0..104 {
            let seed = i as f32;
            if i < 48 {
                let lamp = &world.lights[i % world.lights.len()];
                let p = add(
                    lamp.pos,
                    [rng.range(-2., 2.), (rng.next() * 3. + seconds * 0.07).rem_euclid(3.) - 1., rng.range(-2., 2.)],
                );
                particle(p, 3., [0.004; 2], seed, [0.3, 0.23, 0.12, 0.6]);
            } else {
                let x = world.river.lo[0]
                    + (rng.next() * (world.river.hi[0] - world.river.lo[0]) + seconds * 0.16)
                        .rem_euclid(world.river.hi[0] - world.river.lo[0]);
                let z = rng.range(world.river.lo[2] + 0.1, world.river.hi[2] - 0.1);
                let leaf = i % 5 == 0;
                let height = if leaf { WATER + 0.035 } else { WATER - 0.16 + (seconds * 0.08 + seed).fract() * 0.2 };
                particle(
                    [x, height, z],
                    if leaf { 7. } else { 6. },
                    [if leaf { 0.06 } else { 0.02 }; 2],
                    seed,
                    [0.28, 0.19, 0.055, 0.75],
                );
            }
        }
        for prop in &world.props {
            match *prop {
                Prop::Web(p, _) => particle(add(p, [0., -0.65, 0.]), 8., [0.7, 0.65], 0., [0.25, 0.29, 0.26, 0.35]),
                Prop::Lantern(p, yaw) => {
                    particle(add(p, turn(yaw, [0.235, 0.2, 0.])), 9., [0.16, 0.11], yaw, [3., 0.65, 0.015, 1.])
                }
                _ => {}
            }
        }
        let chamber = world.zones[world.bats].space.centre();
        for i in 0..8 {
            let t = seconds * 1.8 + i as f32;
            let p = add(chamber, [0.7 * t.cos(), 0.5 + 0.2 * t.sin(), 0.7 * (t * 1.2).sin()]);
            particle(
                p,
                10.,
                [0.22, 0.025 + 0.1 * (seconds * 69. + i as f32).sin().abs()],
                i as f32,
                [0.008, 0.006, 0.004, 1.],
            );
        }
        debug_assert_eq!(extras.len(), self.extra_particles * 12);
        self.particles.write_at((self.emitter_count * per_emitter * 48) as u64, &floats(&extras)).map_err(named)?;
        let plane = if world.zones[world.cave].space.contains(eye) { WATER } else { 0. };
        let reflect = [[1., 0., 0., 0.], [0., -1., 0., 0.], [0., 0., 1., 0.], [0., plane * 2., 0., 1.]];
        let vp = multiply(projection, view);
        let reflected = multiply(vp, reflect);
        let reflected_eye = [eye[0], plane * 2. - eye[1], eye[2]];
        self.uniforms(reflected, reflected, reflected_eye, seconds, right, up, world, true, plane)?;
        self.draw(&self.reflection, 0, self.count, true, false)?;
        self.draw(&self.reflection, u32::MAX, 24 * 12 * 6, false, false)?;
        self.draw(&self.reflection, 0, 6, false, true)?;
        self.reflection.readback().map_err(named)?;
        self.uniforms(vp, reflected, eye, seconds, right, up, world, false, plane)?;
        self.draw(&self.target, 0, self.count, true, false)?;
        self.draw(&self.target, u32::MAX, 24 * 12 * 6, false, false)?;
        self.target.readback().map_err(named)?;
        self.draw(&self.target, self.count, self.water_indices, false, false)?;
        self.draw(&self.target, 0, 6, false, true)?;
        self.target.readback().map_err(named)?;
        self.state
            .dispatch_profiled(
                &self.fog,
                [(WIDTH / 4).div_ceil(8), (HEIGHT / 4).div_ceil(8), 1],
                PROFILE.load(Ordering::Relaxed).then_some(self.timestamps),
            )
            .map_err(named)?;
        self.report("fog")?;
        self.state
            .dispatch_profiled(
                &self.post,
                [WIDTH.div_ceil(8), HEIGHT.div_ceil(8), 1],
                PROFILE.load(Ordering::Relaxed).then_some(self.timestamps),
            )
            .map_err(named)?;
        self.report("post")?;
        crate::display::present_buffer(&self.output, WIDTH, HEIGHT).map_err(named)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.state.device.destroy_query_pool(self.timestamps, None);
        }
    }
}
