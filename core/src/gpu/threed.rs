pub const CLASS_ID: u16 = 0xc797;

const REGISTER_BYTES: usize = 0x3c00;
const SET_VERTEX_STREAM_SIZE_A: usize = 0x0600;
const SET_VERTEX_STREAM_SIZE_B: usize = 0x0604;
const SET_COLOR_TARGET_A: usize = 0x0800;
const SET_COLOR_TARGET_B: usize = 0x0804;
const SET_COLOR_TARGET_WIDTH: usize = 0x0808;
const SET_COLOR_TARGET_HEIGHT: usize = 0x080c;
const SET_COLOR_TARGET_FORMAT: usize = 0x0810;
const SET_COLOR_TARGET_MEMORY: usize = 0x0814;
const SET_VIEWPORT_SCALE_X: usize = 0x0a00;
const SET_VIEWPORT_SCALE_Y: usize = 0x0a04;
const SET_VIEWPORT_SCALE_Z: usize = 0x0a08;
const SET_VIEWPORT_OFFSET_X: usize = 0x0a0c;
const SET_VIEWPORT_OFFSET_Y: usize = 0x0a10;
const SET_VIEWPORT_OFFSET_Z: usize = 0x0a14;
const SET_VIEWPORT_CLIP_HORIZONTAL: usize = 0x0c00;
const SET_VIEWPORT_CLIP_VERTICAL: usize = 0x0c04;
const SET_VIEWPORT_CLIP_MIN_Z: usize = 0x0c08;
const SET_VIEWPORT_CLIP_MAX_Z: usize = 0x0c0c;
const SET_VERTEX_ARRAY_START: usize = 0x0d74;
const DRAW_VERTEX_ARRAY: usize = 0x0d78;
const SET_SCISSOR_ENABLE: usize = 0x0e00;
const SET_SCISSOR_HORIZONTAL: usize = 0x0e04;
const SET_SCISSOR_VERTICAL: usize = 0x0e08;
const SET_VERTEX_ATTRIBUTE_A: usize = 0x1160;
const SET_VERTEX_ATTRIBUTE_B: usize = 0x11a0;
const DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST: usize = 0x1214;
const DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT: usize = 0x1218;
const SET_DEPTH_TEST: usize = 0x12cc;
const SET_BLEND_STATE_PER_TARGET: usize = 0x12e4;
const SET_BLEND: usize = 0x1360;
const SET_INDEX_BUFFER_A: usize = 0x17c8;
const SET_INDEX_BUFFER_B: usize = 0x17cc;
const SET_INDEX_BUFFER_E: usize = 0x17d8;
const SET_INDEX_BUFFER_F: usize = 0x17dc;
const DRAW_INDEX_BUFFER: usize = 0x17e0;
const DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_FIRST: usize = 0x17e4;
const DRAW_INDEX_BUFFER16_BEGIN_END_INSTANCE_FIRST: usize = 0x17e8;
const DRAW_INDEX_BUFFER8_BEGIN_END_INSTANCE_FIRST: usize = 0x17ec;
const DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_SUBSEQUENT: usize = 0x17f0;
const DRAW_INDEX_BUFFER16_BEGIN_END_INSTANCE_SUBSEQUENT: usize = 0x17f4;
const DRAW_INDEX_BUFFER8_BEGIN_END_INSTANCE_SUBSEQUENT: usize = 0x17f8;
const CLEAR_SURFACE: usize = 0x19d0;
const SET_CT_WRITE: usize = 0x1a00;
const SET_REPORT_SEMAPHORE_A: usize = 0x1b00;
const SET_REPORT_SEMAPHORE_B: usize = 0x1b04;
const SET_REPORT_SEMAPHORE_C: usize = 0x1b08;
const SET_REPORT_SEMAPHORE_D: usize = 0x1b0c;
const SET_VERTEX_STREAM_A_FORMAT: usize = 0x1c00;
const SET_VERTEX_STREAM_A_LOCATION_A: usize = 0x1c04;
const SET_VERTEX_STREAM_A_LOCATION_B: usize = 0x1c08;
const SET_PIPELINE_SHADER: usize = 0x2000;
const SET_PIPELINE_PROGRAM_ADDRESS_A: usize = 0x2014;
const SET_PIPELINE_PROGRAM_ADDRESS_B: usize = 0x2018;
const SET_CONSTANT_BUFFER_SELECTOR_A: usize = 0x2380;
const SET_CONSTANT_BUFFER_SELECTOR_B: usize = 0x2384;
const SET_CONSTANT_BUFFER_SELECTOR_C: usize = 0x2388;
const LOAD_CONSTANT_BUFFER_OFFSET: usize = 0x238c;
const LOAD_CONSTANT_BUFFER: usize = 0x2390;

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
    pub first: u32,
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
    pub instance_count: u32,
    pub indexed: bool,
    pub vertex_streams: [VertexStream; 32],
    pub vertex_attributes: [VertexAttribute; 32],
    pub shader_programs: [ShaderProgram; 6],
    pub colour_target: ColourTarget,
    pub viewport: Viewport,
    pub scissor: Scissor,
    pub colour_write_mask: u8,
    pub depth_test_enabled: bool,
    pub blend_per_target: bool,
    pub blend_enabled: bool,
    pub index_buffer: IndexBuffer,
}

pub(crate) enum ThreeDEvent {
    Draw(DrawSnapshot),
    Report { addr: u64, payload: u32 },
    Clear { value: u32 },
    LoadConstantBuffer { addr: u64, value: u32 },
}

pub struct ThreeD {
    words: Box<[u32; REGISTER_BYTES / 4]>,
    dirty: DirtyFlags,
    topology: PrimitiveTopology,
    instances: u32,
}

impl Default for ThreeD {
    fn default() -> Self {
        Self {
            words: Box::new([0; REGISTER_BYTES / 4]),
            dirty: DirtyFlags::default(),
            topology: PrimitiveTopology::Points,
            instances: 1,
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
            DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST => {
                self.instances = 1;
                Ok(Some(ThreeDEvent::Draw(self.encoded_draw(value, false)?)))
            }
            DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT => {
                self.instances += 1;
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
                self.instances = 1;
                self.set_index_size(byte);
                Ok(Some(ThreeDEvent::Draw(self.encoded_draw(value, true)?)))
            }
            DRAW_INDEX_BUFFER32_BEGIN_END_INSTANCE_SUBSEQUENT
            | DRAW_INDEX_BUFFER16_BEGIN_END_INSTANCE_SUBSEQUENT
            | DRAW_INDEX_BUFFER8_BEGIN_END_INSTANCE_SUBSEQUENT => {
                self.instances += 1;
                self.set_index_size(byte);
                Ok(Some(ThreeDEvent::Draw(self.encoded_draw(value, true)?)))
            }
            SET_REPORT_SEMAPHORE_D => Ok(Some(ThreeDEvent::Report {
                addr: self.address_pair(SET_REPORT_SEMAPHORE_A, SET_REPORT_SEMAPHORE_B),
                payload: self.word(SET_REPORT_SEMAPHORE_C),
            })),
            CLEAR_SURFACE => Ok(Some(ThreeDEvent::Clear { value })),
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
        let colour_memory = self.word(SET_COLOR_TARGET_MEMORY);
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
            instance_count: self.instances,
            indexed,
            vertex_streams: streams,
            vertex_attributes: attributes,
            shader_programs: shaders,
            colour_target: ColourTarget {
                address: self.address_pair(SET_COLOR_TARGET_A, SET_COLOR_TARGET_B),
                width: self.word(SET_COLOR_TARGET_WIDTH) & 0x0fff_ffff,
                height: self.word(SET_COLOR_TARGET_HEIGHT) & 0x1ffff,
                format: self.word(SET_COLOR_TARGET_FORMAT) as u8,
                layout: if colour_memory & 0x1000 != 0 {
                    MemoryLayout::Pitch
                } else {
                    MemoryLayout::BlockLinear
                },
            },
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
            blend_per_target: self.word(SET_BLEND_STATE_PER_TARGET) & 1 != 0,
            blend_enabled: self.word(SET_BLEND) & 1 != 0,
            index_buffer: IndexBuffer {
                address: self.address_pair(SET_INDEX_BUFFER_A, SET_INDEX_BUFFER_B),
                first: self.word(SET_INDEX_BUFFER_F),
                element_size: index_size,
            },
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
