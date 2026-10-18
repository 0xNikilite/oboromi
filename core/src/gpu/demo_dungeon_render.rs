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
    command_pool: vk::CommandPool,
    command: vk::CommandBuffer,
    recorded: Option<(u32,bool,u32,bool)>,
    frame_uniforms: HostBuffer<'static>,
    cones: GraphicsPipeline<'static>,
    _cone_instances: HostBuffer<'static>,
    cone_count: u32,
    cone_visible: bool,
    opaque: GraphicsPipeline<'static>,
    sprites: GraphicsPipeline<'static>,
    particles_step: ComputePipeline<'static>,
    fog: ComputePipeline<'static>,
    _bloom_buffer: HostBuffer<'static>,
    _fog_buffer: HostBuffer<'static>,
    post: ComputePipeline<'static>,
    target: RenderTarget<'static>,
    reflection: RenderTarget<'static>,
    uniforms: HostBuffer<'static>,
    particle_time: HostBuffer<'static>,
    vertices: HostBuffer<'static>,
    indices: HostBuffer<'static>,
    particles: HostBuffer<'static>,
    lights: HostBuffer<'static>,
    shadows_ready: bool,
    shadow_slots: Vec<Option<usize>>,
    shadow_size: u32,
    shadow_maps: HostBuffer<'static>,
    _rooms: HostBuffer<'static>,
    _geometry: HostBuffer<'static>,
    _web: HostImage<'static>,
    _textures: Vec<HostImage<'static>>,
    _structure: Option<AccelerationStructure<'static>>,
    output: HostBuffer<'static>,
    count: u32,
    visible_count: u32,
    visible_zones: Vec<bool>,
    batches: Vec<(Box3,Vec<u32>)>,
    water_indices: u32,
    grid: [usize; 2],
    emitter_count: usize,
    extra_particles: usize,
    water_dump: Option<std::path::PathBuf>,
    refraction_dump: Option<HostBuffer<'static>>,
    pub showcase: bool,
    pub rays: bool,
}
impl Renderer {
    fn hanging_drips(world: &World) -> impl Iterator<Item=(V3,f32)> + '_ {
        world.props.iter().filter_map(|prop| match *prop {
            Prop::Stalactite(p,h,_) if p[0]>world.zones[world.cave].space.hi[0]-5.5 && (p[2]-world.river.centre()[2]).abs()<2.5 =>
                Some(([p[0]+(p[0]*3.).sin()*0.17,p[1]-h,p[2]+(p[2]*2.).cos()*0.17],WATER)),
            _ => None,
        })
    }
    pub(super) fn new(state: &'static VkState, world: &World, mesh: &Builder, waves: &HostBuffer) -> Result<Self, String> {
        use vk::DescriptorType as D;
        let timestamps = unsafe {
            state.device.create_query_pool(
                &vk::QueryPoolCreateInfo::default().query_type(vk::QueryType::TIMESTAMP).query_count(24),
                None,
            )
        }
        .map_err(named)?;
        let command_pool=unsafe {state.device.create_command_pool(&vk::CommandPoolCreateInfo::default().queue_family_index(state.queue_family).flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),None)}.map_err(named)?;
        let command=unsafe {state.device.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default().command_pool(command_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1))}.map_err(named)?[0];
        let showcase = SHOWCASE.load(Ordering::Relaxed);
        let rays = RAYS.load(Ordering::Relaxed) && state.acceleration_structure.is_some();
        let mut vertices = mesh.vertices.clone();
        let water_vertex = vertices.len();
        let grid = water_grid();
        for i in 0..grid[0]*grid[1] {
            let uv=[(i%grid[0]) as f32/(grid[0]-1) as f32,(i/grid[0]) as f32/(grid[1]-1) as f32];
            let x=world.river.lo[0]+uv[0]*(world.river.hi[0]-world.river.lo[0]);
            vertices.push(Vertex {pos:[x,WATER,world.river_bank(x,0)+uv[1]*(world.river_bank(x,1)-world.river_bank(x,0))],normal:UP,uv,material:4.,..Default::default()});
        }
        let mut batches:Vec<_>=world.zones.iter().map(|_|(Box3{lo:[f32::INFINITY;3],hi:[f32::NEG_INFINITY;3]},Vec::new())).collect();
        for tri in mesh.indices.chunks_exact(3) {
            let centre=scale(add(add(mesh.vertices[tri[0] as usize].pos,mesh.vertices[tri[1] as usize].pos),mesh.vertices[tri[2] as usize].pos),1./3.);
            let zone=world.zones.iter().position(|z|z.space.contains(centre)).unwrap_or_else(||world.zones.iter().enumerate()
                .min_by(|(_,a),(_,b)|length(sub(a.space.centre(),centre)).total_cmp(&length(sub(b.space.centre(),centre)))).unwrap().0);
            let (bounds,indices)=&mut batches[zone];indices.extend_from_slice(tri);
            for &i in tri {for axis in 0..3 {let p=mesh.vertices[i as usize].pos[axis];bounds.lo[axis]=bounds.lo[axis].min(p);bounds.hi[axis]=bounds.hi[axis].max(p);}}
        }
        let mut indices:Vec<_>=batches.iter().flat_map(|(_,indices)|indices.iter().copied()).collect();
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
                let t=t*(2.-t);
                let a = u * TAU;
                let r = ((1.-t*t).max(0.000004).sqrt()*(0.55+0.45*(-t*4.).exp())) * (1. + 0.12 * (a * 5. + t * 8. + origin[0]).sin()) * radius;
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
        let shadow_size = if showcase {256} else {128};
        let mut shadow_lights: Vec<_> = (0..world.lights.len()).collect();
        if !showcase {
            shadow_lights.sort_by(|&a,&b|world.lights[b].colour[0].total_cmp(&world.lights[a].colour[0]));
            shadow_lights.truncate(10);
        }
        let shadow_slots:Vec<_>=(0..world.lights.len()).map(|i|shadow_lights.iter().position(|&l|l==i)).collect();
        let lights_data: Vec<_> = world
            .lights
            .iter()
            .enumerate().flat_map(|(i,l)| [l.pos[0], l.pos[1], l.pos[2], l.flame, l.colour[0], l.colour[1], l.colour[2], shadow_slots[i].map_or(0.,|slot|slot as f32+1.)])
            .collect();
        let mut lights = HostBuffer::new(state, (lights_data.len() * 4) as u64).map_err(named)?;
        lights.write(&floats(&lights_data)).map_err(named)?;
        let bounds: Vec<_> = world.zones.iter().flat_map(|z| [z.space.lo[0],z.space.lo[2],z.space.hi[0],z.space.hi[2]]).collect();
        let mut rooms=HostBuffer::new(state,(bounds.len()*4) as u64).map_err(named)?;
        rooms.write(&floats(&bounds)).map_err(named)?;
        let frame_uniforms=HostBuffer::new(state,512).map_err(named)?;
        let uniforms = HostBuffer::new(state, 256).map_err(named)?;
        let particle_time = HostBuffer::new(state, 16).map_err(named)?;
        let per_emitter = if showcase { 96 } else { 24 };
        let extra_particles = (world.drips.len()+Self::hanging_drips(world).count()) * 2
            + world.props.iter().filter(|p| matches!(p, Prop::Web(..) | Prop::Lantern(..))).count()
            + 220;
        let particles = HostBuffer::new(state, ((world.lights.len() * per_emitter + extra_particles) * 48) as u64)
            .map_err(named)?;
        let target = RenderTarget::with_format(state, WIDTH, HEIGHT, Some(DEPTH), None, HDR).map_err(named)?;
        let reflection_scale=if showcase {2}else{4};
        let reflection = RenderTarget::with_format(state, WIDTH / reflection_scale, HEIGHT / reflection_scale, Some(DEPTH), None, HDR).map_err(named)?;
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
        let shadow_maps = HostBuffer::new(state, (shadow_lights.len() * 6 * shadow_size as usize * shadow_size as usize * 8) as u64).map_err(named)?;
        let mut fragment = vec![(0, D::UNIFORM_BUFFER), (1, D::STORAGE_BUFFER)];
        fragment.extend((2..11).map(|b| (b, D::COMBINED_IMAGE_SAMPLER)));
        fragment.extend((11..14).map(|b| (b, D::STORAGE_BUFFER)));
        fragment.extend([(15, D::STORAGE_BUFFER),(16, D::STORAGE_BUFFER)]);
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
            [&[(0, D::UNIFORM_BUFFER), (1,D::STORAGE_BUFFER)], &fragment],
            &layout,
            &TargetState { blend, depth: Some(DEPTH) },
            HDR,
        )
        .map_err(named)?;
        opaque.descriptors[0].bind(&[(0, &uniforms),(1,waves)]);
        opaque.descriptors[1].bind(&[
            (0, &uniforms),
            (1, &lights),
            (11, &reflection.staging),
            (12, &target.staging),
            (13, &geo),
            (15, &shadow_maps),
            (16, &rooms),
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
            (16, &rooms),
        ]);
        cones.descriptors[1]
            .bind_images(&textures.iter().enumerate().map(|(i, t)| (i as u32 + 2, t)).collect::<Vec<_>>());
        if let Some(s) = &structure {
            cones.descriptors[1].bind_acceleration_structure(14, s);
        }
        // Red holds torn web strands; green holds the soft smoke density.
        let mut web_bytes=vec![0u8;256*256*4];
        for y in 0..256 {for x in 0..256 {
            let q=[x as f32/255.*2.-1.,y as f32/255.*2.-1.];
            let smoke=(1.-q[0]*q[0]-q[1]*q[1]).max(0.).powf(2.5)*(0.55+0.45*fbm(q[0]*3.,q[1]*3.,64,4,37));
            web_bytes[(y*256+x)*4+1]=(smoke*255.) as u8;
            web_bytes[(y*256+x)*4+3]=255;
        }}
        let mut rng=Rng(831);
        let nodes:Vec<[f32;2]>=(0..85).map(|_| {
            let x=rng.range(0.03,0.97);
            [x,rng.range((x*0.75).min(0.94),0.97)]
        }).collect();
        for (i,&a) in nodes.iter().enumerate() {
            web_bytes[((a[1]*255.) as usize*256+(a[0]*255.) as usize)*4]=70;
            let mut neighbours:Vec<_>=(i+1..nodes.len()).collect();
            neighbours.sort_by(|&j,&k|{
                let d=|n:usize|(nodes[n][0]-a[0]).powi(2)+(nodes[n][1]-a[1]).powi(2);
                d(j).total_cmp(&d(k))
            });
            for &j in neighbours.iter().take(3) {
                if rng.next()<0.22 {continue;}
                let b=nodes[j];let sag=rng.range(0.006,0.04);let opacity=rng.range(65.,175.);
                let point=|t:f32|[(a[0]+(b[0]-a[0])*t)*255.,(a[1]+(b[1]-a[1])*t-sag*(t*PI).sin())*255.];
                for step in 0..8 {
                    let a=point(step as f32/8.);let b=point((step+1) as f32/8.);
                    for y in ((a[1].min(b[1])-1.).max(0.) as usize)..=((a[1].max(b[1])+1.).min(255.) as usize) {
                        for x in ((a[0].min(b[0])-1.).max(0.) as usize)..=((a[0].max(b[0])+1.).min(255.) as usize) {
                            let d=[b[0]-a[0],b[1]-a[1]];
                            let t=(((x as f32-a[0])*d[0]+(y as f32-a[1])*d[1])/(d[0]*d[0]+d[1]*d[1]).max(0.001)).clamp(0.,1.);
                            let distance=((x as f32-a[0]-t*d[0]).powi(2)+(y as f32-a[1]-t*d[1]).powi(2)).sqrt();
                            let alpha=((1.-distance).max(0.)*opacity) as u8;
                            let at=(y*256+x)*4;web_bytes[at]=web_bytes[at].max(alpha);
                        }
                    }
                }
            }
        }
        let tic=[8u32|0o2222<<7|0o5432<<19,0,2<<21,32,255|1<<23,255|1<<31,0,0];
        let tic:Vec<u8>=tic.iter().flat_map(|v|v.to_le_bytes()).collect();
        let header=TexturePool{tic:&tic,tsc:&[]}.texture(0)?;
        let sampler=vk::SamplerCreateInfo::default().mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE).address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let web=HostImage::new(state,&header,&web_bytes,&sampler).map_err(named)?;
        let mut sprites = GraphicsPipeline::with_format(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_sprite.vert.spv")),
            None,
            &code(include_bytes!("../../../demos/shaders/dungeon_sprite.frag.spv")),
            [
                &[(0, D::UNIFORM_BUFFER), (1, D::STORAGE_BUFFER)],
                &[(0, D::UNIFORM_BUFFER), (1, D::STORAGE_BUFFER), (2, D::COMBINED_IMAGE_SAMPLER), (12, D::STORAGE_BUFFER)],
            ],
            &VertexLayout { bindings: &[], attributes: &[], topology: vk::PrimitiveTopology::TRIANGLE_LIST },
            &TargetState { blend, depth: Some(DEPTH) },
            HDR,
        )
        .map_err(named)?;
        sprites.descriptors[0].bind(&[(0, &uniforms), (1, &particles)]);
        sprites.descriptors[1].bind(&[(0, &uniforms), (1, &lights), (12, &target.staging)]);
        sprites.descriptors[1].bind_images(&[(2,&web)]);
        let mut particles_step = ComputePipeline::new(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_particles.comp.spv")),
            &[(0, D::STORAGE_BUFFER), (1, D::STORAGE_BUFFER), (2, D::UNIFORM_BUFFER)],
        )
        .map_err(named)?;
        particles_step.descriptors.bind(&[(0, &lights), (1, &particles), (2, &particle_time)]);
        let fog_buffer = HostBuffer::new(state, u64::from(WIDTH / 4 * (HEIGHT / 4) * 16)).map_err(named)?;
        let bloom_buffer = HostBuffer::new(state, u64::from(WIDTH / 4 * (HEIGHT / 4) * 16)).map_err(named)?;
        let mut fog = ComputePipeline::new(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_fog.comp.spv")),
            &[(0, D::STORAGE_BUFFER), (1, D::STORAGE_BUFFER), (2, D::UNIFORM_BUFFER), (3, D::STORAGE_BUFFER), (4, D::STORAGE_BUFFER)],
        )
        .map_err(named)?;
        fog.descriptors.bind(&[(0, &target.staging), (1, &fog_buffer), (2, &uniforms), (3, &lights), (4, &bloom_buffer)]);
        let mut post = ComputePipeline::new(
            state,
            &code(include_bytes!("../../../demos/shaders/dungeon_post.comp.spv")),
            &[
                (0, D::STORAGE_BUFFER),
                (1, D::STORAGE_BUFFER),
                (2, D::UNIFORM_BUFFER),
                (3, D::STORAGE_BUFFER),
                (4, D::STORAGE_BUFFER),
                (5, D::STORAGE_BUFFER),
            ],
        )
        .map_err(named)?;
        post.descriptors.bind(&[(0, &target.staging), (1, &output), (2, &uniforms), (3, &lights), (4, &fog_buffer), (5, &bloom_buffer)]);
        let mut renderer = Self {
            state,
            timestamps,
            command_pool, command, recorded:None, frame_uniforms,
            cones,
            _cone_instances: cone_instances,
            cone_count,
            cone_visible: true,
            opaque,
            sprites,
            particles_step,
            fog,
            _fog_buffer: fog_buffer,
            _bloom_buffer: bloom_buffer,
            post,
            target,
            reflection,
            uniforms,
            particle_time,
            vertices: vb,
            indices: ib,
            particles,
            lights,
            shadows_ready: false,
            shadow_maps,
            shadow_slots,
            shadow_size,
            _rooms: rooms,
            _geometry: geo,
            _web: web,
            _textures: textures,
            _structure: structure,
            output,
            count,
            visible_count: count,
            visible_zones: vec![true;batches.len()],
            batches,
            water_indices,
            grid,
            emitter_count: world.lights.len(),
            extra_particles,
            water_dump: std::env::var_os("DUNGEON_WATER_DUMP").map(std::path::PathBuf::from),
            refraction_dump: if std::env::var_os("DUNGEON_WATER_DUMP").is_some() {Some(HostBuffer::new(state,u64::from(WIDTH*HEIGHT*8)).map_err(named)?)} else {None},
            showcase,
            rays,
        };
        if PROFILE.load(Ordering::Relaxed) {eprintln!("dungeon_settings,showcase={showcase},ray_queries={rays}");}
        renderer.rays = RAYS.load(Ordering::Relaxed) && rays;
        if !renderer.rays { renderer.bake_shadows(world)?; }
        Ok(renderer)
    }
    fn bake_shadows(&mut self, world: &World) -> Result<(), String> {
        let target = RenderTarget::with_format(self.state, self.shadow_size, self.shadow_size, Some(DEPTH), None, HDR).map_err(named)?;
        let projection = [[1., 0., 0., 0.], [0., -1., 0., 0.], [0., 0., -30. / 29.95, -1.], [0., 0., -1.5 / 29.95, 0.]];
        for (i, light) in world.lights.iter().enumerate() {
            let Some(slot)=self.shadow_slots[i] else {continue;};
            let bytes=u64::from(self.shadow_size*self.shadow_size*8);
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
                self.uniforms(vp, vp, light.pos, 0., right, up, 1., world, true, 0., None)?;
                self.uniforms.write_at(216, &2f32.to_le_bytes()).map_err(named)?;
                self.draw(&target, 0, self.count, true, false)?;
                self.draw(&target, u32::MAX, 24 * 12 * 6, false, false)?;
                target.readback().map_err(named)?;
                self.shadow_maps
                    .write_at(
                        (slot * 6 + face) as u64 * bytes,
                        target.staging.view(0..bytes).map_err(named)?,
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
        fade: f32,
        world: &World,
        reflection: bool,
        plane: f32,
        offset: Option<u64>,
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
        u.push(DISABLED.load(Ordering::Relaxed) as f32 + fade.min(0.999) * 0.5);
        u.extend([WIDTH as f32, HEIGHT as f32, reflection as u8 as f32, plane]);
        let focus=sub(eye,scale(cross(right,up),4.));
        let mut nearest: Vec<_> = (0..world.lights.len()).collect();
        nearest.sort_by(|&a, &b| {
            let score = |i: usize| {
                world.lights[i].colour[0] / (1. + dot(sub(world.lights[i].pos, focus), sub(world.lights[i].pos, focus)))
            };
            score(b).total_cmp(&score(a))
        });
        u.extend(nearest.iter().take(4).map(|&i| i as f32));
        u.extend([self.grid[0] as f32,self.grid[1] as f32,nearest[4] as f32,nearest[5] as f32]);
        match offset {Some(offset)=>self.frame_uniforms.write_at(offset,&floats(&u)),None=>self.uniforms.write(&floats(&u))}.map_err(named)
    }
    fn draw(&self, target: &RenderTarget, first: u32, count: u32, clear: bool, sprites: bool) -> Result<(), String> {
        self.draw_commands(None,target,first,count,clear,sprites)
    }
    fn draw_commands(&self, command:Option<(vk::CommandBuffer,u32)>, target:&RenderTarget, first:u32,count:u32,clear:bool,sprites:bool)->Result<(),String> {
        let cones = first == u32::MAX;
        if cones && !self.cone_visible {return Ok(());}
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
        let pipeline=if sprites {
                    &self.sprites
                } else if cones {
                    &self.cones
                } else {
                    &self.opaque
                };
        let draw=DrawState {
                    vertex_buffers: if sprites || cones { &[] } else { &vb },
                    index_buffer: if sprites || cones { None } else { Some((&self.indices, vk::IndexType::UINT32)) },
                    vertex_count: if cones && !self.showcase {12*8*6} else {count},
                    first_vertex: if cones { 0 } else { first },
                    first_instance: 0,
                    instance_count: if sprites {
                        (self.particles.size / 48) as u32
                    } else if cones {
                        self.cone_count
                    } else {
                        1
                    },
                    clear: clear.then_some(if target.extent.width == self.shadow_size {
                        [30.; 4]
                    } else {
                        [0.008, 0.012, 0.018, 40.]
                    }),
                    clear_depth: clear.then_some(1.),
                    load: false,
                    raster,
                };
        if let Some((cmd,offset))=command {
            self.state.record_draw(cmd,pipeline,target,&draw,PROFILE.load(Ordering::Relaxed).then_some((self.timestamps,offset)));
            return Ok(());
        }
        self.state.draw_profiled(pipeline,target,&draw,(PROFILE.load(Ordering::Relaxed) && target.extent.width>self.shadow_size).then_some(self.timestamps)).map_err(named)?;
        if target.extent.width == self.shadow_size {
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
    fn copy_uniforms(&self,cmd:vk::CommandBuffer,offset:u64) {
        use vk::{PipelineStageFlags as S,AccessFlags as A};
        unsafe {
            let d=&self.state.device;
            let before=vk::MemoryBarrier::default().src_access_mask(A::UNIFORM_READ).dst_access_mask(A::TRANSFER_WRITE);
            d.cmd_pipeline_barrier(cmd,S::ALL_GRAPHICS|S::COMPUTE_SHADER,S::TRANSFER,Default::default(),&[before],&[],&[]);
            d.cmd_copy_buffer(cmd,self.frame_uniforms.buffer,self.uniforms.buffer,&[vk::BufferCopy::default().src_offset(offset).size(256)]);
            let after=vk::MemoryBarrier::default().src_access_mask(A::TRANSFER_WRITE).dst_access_mask(A::UNIFORM_READ);
            d.cmd_pipeline_barrier(cmd,S::TRANSFER,S::ALL_GRAPHICS|S::COMPUTE_SHADER,Default::default(),&[after],&[],&[]);
        }
    }
    pub(super) fn query_pool(&self)->Option<vk::QueryPool> { PROFILE.load(Ordering::Relaxed).then_some(self.timestamps) }
    pub(super) fn report(&self, pass: &str) -> Result<(), String> {
        self.report_at(pass,0)
    }
    fn report_at(&self, pass:&str, offset:u32)->Result<(),String> {
        if PROFILE.load(Ordering::Relaxed) {
            let mut ticks = [0u64; 2];
            unsafe {
                self.state.device.get_query_pool_results(
                    self.timestamps,
                    offset,
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
        fade: f32,
        waves: &HostBuffer,
    ) -> Result<(), String> {
        if !self.rays && !self.shadows_ready {self.bake_shadows(world)?;}
        let grid = self.grid;
        if let Some(path) = &self.water_dump {
            std::fs::create_dir_all(path).map_err(named)?;
            let bytes=waves.view(0..waves.size).map_err(named)?;
            let heights:Vec<_>=bytes.chunks_exact(16).map(|p|f32::from_le_bytes(p[..4].try_into().unwrap())).collect();
            let lo = heights.iter().copied().fold(f32::INFINITY, f32::min);
            let hi = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let image: Vec<_> = heights.iter().map(|h| ((h-lo)/(hi-lo).max(1e-8)*255.) as u8).collect();
            std::fs::write(path.join("heightfield.pgm"), [format!("P5\n{} {}\n255\n",grid[0],grid[1]).as_bytes(), &image].concat()).map_err(named)?;
            std::fs::write(path.join("surface.f32"), bytes).map_err(named)?;
            eprintln!("water_check,height_range,{lo},{hi},reflection,{}", if self.rays {"TLAS binding14"} else {"planar binding11"});
        }
        let light_data: Vec<_> = world.lights.iter().enumerate().flat_map(|(i,l)| {
            let phase=i as f32*2.3;
            let flicker=1.+l.flicker*(0.21*(seconds*11.7+phase).sin()+0.11*(seconds*23.1+phase*1.7).sin()+0.08*(seconds*4.3+phase).sin());
            [l.pos[0],l.pos[1],l.pos[2],l.flame,l.colour[0]*flicker,l.colour[1]*flicker,l.colour[2]*flicker,self.shadow_slots[i].map_or(0.,|slot|slot as f32+1.)]
        }).collect();
        self.lights.write(&floats(&light_data)).map_err(named)?;
        let per_emitter = if self.showcase { 96 } else { 24 };
        self.particle_time.write(&floats(&[seconds, per_emitter as f32, 0., 0.])).map_err(named)?;

        let mut extras = Vec::new();
        let mut particle = |p: V3, kind: f32, size: [f32; 2], seed: f32, colour: [f32; 4]| {
            extras.extend([
                p[0], p[1], p[2], kind, size[0], size[1], seed, 0., colour[0], colour[1], colour[2], colour[3],
            ]);
        };
        for (i, (tip, floor)) in world.drips.iter().copied().chain(Self::hanging_drips(world)).enumerate() {
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
        for i in 0..152 {
            let seed = i as f32;
            if i < 96 {
                let lamp = &world.lights[i % world.lights.len()];
                let p = add(
                    lamp.pos,
                    [rng.range(-2., 2.), (rng.next() * 3. + seconds * 0.07).rem_euclid(3.) - 1., rng.range(-2., 2.)],
                );
                particle(p, 3., [0.008; 2], seed, [0.65, 0.55, 0.37, 0.75]);
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
        for i in 0..24 {
            let h=(seconds*0.08+i as f32*0.17).rem_euclid(world.grate.hi[1]);
            let p=add(world.grate.centre(),[rng.range(-0.65,0.65)+0.2*h,-h,rng.range(-0.65,0.65)-0.15*h]);
            particle(p,3.,[0.009;2],i as f32,[0.7,1.,1.5,0.7]);
        }
        for prop in &world.props {
            match *prop {
                Prop::Web(p, signs) => particle(add(p, [signs[0]*0.32, -0.38, signs[1]*0.32]), 8., [0.45, 0.4], signs[0].atan2(-signs[1]), [0.38, 0.4, 0.37, 0.3]),
                Prop::Lantern(p, yaw) => {
                    particle(add(p, turn(yaw, [0.235, 0.2, 0.])), 9., [0.16, 0.11], yaw, [3., 0.65, 0.015, 1.])
                }
                _ => {}
            }
        }
        for i in 0..28 {
            let seed=i as f32*2.39996;
            let x=world.river.lo[0]+(i as f32+0.5)/28.*(world.river.hi[0]-world.river.lo[0])+0.45*(seconds*0.12+seed).sin();
            let z=world.river.centre()[2]+1.1*(seed+seconds*0.06).sin();
            let h=0.28+0.12*(seed*1.7).sin();
            particle([x,WATER+h*0.65,z],11.,[1.2+0.4*seed.sin(),h],seed,[0.25,0.32,0.38,0.25]);
        }
        for i in 0..8 {
            let seed=i as f32*2.39996;let age=(seconds*0.16+i as f32*0.127).fract();
            particle([world.bridge.centre()[0]+0.5*seed.sin(),WATER+0.2+age*0.9,world.river.centre()[2]+seed.cos()],11.,[0.35+age*0.35,0.3+age*0.25],seed,[0.28,0.34,0.4,0.3*(PI*age).sin()]);
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
        let visible=|bounds:&Box3,matrix:Matrix| {
            let corners:[[f32;4];8]=std::array::from_fn(|i|{
                let p=[if i&1==0{bounds.lo[0]}else{bounds.hi[0]},if i&2==0{bounds.lo[1]}else{bounds.hi[1]},if i&4==0{bounds.lo[2]}else{bounds.hi[2]},1.];
                std::array::from_fn(|r|(0..4).map(|c|matrix[c][r]*p[c]).sum())
            });
            !(0..6).any(|plane|corners.iter().all(|p|match plane{0=>p[0]< -p[3],1=>p[0]>p[3],2=>p[1]< -p[3],3=>p[1]>p[3],4=>p[2]<0.,_=>p[2]>p[3]}))
        };
        let cave=world.zones[world.cave].space;
        let formations=Box3{lo:sub(cave.lo,[0.5;3]),hi:add(cave.hi,[0.5;3])};
        self.cone_visible=visible(&formations,vp)||(!self.rays&&visible(&formations,reflected));
        let zones:Vec<_>=self.batches.iter().map(|(bounds,_)|visible(bounds,vp)||(!self.rays&&visible(bounds,reflected))).collect();
        if zones!=self.visible_zones {
            let indices:Vec<_>=self.batches.iter().zip(&zones).filter(|(_,visible)|**visible).flat_map(|((_,indices),_)|indices.iter().copied()).collect();
            self.visible_count=indices.len() as u32;
            self.indices.write_at(0,&indices.iter().flat_map(|i|i.to_le_bytes()).collect::<Vec<_>>()).map_err(named)?;
            self.visible_zones=zones;
        }
        self.uniforms(reflected,reflected,reflected_eye,seconds,right,up,fade,world,true,plane,Some(0))?;
        self.uniforms(vp,reflected,eye,seconds,right,up,fade,world,false,plane,Some(256))?;
        let key=(self.visible_count,self.cone_visible,DISABLED.load(Ordering::Relaxed),PROFILE.load(Ordering::Relaxed));
        if self.recorded!=Some(key) {
            let start=std::time::Instant::now();
            let cmd=self.command;
            unsafe {
                self.state.device.reset_command_buffer(cmd,Default::default()).map_err(named)?;
                self.state.device.begin_command_buffer(cmd,&vk::CommandBufferBeginInfo::default()).map_err(named)?;
            }
            let stamp=|offset| PROFILE.load(Ordering::Relaxed).then_some((self.timestamps,offset));
            self.state.record_dispatch(cmd,&self.particles_step,[(self.emitter_count*per_emitter).div_ceil(64) as u32,1,1],stamp(2));
            if !self.rays {
                self.copy_uniforms(cmd,0);
                self.draw_commands(Some((cmd,4)),&self.reflection,0,self.visible_count,true,false)?;
                self.draw_commands(Some((cmd,6)),&self.reflection,u32::MAX,24*12*6,false,false)?;
                self.draw_commands(Some((cmd,8)),&self.reflection,0,6,false,true)?;
                self.reflection.record_readback(cmd);
            }
            self.copy_uniforms(cmd,256);
            self.draw_commands(Some((cmd,10)),&self.target,0,self.visible_count,true,false)?;
            self.draw_commands(Some((cmd,12)),&self.target,u32::MAX,24*12*6,false,false)?;
            self.target.record_readback(cmd);
            if let Some(buffer)=&self.refraction_dump {unsafe {
                let barrier=vk::MemoryBarrier::default().src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::TRANSFER_READ);
                self.state.device.cmd_pipeline_barrier(cmd,vk::PipelineStageFlags::TRANSFER,vk::PipelineStageFlags::TRANSFER,Default::default(),&[barrier],&[],&[]);
                self.state.device.cmd_copy_buffer(cmd,self.target.staging.buffer,buffer.buffer,&[vk::BufferCopy::default().size(buffer.size)]);
                let host=vk::MemoryBarrier::default().src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::HOST_READ);
                self.state.device.cmd_pipeline_barrier(cmd,vk::PipelineStageFlags::TRANSFER,vk::PipelineStageFlags::HOST,Default::default(),&[host],&[],&[]);
            }}
            self.draw_commands(Some((cmd,14)),&self.target,self.count,self.water_indices,false,false)?;
            self.draw_commands(Some((cmd,16)),&self.target,0,6,false,true)?;
            self.target.record_readback(cmd);
            self.state.record_dispatch(cmd,&self.fog,[(WIDTH/4).div_ceil(8),(HEIGHT/4).div_ceil(8),1],stamp(18));
            self.state.record_dispatch(cmd,&self.post,[WIDTH.div_ceil(8),HEIGHT.div_ceil(8),1],stamp(20));
            unsafe {self.state.device.end_command_buffer(cmd)}.map_err(named)?;
            self.recorded=Some(key);
            if PROFILE.load(Ordering::Relaxed) {eprintln!("dungeon_host,record_cached,{:.4}",start.elapsed().as_secs_f64()*1000.);}
        }
        self.state.submit_recorded(self.command).map_err(named)?;
        for (offset,name) in [(2,"particles"),(4,"reflection"),(6,"cones"),(8,"sprites"),(10,"opaque"),(12,"cones"),(14,"water"),(16,"sprites"),(18,"fog"),(20,"post")] {
            if (self.rays && (4..=8).contains(&offset)) || (!self.cone_visible && matches!(offset,6|12)) || (offset==14 && DISABLED.load(Ordering::Relaxed)&2!=0) {continue;}
            self.report_at(name,offset)?;
        }
        if let Some(path)=&self.water_dump {
            if !self.rays {std::fs::write(path.join("reflection.rgba16f"),self.reflection.staging.view(0..self.reflection.staging.size).map_err(named)?).map_err(named)?;}
            if let Some(buffer)=&self.refraction_dump {std::fs::write(path.join("refraction.rgba16f"),buffer.view(0..buffer.size).map_err(named)?).map_err(named)?;}
        }
        crate::display::present_buffer(&self.output, WIDTH, HEIGHT).map_err(named)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.state.device.destroy_command_pool(self.command_pool,None);
            self.state.device.destroy_query_pool(self.timestamps, None);
        }
    }
}
