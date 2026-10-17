use super::*;

const DIM_2D: u32 = 1;
const LOD_OPERAND: u32 = 2;
const GRAD_OPERAND: u32 = 4;

#[repr(u32)]
pub enum TextureType {
    ONE_D = 0,
    TWO_D = 1,
    THREE_D = 2,
    CUBEMAP = 3,
    ONE_D_ARRAY = 4,
    TWO_D_ARRAY = 5,
    ONE_D_BUFFER = 6,
    TWO_D_NO_MIPMAP = 7,
    CUBE_ARRAY = 8,
}

#[repr(u32)]
pub enum TicSource {
    ZERO = 0,
    R = 2,
    G = 3,
    B = 4,
    A = 5,
    ONE_INT = 6,
    ONE_FLOAT = 7,
}

#[repr(u32)]
pub enum TicType {
    SNORM = 1,
    UNORM = 2,
    SINT = 3,
    UINT = 4,
    SNORM_FORCE_FP16 = 5,
    UNORM_FORCE_FP16 = 6,
    FLOAT = 7,
}

#[repr(u32)]
pub enum TicComponentSize {
    R32_G32_B32_A32 = 0x01,
    R32_G32_B32 = 0x02,
    R16_G16_B16_A16 = 0x03,
    R32_G32 = 0x04,
    R32_B24G8 = 0x05,
    X8B8G8R8 = 0x07,
    A8B8G8R8 = 0x08,
    A2B10G10R10 = 0x09,
    R16_G16 = 0x0c,
    G8R24 = 0x0d,
    G24R8 = 0x0e,
    R32 = 0x0f,
    A4B4G4R4 = 0x12,
    A5B5G5R1 = 0x13,
    A1B5G5R5 = 0x14,
    B5G6R5 = 0x15,
    B6G5R5 = 0x16,
    G8R8 = 0x18,
    R16 = 0x1b,
    Y8_VIDEO = 0x1c,
    R8 = 0x1d,
    G4R4 = 0x1e,
    R1 = 0x1f,
    E5B9G9R9_SHAREDEXP = 0x20,
    BF10GF11RF11 = 0x21,
    G8B8G8R8 = 0x22,
    B8G8R8G8 = 0x23,
    DXT1 = 0x24,
    DXT23 = 0x25,
    DXT45 = 0x26,
    DXN1 = 0x27,
    DXN2 = 0x28,
    BC6H_SF16 = 0x10,
    BC6H_UF16 = 0x11,
    BC7U = 0x17,
    Z24S8 = 0x29,
    X8Z24 = 0x2a,
    S8Z24 = 0x2b,
    X4V4Z24__COV4R4V = 0x2c,
    X4V4Z24__COV8R8V = 0x2d,
    V8Z24__COV4R12V = 0x2e,
    ZF32 = 0x2f,
    ZF32_X24S8 = 0x30,
    X8Z24_X20V4S8__COV4R4V = 0x31,
    X8Z24_X20V4S8__COV8R8V = 0x32,
    ZF32_X20V4X8__COV4R4V = 0x33,
    ZF32_X20V4X8__COV8R8V = 0x34,
    ZF32_X20V4S8__COV4R4V = 0x35,
    ZF32_X20V4S8__COV8R8V = 0x36,
    X8Z24_X16V8S8__COV4R12V = 0x37,
    ZF32_X16V8X8__COV4R12V = 0x38,
    ZF32_X16V8S8__COV4R12V = 0x39,
    Z16 = 0x3a,
    V8Z24__COV8R24V = 0x3b,
    X8Z24_X16V8S8__COV8R24V = 0x3c,
    ZF32_X16V8X8__COV8R24V = 0x3d,
    ZF32_X16V8S8__COV8R24V = 0x3e,
    CS_BITFIELD_SIZE = 0x3f
}

pub struct TicHeaderV1 {}

#[repr(u32)]
pub enum MaxAnisotropy {
    _1_TO_1 = 0x0,
    _2_TO_1 = 0x1,
    _4_TO_1 = 0x2,
    _6_TO_1 = 0x3,
    _8_TO_1 = 0x4,
    _10_TO_1 = 0x5,
    _12_TO_1 = 0x6,
    _16_TO_1 = 0x7,
}

#[repr(u32)]
pub enum SurfaceFormat {
    BITMAP = 0x001c,
    UNK1D = 0x001d,
    RGBA32_FLOAT = 0x00c0,
    RGBA32_SINT = 0x00c1,
    RGBA32_UINT = 0x00c2,
    RGBX32_FLOAT = 0x00c3,
    RGBX32_SINT = 0x00c4,
    RGBX32_UINT = 0x00c5,
    RGBA16_UNORM = 0x00c6,
    RGBA16_SNORM = 0x00c7,
    RGBA16_SINT = 0x00c8,
    RGBA16_UINT = 0x00c9,
    RGBA16_FLOAT = 0x00ca,
    RG32_FLOAT = 0x00cb,
    RG32_SINT = 0x00cc,
    RG32_UINT = 0x00cd,
    RGBX16_FLOAT = 0x00ce,
    BGRA8_UNORM = 0x00cf,
    BGRA8_SRGB = 0x00d0,
    RGB10_A2_UNORM = 0x00d1,
    RGB10_A2_UINT = 0x00d2,
    RGBA8_UNORM = 0x00d5,
    RGBA8_SRGB = 0x00d6,
    RGBA8_SNORM = 0x00d7,
    RGBA8_SINT = 0x00d8,
    RGBA8_UINT = 0x00d9,
    RG16_UNORM = 0x00da,
    RG16_SNORM = 0x00db,
    RG16_SINT = 0x00dc,
    RG16_UINT = 0x00dd,
    RG16_FLOAT = 0x00de,
    BGR10_A2_UNORM = 0x00df,
    R11G11B10_FLOAT = 0x00e0,
    R32_SINT = 0x00e3,
    R32_UINT = 0x00e4,
    R32_FLOAT = 0x00e5,
    BGRX8_UNORM = 0x00e6,
    BGRX8_SRGB = 0x00e7,
    B5G6R5_UNORM = 0x00e8,
    BGR5_A1_UNORM = 0x00e9,
    RG8_UNORM = 0x00ea,
    RG8_SNORM = 0x00eb,
    RG8_SINT = 0x00ec,
    RG8_UINT = 0x00ed,
    R16_UNORM = 0x00ee,
    R16_SNORM = 0x00ef,
    R16_SINT = 0x00f0,
    R16_UINT = 0x00f1,
    R16_FLOAT = 0x00f2,
    R8_UNORM = 0x00f3,
    R8_SNORM = 0x00f4,
    R8_SINT = 0x00f5,
    R8_UINT = 0x00f6,
    A8_UNORM = 0x00f7,
    BGR5_X1_UNORM = 0x00f8,
    RGBX8_UNORM = 0x00f9,
    RGBX8_SRGB = 0x00fa,
    BGR5_X1_UNORM_UNKFB = 0x00fb,
    BGR5_X1_UNORM_UNKFC = 0x00fc,
    BGRX8_UNORM_UNKFD = 0x00fd,
    BGRX8_UNORM_UNKFE = 0x00fe,
    Y32_UINT_UNKFF = 0x00ff,
}

pub enum ZetaFormat {
    Z32_FLOAT = 0x000a,
    Z16_UNORM = 0x0013,
    S8_Z24_UNORM = 0x0014,
    Z24_X8_UNORM = 0x0015,
    Z24_S8_UNORM = 0x0016,
    Z24_C8_UNORM = 0x0018,
    Z32_S8_X24_FLOAT = 0x0019,
    Z24_X8_S8_C8_X16_UNORM = 0x001d,
    Z32_X8_C8_X16_FLOAT = 0x001e,
    Z32_S8_C8_X16_FLOAT = 0x001f,
}

// SUTP
pub enum ImageFormat {
    RGBA32_FLOAT = 0x02,
    RGBA32_SINT = 0x03,
    RGBA32_UINT = 0x04,
    RGBA16_UNORM = 0x08,
    RGBA16_SNORM = 0x09,
    RGBA16_SINT = 0x0a,
    RGBA16_UINT = 0x0b,
    RGBA16_FLOAT = 0x0c,
    RG32_FLOAT = 0x0d,
    RG32_SINT = 0x0e,
    RG32_UINT = 0x0f,
    BGRA8_UNORM = 0x11,
    RGB10_A2_UNORM = 0x13,
    RGB10_A2_UINT = 0x15,
    RGBA8_UNORM = 0x18,
    RGBA8_SNORM = 0x1a,
    RGBA8_SINT = 0x1b,
    RGBA8_UINT = 0x1c,
    RG16_UNORM = 0x1d,
    RG16_SNORM = 0x1e,
    RG16_SINT = 0x1f,
    RG16_UINT = 0x20,
    RG16_FLOAT = 0x21,
    R11G11B10_FLOAT = 0x24,
    R32_SINT = 0x27,
    R32_UINT = 0x28,
    R32_FLOAT = 0x29,
    RG8_UNORM = 0x2e,
    RG8_SNORM = 0x2f,
    RG8_SINT = 0x30,
    RG8_UINT = 0x31,
    R16_UNORM = 0x32,
    R16_SNORM = 0x33,
    R16_SINT = 0x34,
    R16_UINT = 0x35,
    R16_FLOAT = 0x36,
    R8_UNORM = 0x37,
    R8_SNORM = 0x38,
    R8_SINT = 0x39,
    R8_UINT = 0x3a,
}

fn bound_handle(inst: u128) -> (u32, u32) {
    ((inst >> 54 & 0x1f) as u32, (inst >> 40 & 0x3fff) as u32 * 4)
}

impl<'a> Decoder<'a> {
    pub fn footprint(&mut self, inst: u128) {
        todo!("FOOTPRINT reports the sparse tiles a sample would touch in NVIDIA's tile layout; only VK_NV_shader_image_footprint can express it, and neither Intel nor lavapipe has it: {inst:#034x}");
    }
    pub fn suatom(&mut self, inst: u128) {
        let op = (opcode(inst) != 0x595).then_some((inst >> 87 & 0xf) as u32);
        let old = self.surface_atomic(inst, op);
        self.store_register((inst >> 16 & 0xff) as u32, old);
    }
    pub fn suld(&mut self, inst: u128) {
        assert!(inst >> 81 & 7 == PT as u128, "SULD sparse residency predicate");
        let (image, coord, raw) = self.surface_2d(inst, [0xb99, 0xb97], "SULD");
        let texel = self.ir.emit_image_read(self.type_f32[4], image, coord, &[]);
        let rd = (inst >> 16 & 0xff) as u32;
        if raw {
            assert!(inst >> 73 & 7 == 4, "SULD.D size {} is not 32", inst >> 73 & 7);
            // The host binds only RGBA8 surfaces, so a raw 32-bit word is the packed texel.
            let bits = self.glsl(spirv::glsl::PACK_UNORM_4X8, self.type_u32[1], &[texel]);
            self.store_register(rd, bits);
            return;
        }
        let bits = self.ir.emit_bitcast(self.type_u32[4], texel);
        let components = match inst >> 72 & 0xf {
            1 => 1,
            3 => 2,
            15 => 4,
            mask => panic!("SULD.P component mask {mask:#x}"),
        };
        for c in 0..components {
            let value = self.ir.emit_composite_extract(self.type_u32[1], bits, &[c]);
            self.store_register(nth_reg(rd, RZ, c), value);
        }
    }

    // The bound 2D surface, the texel coordinate in Ra, Ra+1, and whether `inst` is the .D rather than the .P opcode.
    fn surface_2d(&mut self, inst: u128, [d_form, p_form]: [u32; 2], name: &str) -> (u32, u32, bool) {
        let raw = match opcode(inst) {
            op if op == d_form => true,
            op if op == p_form => false,
            _ => panic!("{name} only in the bound cbuf form: {inst:#034x}"),
        };
        let (var, ty, coord) = self.surface(inst, vk::Format::R8G8B8A8_UNORM, raw && inst >> 72 & 1 != 0);
        (self.ir.emit_load(ty, var), coord, raw)
    }

    // The bound 2D surface variable holding `format` texels, its image type, and the texel coordinate in Ra, Ra+1,
    // where a byte-addressed x counts the four bytes of each texel.
    fn surface(&mut self, inst: u128, format: vk::Format, byte_addressed: bool) -> (u32, u32, u32) {
        const STORAGE: u32 = 2;
        const RGBA8: u32 = 4;
        const R32UI: u32 = 33;
        assert!(inst >> 61 & 7 == 3, "surface dimension {} is not 2D: {inst:#034x}", inst >> 61 & 7);
        let (slot, sampled, image_format) = match format {
            vk::Format::R32_UINT => (1, self.type_u32[1], R32UI),
            _ => (0, self.type_f32[1], RGBA8),
        };
        if self.type_surface_2d[slot] == 0 {
            self.type_surface_2d[slot] = self.ir.emit_type_image(sampled, DIM_2D, 0, 0, 0, STORAGE, image_format);
        }
        let ty = self.type_surface_2d[slot];
        let (cbuf_bank, byte_offset) = bound_handle(inst);
        let var = self.image_variable(ResourceKind::Surface2D { cbuf_bank, byte_offset, format }, ty);
        let ra = (inst >> 24 & 0xff) as u32;
        let mut coords = [ra, ra + 1].map(|r| self.load_register(r));
        if byte_addressed {
            let two = self.cached_const_u32(2);
            coords[0] = self.ir.emit_shift_right_logical(self.type_u32[1], coords[0], two);
        }
        (var, ty, self.ir.emit_composite_construct(self.type_u32[2], &coords))
    }
    pub fn suquery(&mut self, inst: u128) {
        todo!("SUQUERY on SM86 only has COMP_STATUS, the memory compression state of the texel at [Ra], which Vulkan does not expose; surface sizes come from TXQ: {inst:#034x}");
    }
    pub fn sured(&mut self, inst: u128) {
        self.surface_atomic(inst, Some((inst >> 87 & 7) as u32));
    }

    // `op` numbers the operation as ATOM does, None is CAS comparing with Rb and swapping in Rb+1.
    fn surface_atomic(&mut self, inst: u128, op: Option<u32>) -> u32 {
        use spirv::memory_semantics::{ACQUIRE_RELEASE, IMAGE_MEMORY};
        assert!(matches!(opcode(inst), 0x593 | 0x595 | 0x59f), "surface atomics only in the bound cbuf form: {inst:#034x}");
        let size = (inst >> 73 & 7) as usize;
        let name = ["U32", "S32", "U64", "F32.FTZ.RN", "F16x2.RN", "S64", "SD32", "SD64"][size];
        assert!(size < 2, "surface atomic .{name}: the R32_UINT storage image holds 32-bit integer atomics only");
        let (var, _, coord) = self.surface(inst, vk::Format::R32_UINT, inst >> 72 & 1 != 0);
        let ptr_ty = self.ir.emit_type_pointer(storage_class::IMAGE, self.type_u32[1]);
        let ptr = self.ir.emit_image_texel_pointer(ptr_ty, var, coord, self.const_u32_0);
        let scope = self.cached_const_u32(spirv::scope::DEVICE);
        let sem = self.cached_const_u32(ACQUIRE_RELEASE | IMAGE_MEMORY);
        let rb = (inst >> 32 & 0xff) as u32;
        let value = self.load_register(rb);
        match op {
            Some(op) => self.atomic_op(false, ptr, scope, sem, op, size == 1, value),
            None => {
                let swap = self.load_register(nth_reg(rb, RZ, 1));
                self.ir.emit_atomic_compare_exchange(self.type_u32[1], ptr, scope, sem, self.const_u32_0, swap, value)
            }
        }
    }
    pub fn sust(&mut self, inst: u128) {
        let (image, coord, raw) = self.surface_2d(inst, [0xb9d, 0xb9b], "SUST");
        let rb = (inst >> 32 & 0xff) as u32;
        let texel = if raw {
            let mut bits = self.load_register(rb);
            let width = match inst >> 73 & 7 {
                0 | 1 => 8,
                2 | 3 => 16,
                4 => 32,
                size => panic!("SUST.D size {size} is wider than an RGBA8 texel"),
            };
            if width < 32 {
                assert!(inst >> 72 & 1 != 0, "SUST.D.U8 and .U16 only byte-addressed");
                let old = self.ir.emit_image_read(self.type_f32[4], image, coord, &[]);
                let old = self.glsl(spirv::glsl::PACK_UNORM_4X8, self.type_u32[1], &[old]);
                let x = self.load_register((inst >> 24 & 0xff) as u32);
                let [three, byte_bits, width] = [3, 24, width].map(|v| self.cached_const_u32(v));
                let x_bits = self.ir.emit_shift_left_logical(self.type_u32[1], x, three);
                let offset = self.ir.emit_bitwise_and(self.type_u32[1], x_bits, byte_bits);
                bits = self.ir.emit_bit_field_insert(self.type_u32[1], old, bits, offset, width);
            }
            self.glsl(spirv::glsl::UNPACK_UNORM_4X8, self.type_f32[4], &[bits])
        } else {
            assert!(inst >> 72 & 0xf == 0xf, "SUST.P component mask {:#x}", inst >> 72 & 0xf);
            let components = [0, 1, 2, 3].map(|c| {
                let bits = self.load_register(nth_reg(rb, RZ, c));
                self.bitcast_f32(bits)
            });
            self.ir.emit_composite_construct(self.type_f32[4], &components)
        };
        self.ir.emit_image_write(image, coord, texel, &[]);
    }
    pub fn tex(&mut self, inst: u128) {
        assert!(opcode(inst) == 0xb60 && inst >> 59 & 1 == 0, "TEX only in the bound cbuf form: {inst:#034x}");
        for (bit, modifier) in [(76, "AOFFI"), (77, "NDV"), (78, "DC"), (79, "F16.RN"), (80, "F16.RZ")] {
            assert!(inst >> bit & 1 == 0, "TEX.{modifier}");
        }
        let dimension = (inst >> 61 & 7) as usize;
        assert!(dimension == 1, "TEX.{}", ["1D", "2D", "3D", "CUBE", "ARRAY_1D", "ARRAY_2D", "INVALID6", "ARRAY_CUBE"][dimension]);
        assert!(inst >> 81 & 7 == PT as u128, "TEX sparse residency predicate");
        let [ra, rb] = [24, 32].map(|bit| (inst >> bit & 0xff) as u32);
        let lod = match inst >> 87 & 7 {
            0 => None,
            1 => Some(self.const_u32_0),
            3 => Some(self.load_register(rb)),
            mode => todo!("TEX.{}", ["implicit lod", "LZ", "LB", "LL", "LC", "LB.LC", "LC.FDV", "INVALID7"][mode as usize]),
        };
        // .SCR splits the operands between Ra and Rb, so without a lod operand y is in Rb (RTX 3090 fixtures).
        let y = if inst >> 60 & 1 != 0 && inst >> 87 & 7 < 2 { rb } else { ra + 1 };
        let coord = self.float_vec2([ra, y]);
        let sampled = self.texture_2d(inst);
        let texel = match lod {
            Some(lod) => {
                let lod = self.bitcast_f32(lod);
                self.ir.emit_image_sample_explicit_lod(self.type_f32[4], sampled, coord, LOD_OPERAND, &[lod])
            }
            None => {
                self.derivatives = true;
                self.ir.emit_image_sample_implicit_lod(self.type_f32[4], sampled, coord, &[])
            }
        };
        let bits = self.ir.emit_bitcast(self.type_u32[4], texel);
        self.store_texel(inst, bits);
    }
    pub fn tld(&mut self, inst: u128) {
        assert!(opcode(inst) == 0xb66 && inst >> 59 & 3 == 0, "TLD only in the bound cbuf form: {inst:#034x}");
        for (bit, modifier) in [(76, "AOFFI"), (77, "CL"), (78, "MS"), (79, "F16.RN"), (80, "F16.RZ")] {
            assert!(inst >> bit & 1 == 0, "TLD.{modifier}");
        }
        assert!(inst >> 61 & 7 == 1, "TLD dimension {} is not 2D", inst >> 61 & 7);
        assert!(inst >> 81 & 7 == PT as u128, "TLD sparse residency predicate");
        let [ra, rb] = [24, 32].map(|bit| (inst >> bit & 0xff) as u32);
        let lod = match inst >> 87 & 7 {
            1 => self.const_u32_0,
            3 => self.load_register(rb),
            mode => todo!("TLD lod mode {mode}"),
        };
        let coords = [ra, ra + 1].map(|r| self.load_register(r));
        let coord = self.ir.emit_composite_construct(self.type_u32[2], &coords);
        let sampled = self.texture_2d(inst);
        let image = self.ir.emit_image(self.type_texture_2d, sampled);
        let texel = self.ir.emit_image_fetch(self.type_f32[4], image, coord, &[LOD_OPERAND, lod]);
        let bits = self.ir.emit_bitcast(self.type_u32[4], texel);
        self.store_texel(inst, bits);
    }

    // Written components pack in RGBA order into Rd, Rd+1, then Rd2, Rd2+1; with Rd = RZ they start at Rd2, as the
    // NVIDIA driver's `TEX RZ, R0, ..., 0x2` that goes on to read the green texel from R0 shows.
    fn store_texel(&mut self, inst: u128, texel: u32) {
        let [rd, rd2] = [16, 64].map(|bit| (inst >> bit & 0xff) as u32);
        let [rd, rd2] = if rd == RZ { [rd2, RZ] } else { [rd, rd2] };
        let written = (0..4).filter(|c| inst >> (72 + c) & 1 != 0);
        for (i, component) in written.enumerate() {
            let value = self.ir.emit_composite_extract(self.type_u32[1], texel, &[component]);
            let reg = if i < 2 { nth_reg(rd, RZ, i as u32) } else { nth_reg(rd2, RZ, i as u32 - 2) };
            self.store_register(reg, value);
        }
    }

    // The texture unit reads a NaN operand as 0 (RTX 3090 fixtures).
    fn float_vec2(&mut self, regs: [u32; 2]) -> u32 {
        let floats = regs.map(|r| {
            let bits = self.load_register(r);
            let value = self.bitcast_f32(bits);
            let nan = self.ir.emit_is_nan(self.type_bool[1], value);
            let bits = self.ir.emit_select(self.type_u32[1], nan, self.const_u32_0, bits);
            self.bitcast_f32(bits)
        });
        self.ir.emit_composite_construct(self.type_f32[2], &floats)
    }

    fn texture_2d(&mut self, inst: u128) -> u32 {
        if self.type_sampled_texture_2d == 0 {
            self.ir.emit_capability(spirv::capability::IMAGE_QUERY);
            self.type_texture_2d = self.ir.emit_type_image(self.type_f32[1], DIM_2D, 0, 0, 0, 1, 0);
            self.type_sampled_texture_2d = self.ir.emit_type_sampled_image(self.type_texture_2d);
        }
        let (cbuf_bank, byte_offset) = bound_handle(inst);
        let var = self.image_variable(ResourceKind::Texture2D { cbuf_bank, byte_offset }, self.type_sampled_texture_2d);
        self.ir.emit_load(self.type_sampled_texture_2d, var)
    }

    // A bound texture or surface is keyed by the cbuf word holding its handle, so the host picks the image from that handle at dispatch.
    fn image_variable(&mut self, kind: ResourceKind, ty: u32) -> u32 {
        use spirv::{decoration as deco, storage_class::UNIFORM_CONSTANT};
        if let Some(var) = self.resource(kind) {
            return var;
        }
        let images = self.resources.iter().filter(|r| matches!(r.kind, ResourceKind::Texture2D { .. } | ResourceKind::Surface2D { .. }));
        let binding = TEXTURE_BINDING + images.count() as u32;
        let ptr = self.ir.emit_type_pointer(UNIFORM_CONSTANT, ty);
        let var = self.ir.emit_variable(ptr, UNIFORM_CONSTANT);
        self.ir.emit_decorate(var, deco::DESCRIPTOR_SET, &[0]);
        self.ir.emit_decorate(var, deco::BINDING, &[binding]);
        self.resources.push(Resource { kind, var });
        var
    }
    pub fn tld4(&mut self, inst: u128) {
        assert!(opcode(inst) == 0xb63 && inst >> 59 & 3 == 0, "TLD4 only in the bound cbuf form: {inst:#034x}");
        for (bit, modifier) in [(76, "AOFFI"), (77, "PTP"), (78, "DC"), (79, "F16.RN"), (80, "F16.RZ")] {
            assert!(inst >> bit & 1 == 0, "TLD4.{modifier}");
        }
        assert!(inst >> 61 & 7 == 1, "TLD4 dimension {} is not 2D", inst >> 61 & 7);
        assert!(inst >> 81 & 7 == PT as u128, "TLD4 sparse residency predicate");
        let ra = (inst >> 24 & 0xff) as u32;
        let coord = self.float_vec2([ra, ra + 1]);
        let sampled = self.texture_2d(inst);
        let component = self.cached_const_u32((inst >> 87 & 3) as u32);
        let texels = self.ir.emit_image_gather(self.type_f32[4], sampled, coord, component);
        let bits = self.ir.emit_bitcast(self.type_u32[4], texels);
        self.store_texel(inst, bits);
    }
    pub fn tmml(&mut self, inst: u128) {
        assert!(opcode(inst) == 0xb69 && inst >> 59 & 1 == 0, "TMML only in the bound cbuf form: {inst:#034x}");
        assert!(inst >> 77 & 1 == 0, "TMML.NDV");
        assert!(inst >> 61 & 7 == 1, "TMML dimension {} is not 2D", inst >> 61 & 7);
        assert!(inst >> 74 & 3 == 0, "TMML writes only the two lod components");
        let ra = (inst >> 24 & 0xff) as u32;
        let coord = self.float_vec2([ra, ra + 1]);
        let sampled = self.texture_2d(inst);
        self.derivatives = true;
        let lod = self.ir.emit_image_query_lod(self.type_f32[2], sampled, coord);
        // TMML returns signed 8.8 fixed point in the low half, the computed lod before the accessed mip level, the reverse of
        // OpImageQueryLod, and silicon clamps a lod below -32 (zero derivatives) to -32.
        let [scale, min_lod] = [256.0f32, -32.0].map(|c| self.ir.emit_constant_typed(self.type_f32[1], c));
        let low_half = self.cached_const_u32(0xffff);
        let fixed = [1, 0].map(|c| {
            let value = self.ir.emit_composite_extract(self.type_f32[1], lod, &[c]);
            let value = self.glsl(spirv::glsl::N_MAX, self.type_f32[1], &[value, min_lod]);
            let scaled = self.ir.emit_fmul(self.type_f32[1], value, scale);
            let fixed = self.ir.emit_convert_f_to_s(self.type_s32[1], scaled);
            let fixed = self.ir.emit_bitcast(self.type_u32[1], fixed);
            self.ir.emit_bitwise_and(self.type_u32[1], fixed, low_half)
        });
        let lods = self.ir.emit_composite_construct(self.type_u32[2], &fixed);
        self.store_texel(inst, lods);
    }
    pub fn txd(&mut self, inst: u128) {
        assert!(opcode(inst) == 0xb6c && inst >> 59 & 1 == 0, "TXD only in the bound cbuf form: {inst:#034x}");
        for (bit, modifier) in [(76, "AOFFI"), (79, "F16.RN"), (80, "F16.RZ"), (87, "LC")] {
            assert!(inst >> bit & 1 == 0, "TXD.{modifier}");
        }
        assert!(inst >> 61 & 7 == 1, "TXD dimension {} is not 2D", inst >> 61 & 7);
        assert!(inst >> 81 & 7 == PT as u128, "TXD sparse residency predicate");
        let [ra, rb] = [24, 32].map(|bit| (inst >> bit & 0xff) as u32);
        let coord = self.float_vec2([ra, ra + 1]);
        // Rb holds du/dx, du/dy, dv/dx, dv/dy.
        let [dx, dy] = [[0, 2], [1, 3]].map(|pair| self.float_vec2(pair.map(|i| nth_reg(rb, RZ, i))));
        let sampled = self.texture_2d(inst);
        let texel = self.ir.emit_image_sample_explicit_lod(self.type_f32[4], sampled, coord, GRAD_OPERAND, &[dx, dy]);
        let bits = self.ir.emit_bitcast(self.type_u32[4], texel);
        self.store_texel(inst, bits);
    }
    pub fn txq(&mut self, inst: u128) {
        assert!(opcode(inst) == 0xb6f && inst >> 59 & 1 == 0, "TXQ only in the bound cbuf form: {inst:#034x}");
        let query = (inst >> 62 & 3) as usize;
        assert!(query == 0, "TXQ {}", ["TEX_HEADER_DIMENSION", "TEX_HEADER_TEXTURE_TYPE", "TEX_HEADER_SAMPLER_POS", "INVALID3"][query]);
        let lod = self.load_register((inst >> 24 & 0xff) as u32);
        let sampled = self.texture_2d(inst);
        let image = self.ir.emit_image(self.type_texture_2d, sampled);
        let size = self.ir.emit_image_query_size_lod(self.type_u32[2], image, lod);
        let [width, height] = [0, 1].map(|c| self.ir.emit_composite_extract(self.type_u32[1], size, &[c]));
        let levels = self.ir.emit_image_query_levels(self.type_u32[1], image);
        let dimensions = self.ir.emit_composite_construct(self.type_u32[4], &[width, height, self.const_u32_0, levels]);
        self.store_texel(inst, dimensions);
    }
}
