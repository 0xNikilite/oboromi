use super::compute_class::DescriptorPool;
use scene::threed::*;

const REGISTER_BYTES: usize = 0x3c00;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub vertex_streams: bool,
    pub vertex_attributes: bool,
    pub colour_targets: bool,
    pub viewport_scissor: bool,
    pub shader_programs: bool,
    pub cbuf_binds: bool,
    pub blend_depth: bool,
    pub index_buffer: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VertexStream {
    pub address: u64,
    pub size: u64,
    pub stride: u16,
    pub enabled: bool,
    /// Advances every `frequency` instances instead of every vertex.
    pub instanced: bool,
    pub frequency: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VertexAttribute {
    pub stream: u8,
    pub offset: u16,
    pub size: u8,
    pub kind: u8,
    pub bgra: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShaderProgram {
    pub enabled: bool,
    pub address: u64,
    pub binding_group: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConstantBuffer {
    pub address: u64,
    pub size: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryLayout {
    #[default]
    BlockLinear,
    Pitch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ColourTarget {
    pub address: u64,
    pub width: u32,
    pub height: u32,
    pub format: u8,
    pub layout: MemoryLayout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DepthTarget {
    pub address: u64,
    pub format: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clear {
    pub value: u32,
    pub colour: [f32; 4],
    pub target: ColourTarget,
    pub depth: f32,
    pub depth_target: Option<DepthTarget>,
}

/// The colour and alpha equations, each as the guest's (op, source coefficient, destination coefficient).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Blend {
    pub colour: [u32; 3],
    pub alpha: [u32; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    pub scale: [f32; 3],
    pub offset: [f32; 3],
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub min_depth: f32,
    pub max_depth: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scissor {
    pub enabled: bool,
    pub x_min: u16,
    pub x_max: u16,
    pub y_min: u16,
    pub y_max: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndexBuffer {
    pub address: u64,
    pub element_size: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PrimitiveTopology {
    Points,
    Lines,
    LineLoop,
    LineStrip,
    Triangles,
    TriangleStrip,
    TriangleFan,
    Quads,
    QuadStrip,
    Polygon,
    LineListAdjacency,
    LineStripAdjacency,
    TriangleListAdjacency,
    TriangleStripAdjacency,
    Patch,
}

impl TryFrom<u8> for PrimitiveTopology {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        const VALUES: [PrimitiveTopology; 15] = [
            PrimitiveTopology::Points,
            PrimitiveTopology::Lines,
            PrimitiveTopology::LineLoop,
            PrimitiveTopology::LineStrip,
            PrimitiveTopology::Triangles,
            PrimitiveTopology::TriangleStrip,
            PrimitiveTopology::TriangleFan,
            PrimitiveTopology::Quads,
            PrimitiveTopology::QuadStrip,
            PrimitiveTopology::Polygon,
            PrimitiveTopology::LineListAdjacency,
            PrimitiveTopology::LineStripAdjacency,
            PrimitiveTopology::TriangleListAdjacency,
            PrimitiveTopology::TriangleStripAdjacency,
            PrimitiveTopology::Patch,
        ];
        VALUES
            .get(value as usize)
            .copied()
            .ok_or_else(|| format!("unknown C797 primitive topology {value:#x}"))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawSnapshot {
    pub topology: PrimitiveTopology,
    pub vertex_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
    pub instance_count: u32,
    pub indexed: bool,
    pub vertex_streams: [VertexStream; 32],
    pub vertex_attributes: [VertexAttribute; 32],
    pub shader_programs: [ShaderProgram; 6],
    pub colour_target: ColourTarget,
    pub depth_target: Option<DepthTarget>,
    pub viewport: Viewport,
    pub scissor: Scissor,
    pub colour_write_mask: u8,
    pub depth_test_enabled: bool,
    pub depth_write_enabled: bool,
    pub depth_func: u32,
    /// SET_WINDOW_ORIGIN MODE LOWER_LEFT: window y counts up from the target's last row.
    pub lower_left_origin: bool,
    /// SET_WINDOW_ORIGIN FLIP_Y: triangles face the other way.
    pub flip_y: bool,
    pub cull_enabled: bool,
    pub cull_face: u32,
    pub front_face: u32,
    /// Target 0's blend equations when its blending is on.
    pub blend: Option<Blend>,
    pub blend_constant: [f32; 4],
    pub index_buffer: IndexBuffer,
    /// Per binding group, the buffer bound to each constant buffer slot.
    pub constant_buffers: [[Option<ConstantBuffer>; 18]; 5],
    pub texture_pool: DescriptorPool,
    pub sampler_pool: DescriptorPool,
    /// A texture's sampler is the TSC entry at its TIC index rather than the one its handle names.
    pub sampler_via_header: bool,
}

impl DrawSnapshot {
    /// Whether `next` draws the instance after this draw's last with the same state, so both make one instanced draw.
    pub fn continued_by(&self, next: &DrawSnapshot) -> bool {
        next.first_instance == self.first_instance + self.instance_count
            && *self == DrawSnapshot { first_instance: self.first_instance, instance_count: self.instance_count, ..next.clone() }
    }
}

pub(crate) enum ThreeDEvent {
    Draw(DrawSnapshot),
    Report { addr: u64, payload: u32, one_word: bool },
    SemaphoreAcquire { addr: u64, payload: u32, at_least: bool },
    Clear(Clear),
    LoadConstantBuffer { addr: u64, value: u32 },
}

pub struct ThreeD {
    words: Box<[u32; REGISTER_BYTES / 4]>,
    dirty: DirtyFlags,
    topology: PrimitiveTopology,
    instance: u32,
    constant_buffers: [[Option<ConstantBuffer>; 18]; 5],
}

impl Default for ThreeD {
    fn default() -> Self {
        Self {
            words: Box::new([0; REGISTER_BYTES / 4]),
            dirty: DirtyFlags::default(),
            topology: PrimitiveTopology::Points,
            instance: 0,
            constant_buffers: Default::default(),
        }
    }
}

impl ThreeD {
    pub fn dirty(&self) -> DirtyFlags {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = DirtyFlags::default();
    }

    pub fn raw(&self, method: u16) -> u32 {
        self.words[method as usize]
    }

    pub(crate) fn write(&mut self, method: u16, value: u32) -> Result<Option<ThreeDEvent>, String> {
        let byte = method as usize * 4;
        if byte >= REGISTER_BYTES {
            return Err(format!("C797 method {byte:#06x} outside register file"));
        }
        self.words[method as usize] = value;
        self.mark_dirty(byte);
        match byte {
            DRAW_VERTEX_ARRAY => Ok(Some(ThreeDEvent::Draw(self.snapshot(
                self.topology,
                self.word(SET_VERTEX_ARRAY_START),
                value,
                false,
            )?))),
            BEGIN => {
                self.topology = u8::try_from(value & 0xffff).unwrap_or(u8::MAX).try_into()?;
                match value >> 26 & 3 {
                    0 => self.instance = 0,
                    1 => self.instance += 1,
                    _ => {}
                }
                Ok(None)
            }
            DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST => {
                self.instance = 0;
                Ok(Some(ThreeDEvent::Draw(self.encoded_draw(value, false)?)))
            }
            DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT => {
                self.instance += 1;
                Ok(Some(ThreeDEvent::Draw(self.encoded_draw(value, false)?)))
            }
            DRAW_INDEX_BUFFER => Ok(Some(ThreeDEvent::Draw(self.snapshot(
                self.topology,
                self.word(SET_INDEX_BUFFER_F),
                value,
                true,
            )?))),
            DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST
            | DRAW_INDEX_BUFFER16_BEGIN_END_INSTANCE_FIRST
            | DRAW_INDEX_BUFFER8_BEGIN_END_INSTANCE_FIRST => {
                self.instance = 0;
                self.set_index_size(byte);
                Ok(Some(ThreeDEvent::Draw(self.encoded_draw(value, true)?)))
            }
            DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_SUBSEQUENT
            | DRAW_INDEX_BUFFER16_BEGIN_END_INSTANCE_SUBSEQUENT
            | DRAW_INDEX_BUFFER8_BEGIN_END_INSTANCE_SUBSEQUENT => {
                self.instance += 1;
                self.set_index_size(byte);
                Ok(Some(ThreeDEvent::Draw(self.encoded_draw(value, true)?)))
            }
            // OPERATION RELEASE or ACQUIRE in bits 1:0, REDUCTION_ENABLE 3, COMPARISON GE 16, STRUCTURE_SIZE ONE_WORD 28.
            SET_REPORT_SEMAPHORE_D => {
                let (addr, payload) = (self.address_pair(SET_REPORT_SEMAPHORE_A, SET_REPORT_SEMAPHORE_B), self.word(SET_REPORT_SEMAPHORE_C));
                match (value & 3, value >> 3 & 1) {
                    (0, 0) => Ok(Some(ThreeDEvent::Report { addr, payload, one_word: value >> 28 & 1 != 0 })),
                    (1, 0) => Ok(Some(ThreeDEvent::SemaphoreAcquire { addr, payload, at_least: value >> 16 & 1 != 0 })),
                    _ => Err(format!("C797 report semaphore {value:#x} is not implemented, only a release or an acquire")),
                }
            }
            CLEAR_SURFACE => Ok(Some(ThreeDEvent::Clear(Clear {
                value,
                colour: [0, 4, 8, 12].map(|i| f32::from_bits(self.word(SET_COLOR_CLEAR_VALUE + i))),
                target: self.colour_target(),
                depth: f32::from_bits(self.word(SET_Z_CLEAR_VALUE)),
                depth_target: self.depth_target(),
            }))),
            BIND_GROUP_CONSTANT_BUFFER..=BIND_GROUP_CONSTANT_BUFFER_LAST
                if (byte - BIND_GROUP_CONSTANT_BUFFER) % 32 == 0 =>
            {
                let slot = (value >> 4) & 0x1f;
                let buffer = (value & 1 != 0).then(|| ConstantBuffer {
                    address: self.address_pair(SET_CONSTANT_BUFFER_SELECTOR_B, SET_CONSTANT_BUFFER_SELECTOR_C),
                    size: self.word(SET_CONSTANT_BUFFER_SELECTOR_A) & 0x1ffff,
                });
                *self.constant_buffers[(byte - BIND_GROUP_CONSTANT_BUFFER) / 32]
                    .get_mut(slot as usize)
                    .ok_or_else(|| format!("C797 constant buffer slot {slot} out of range"))? = buffer;
                Ok(None)
            }
            LOAD_CONSTANT_BUFFER => {
                let addr = self.address_pair(
                    SET_CONSTANT_BUFFER_SELECTOR_B,
                    SET_CONSTANT_BUFFER_SELECTOR_C,
                ) + (self.word(LOAD_CONSTANT_BUFFER_OFFSET) & 0xffff) as u64;
                self.words[LOAD_CONSTANT_BUFFER_OFFSET / 4] =
                    (self.word(LOAD_CONSTANT_BUFFER_OFFSET) + 4) & 0xffff;
                Ok(Some(ThreeDEvent::LoadConstantBuffer { addr, value }))
            }
            _ => Ok(None),
        }
    }

    fn encoded_draw(&mut self, value: u32, indexed: bool) -> Result<DrawSnapshot, String> {
        self.topology = ((value >> 28) as u8).try_into()?;
        self.snapshot(
            self.topology,
            value & 0xffff,
            (value >> 16) & 0xfff,
            indexed,
        )
    }

    fn snapshot(
        &self,
        topology: PrimitiveTopology,
        first_vertex: u32,
        vertex_count: u32,
        indexed: bool,
    ) -> Result<DrawSnapshot, String> {
        let mut streams = [VertexStream::default(); 32];
        for (i, stream) in streams.iter_mut().enumerate() {
            let format = self.word(SET_VERTEX_STREAM_A_FORMAT + i * 16);
            *stream = VertexStream {
                address: self.address_pair(
                    SET_VERTEX_STREAM_A_LOCATION_A + i * 16,
                    SET_VERTEX_STREAM_A_LOCATION_B + i * 16,
                ),
                size: self.address_pair(
                    SET_VERTEX_STREAM_SIZE_A + i * 8,
                    SET_VERTEX_STREAM_SIZE_B + i * 8,
                ),
                stride: (format & 0xfff) as u16,
                enabled: format & 0x1000 != 0,
                instanced: self.word(SET_VERTEX_STREAM_INSTANCE_A + i * 4) & 1 != 0,
                frequency: self.word(SET_VERTEX_STREAM_A_FREQUENCY + i * 16),
            };
        }
        let mut attributes = [VertexAttribute::default(); 32];
        for (i, attribute) in attributes.iter_mut().enumerate() {
            let method = if i < 16 {
                SET_VERTEX_ATTRIBUTE_A + i * 4
            } else {
                SET_VERTEX_ATTRIBUTE_B + (i - 16) * 4
            };
            let value = self.word(method);
            *attribute = VertexAttribute {
                stream: (value & 0x1f) as u8,
                offset: ((value >> 7) & 0x3fff) as u16,
                size: ((value >> 21) & 0x3f) as u8,
                kind: ((value >> 27) & 7) as u8,
                bgra: value >> 31 != 0,
            };
        }
        let mut shaders = [ShaderProgram::default(); 6];
        for (i, shader) in shaders.iter_mut().enumerate() {
            let base = SET_PIPELINE_SHADER + i * 64;
            *shader = ShaderProgram {
                enabled: self.word(base) & 1 != 0,
                address: self.address_pair(
                    SET_PIPELINE_PROGRAM_ADDRESS_A + i * 64,
                    SET_PIPELINE_PROGRAM_ADDRESS_B + i * 64,
                ),
                binding_group: (self.word(SET_PIPELINE_BINDING + i * 64) & 7) as u8,
            };
        }
        let viewport_scale = [
            SET_VIEWPORT_SCALE_X,
            SET_VIEWPORT_SCALE_Y,
            SET_VIEWPORT_SCALE_Z,
        ]
        .map(|method| f32::from_bits(self.word(method)));
        let viewport_offset = [
            SET_VIEWPORT_OFFSET_X,
            SET_VIEWPORT_OFFSET_Y,
            SET_VIEWPORT_OFFSET_Z,
        ]
        .map(|method| f32::from_bits(self.word(method)));
        let clip_h = self.word(SET_VIEWPORT_CLIP_HORIZONTAL);
        let clip_v = self.word(SET_VIEWPORT_CLIP_VERTICAL);
        let scissor_h = self.word(SET_SCISSOR_HORIZONTAL);
        let scissor_v = self.word(SET_SCISSOR_VERTICAL);
        let index_size = match self.word(SET_INDEX_BUFFER_E) & 3 {
            0 => 1,
            1 => 2,
            2 => 4,
            v => return Err(format!("unknown C797 index size {v}")),
        };
        let ct_write = self.word(SET_CT_WRITE);
        Ok(DrawSnapshot {
            topology,
            vertex_count,
            first_vertex,
            first_instance: self.instance,
            instance_count: 1,
            indexed,
            vertex_streams: streams,
            vertex_attributes: attributes,
            shader_programs: shaders,
            colour_target: self.colour_target(),
            depth_target: self.depth_target(),
            viewport: Viewport {
                scale: viewport_scale,
                offset: viewport_offset,
                x: clip_h as u16,
                width: (clip_h >> 16) as u16,
                y: clip_v as u16,
                height: (clip_v >> 16) as u16,
                min_depth: f32::from_bits(self.word(SET_VIEWPORT_CLIP_MIN_Z)),
                max_depth: f32::from_bits(self.word(SET_VIEWPORT_CLIP_MAX_Z)),
            },
            scissor: Scissor {
                enabled: self.word(SET_SCISSOR_ENABLE) & 1 != 0,
                x_min: scissor_h as u16,
                x_max: (scissor_h >> 16) as u16,
                y_min: scissor_v as u16,
                y_max: (scissor_v >> 16) as u16,
            },
            colour_write_mask: (ct_write & 1
                | (ct_write >> 3) & 2
                | (ct_write >> 6) & 4
                | (ct_write >> 9) & 8) as u8,
            depth_test_enabled: self.word(SET_DEPTH_TEST) & 1 != 0,
            depth_write_enabled: self.word(SET_DEPTH_WRITE) & 1 != 0,
            depth_func: self.word(SET_DEPTH_FUNC),
            lower_left_origin: self.word(SET_WINDOW_ORIGIN) & 1 != 0,
            flip_y: self.word(SET_WINDOW_ORIGIN) >> 4 & 1 != 0,
            cull_enabled: self.word(OGL_SET_CULL) & 1 != 0,
            cull_face: self.word(OGL_SET_CULL_FACE),
            front_face: self.word(OGL_SET_FRONT_FACE),
            blend: (self.word(SET_BLEND) & 1 != 0).then(|| self.blend()),
            blend_constant: [0, 4, 8, 12].map(|i| f32::from_bits(self.word(SET_BLEND_CONST_RED + i))),
            index_buffer: IndexBuffer {
                address: self.address_pair(SET_INDEX_BUFFER_A, SET_INDEX_BUFFER_B),
                element_size: index_size,
            },
            constant_buffers: self.constant_buffers,
            texture_pool: DescriptorPool {
                address: self.address_pair(SET_TEX_HEADER_POOL_A, SET_TEX_HEADER_POOL_B),
                maximum_index: self.word(SET_TEX_HEADER_POOL_C) & 0x3f_ffff,
            },
            sampler_pool: DescriptorPool {
                address: self.address_pair(SET_TEX_SAMPLER_POOL_A, SET_TEX_SAMPLER_POOL_B),
                maximum_index: self.word(SET_TEX_SAMPLER_POOL_C) & 0xf_ffff,
            },
            sampler_via_header: self.word(SET_SAMPLER_BINDING) & 1 != 0,
        })
    }

    fn colour_target(&self) -> ColourTarget {
        ColourTarget {
            address: self.address_pair(SET_COLOR_TARGET_A, SET_COLOR_TARGET_B),
            width: self.word(SET_COLOR_TARGET_WIDTH) & 0x0fff_ffff,
            height: self.word(SET_COLOR_TARGET_HEIGHT) & 0x1ffff,
            format: self.word(SET_COLOR_TARGET_FORMAT) as u8,
            layout: if self.word(SET_COLOR_TARGET_MEMORY) & 0x1000 != 0 {
                MemoryLayout::Pitch
            } else {
                MemoryLayout::BlockLinear
            },
        }
    }

    fn blend(&self) -> Blend {
        let s = SET_BLEND_COLOR_OP;
        // SET_GLOBAL_COLOR_KEY sits between the shared alpha source and destination coefficients (clc797.h).
        let [separate, colour @ .., alpha_op, alpha_source, alpha_dest] = match self.word(SET_BLEND_STATE_PER_TARGET) & 1 {
            0 => [SET_BLEND_SEPARATE_FOR_ALPHA, s, s + 4, s + 8, s + 12, s + 16, SET_BLEND_ALPHA_DEST_COEFF],
            _ => [0, 4, 8, 12, 16, 20, 24].map(|i| SET_BLEND_PER_TARGET_SEPARATE_FOR_ALPHA + i),
        }
        .map(|method| self.word(method));
        Blend { colour, alpha: if separate & 1 != 0 { [alpha_op, alpha_source, alpha_dest] } else { colour } }
    }

    fn depth_target(&self) -> Option<DepthTarget> {
        (self.word(SET_ZT_SELECT) & 1 != 0).then(|| DepthTarget {
            address: self.address_pair(SET_ZT_A, SET_ZT_B),
            format: (self.word(SET_ZT_FORMAT) & 0x1f) as u8,
        })
    }

    fn word(&self, byte: usize) -> u32 {
        self.words[byte / 4]
    }

    fn address_pair(&self, upper: usize, lower: usize) -> u64 {
        ((self.word(upper) & 0xff) as u64) << 32 | self.word(lower) as u64
    }

    fn set_index_size(&mut self, method: usize) {
        self.words[SET_INDEX_BUFFER_E / 4] = match method {
            DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST
            | DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_SUBSEQUENT => 2,
            DRAW_INDEX_BUFFER16_BEGIN_END_INSTANCE_FIRST
            | DRAW_INDEX_BUFFER16_BEGIN_END_INSTANCE_SUBSEQUENT => 1,
            _ => 0,
        };
    }

    fn mark_dirty(&mut self, byte: usize) {
        match byte {
            0x0600..=0x06fc | 0x1c00..=0x1efc => self.dirty.vertex_streams = true,
            0x1160..=0x11dc => self.dirty.vertex_attributes = true,
            0x0800..=0x09fc | SET_CT_WRITE..=0x1a1c => self.dirty.colour_targets = true,
            0x0a00..=0x0bfc | 0x0c00..=0x0cfc | 0x0e00..=0x0efc => {
                self.dirty.viewport_scissor = true
            }
            0x2000..=0x217c => self.dirty.shader_programs = true,
            SET_CONSTANT_BUFFER_SELECTOR_A..=0x25ec => self.dirty.cbuf_binds = true,
            0x12cc..=0x139c => self.dirty.blend_depth = true,
            0x17c8..=0x17dc => self.dirty.index_buffer = true,
            _ => {}
        }
    }
}
