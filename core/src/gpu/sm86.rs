#![allow(unused_parens)]

use crate::gpu::spirv;

static MAX_REG_COUNT: usize = 254;
static MAX_UNIFORM_REG_COUNT: usize = 63;
static MAX_CONST_BANK: usize = 17;
static ALLOW_F16_PARTIAL_WRITES: usize = 1;

enum TextureType {
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

enum TicSource {
    ZERO = 0,
    R = 2,
    G = 3,
    B = 4,
    A = 5,
    ONE_INT = 6,
    ONE_FLOAT = 7,
}

enum TicType {
    SNORM = 1,
    UNORM = 2,
    SINT = 3,
    UINT = 4,
    SNORM_FORCE_FP16 = 5,
    UNORM_FORCE_FP16 = 6,
    FLOAT = 7,
}

enum MaxAnisotropy {
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
enum SurfaceFormat {
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

enum ZetaFormat {
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
enum ImageFormat {
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

enum BitSize {
    B32 = 0b00,
    B64 = 0b01,
    B96 = 0b10,
    B128 = 0b11,
}

pub struct Decoder<'a> {
    pub ir: &'a mut spirv::Emitter,
    // Declared types for headers
    type_u8: u32,
    type_u16: u32,
    type_u32: u32,
    type_u64: u32,
    type_s8: u32,
    type_s16: u32,
    type_s32: u32,
    type_s64: u32,
    type_f16: u32,
    type_f32: u32,
    type_f64: u32,
}
impl<'a> Decoder<'a> {
    pub fn init(&mut self) {
        self.type_u8 = self.ir.emit_type_int(8, 0);
        self.type_u16 = self.ir.emit_type_int(16, 0);
        self.type_u32 = self.ir.emit_type_int(32, 0);
        self.type_u64 = self.ir.emit_type_int(64, 0);
        self.type_s8 = self.ir.emit_type_int(8, 1);
        self.type_s16 = self.ir.emit_type_int(16, 1);
        self.type_s32 = self.ir.emit_type_int(32, 1);
        self.type_s64 = self.ir.emit_type_int(64, 1);
        self.type_f16 = self.ir.emit_type_float(16);
        self.type_f32 = self.ir.emit_type_float(32);
        self.type_f64 = self.ir.emit_type_float(64);
    }
    pub fn finish(&mut self) {

    }

    pub fn al2p(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let ra = (((inst >> 24) & 0xff) << 0) as usize;
        let ra_offset = (((inst >> 40) & 0x7ff) << 0) as usize;
        let bop = (((inst >> 74) & 0x3) << 0) as usize;
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        // %rd := %ra + $ra_offset
        assert!(ra <= MAX_REG_COUNT);
        assert!(bop == BitSize::B32 as usize);
        let ir_imm32 = self.ir.emit_constant_typed::<u32>(self.type_u32, ra_offset as u32);
        todo!();
    }
    pub fn ald(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn arrives(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ast(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn atom(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn atomg(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn atoms(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn b2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _barname = (((inst >> 54) & 0xf) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc = (((inst >> 42) & 0xfff) << 0);
        let _barname = (((inst >> 54) & 0xf) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bitextract(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _num = (((inst >> 53) & 0x3) << 0);
        let _srchalf = (((inst >> 56) & 0x1) << 0);
        let _dstbyte = (((inst >> 57) & 0x3) << 0);
        let _datasize = (((inst >> 61) & 0x3) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _mdidx = (((inst >> 78) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bmma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _op = (((inst >> 77) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bmov(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bmsk(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bpt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 34) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bra(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cond = (((inst >> 32) & 0x3) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn break_(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _barreg = (((inst >> 16) & 0xf) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn brev(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn brx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn brxu(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _cond = (((inst >> 32) & 0x3) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bssy(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _barreg = (((inst >> 16) & 0xf) << 0);
        let _sa = (((inst >> 34) & 0x3fffffff) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn bsync(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _barreg = (((inst >> 16) & 0xf) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn call(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn cctl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _mode = (((inst >> 78) & 0x7) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn cctll(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn cctlt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn clmad(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn cs2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn csmtest(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sa = (((inst >> 32) & 0xfffffff) << 0);
        let _vtgmode = (((inst >> 62) & 0x3) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dadd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn depbar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _cnt = (((inst >> 38) & 0x3f) << 0);
        let _sbidx = (((inst >> 44) & 0x7) << 0);
        let _le = (((inst >> 47) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dfma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dmma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dmul(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn dsetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn errbar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn exit(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _mode = (((inst >> 84) & 0x3) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn f2f(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _dstfmt_srcfmt = (((inst >> 84) & 0x7) << 3) | (((inst >> 75) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn f2fp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _rndmode = (((inst >> 79) & 0x7) << 0);
        let _dstfmt = (((inst >> 86) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn f2i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _dstfmt = (((inst >> 75) & 0x3) << 1) | (((inst >> 72) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _mode = (((inst >> 84) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn f2ip(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fadd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fadd32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fchk(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ffma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ffma32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn flo(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fmnmx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fmul(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fmul32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn footprint(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn frnd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _fmt = (((inst >> 84) & 0x3) << 2) | (((inst >> 75) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fsel(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fset(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fsetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn fswzadd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn gather(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _num = (((inst >> 53) & 0x3) << 0);
        let _srchalf = (((inst >> 56) & 0x1) << 0);
        let _dstbyte = (((inst >> 57) & 0x3) << 0);
        let _datasize = (((inst >> 61) & 0x3) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _mdidx = (((inst >> 78) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn genmetadata(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _num = (((inst >> 53) & 0x3) << 0);
        let _seq = (((inst >> 55) & 0x1) << 0);
        let _fmt = (((inst >> 60) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _mdidx = (((inst >> 78) & 0xf) << 0);
        let _vecidx6 = (((inst >> 84) & 0x3f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn getlmembase(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hadd2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hadd2_32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _sc = (((inst >> 48) & 0xffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hfma2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satrelu = (((inst >> 79) & 0x1) << 1) | (((inst >> 77) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _mode = (((inst >> 81) & 0x3) << 0);
        let _clear = (((inst >> 83) & 0x1) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _ofmt = (((inst >> 85) & 0x1) << 1) | (((inst >> 78) & 0x1) << 0);
        let _iswzb = (((inst >> 86) & 0x1) << 2) | (((inst >> 60) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hfma2_mma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _satrelu = (((inst >> 79) & 0x1) << 1) | (((inst >> 77) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _clear = (((inst >> 83) & 0x1) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hfma2_32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _sc = (((inst >> 48) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satrelu = (((inst >> 79) & 0x1) << 1) | (((inst >> 77) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _mode = (((inst >> 81) & 0x3) << 0);
        let _clear = (((inst >> 83) & 0x1) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _ofmt = (((inst >> 85) & 0x1) << 1) | (((inst >> 78) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hmma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _re = (((inst >> 40) & 0xff) << 0);
        let _id = (((inst >> 48) & 0x3) << 0);
        let _re_reuse_src_e = (((inst >> 50) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _size = (((inst >> 78) & 0x1) << 1) | (((inst >> 75) & 0x1) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _srcfmt = (((inst >> 82) & 0x3) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hmnmx2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _ofmt = (((inst >> 85) & 0x1) << 1) | (((inst >> 78) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hmul2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hmul2_32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _sc = (((inst >> 48) & 0xffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _fmz = (((inst >> 80) & 0x1) << 1) | (((inst >> 76) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hset2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _bop = (((inst >> 69) & 0x3) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn hsetp2(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _bop = (((inst >> 69) & 0x3) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _test = (((inst >> 76) & 0xf) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn i2f(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _extract = (((inst >> 60) & 0x3) << 0);
        let _size = (((inst >> 75) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _srcfmt = (((inst >> 84) & 0x3) << 1) | (((inst >> 74) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn i2fp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 75) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _srcfmt = (((inst >> 84) & 0x3) << 1) | (((inst >> 74) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn i2i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn i2ip(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn iabs(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn iadd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn iadd3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn iadd32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ide(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn idp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn idp4a(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn imad(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn imma(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _re_reuse_src_e = (((inst >> 50) & 0x1) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _srcfmta = (((inst >> 83) & 0x1) << 2) | (((inst >> 76) & 0x3) << 0);
        let _srcfmtb = (((inst >> 84) & 0x1) << 2) | (((inst >> 78) & 0x3) << 0);
        let _size = (((inst >> 85) & 0x3) << 1) | (((inst >> 75) & 0x1) << 0);
        let _op = (((inst >> 87) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn imnmx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn imul(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn imul32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ipa(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn isberd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn isbewr(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn iscadd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _scaleu5 = (((inst >> 75) & 0x1f) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn iscadd32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _scaleu5 = (((inst >> 75) & 0x1f) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn isetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn jmp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cond = (((inst >> 32) & 0x3) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn jmx(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn jmxu(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _cond = (((inst >> 32) & 0x3) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _depth = (((inst >> 85) & 0x3) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn kill(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _pnz = (((inst >> 64) & 0xf) << 0);
        let _sp2 = (((inst >> 68) & 0x3) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 38) & 0xffff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldg(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _pnz = (((inst >> 64) & 0xf) << 0);
        let _sp2 = (((inst >> 68) & 0x3) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldgdepbar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldgsts(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xfff) << 0);
        let _rb_offset = (((inst >> 44) & 0xfffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _input_reg_sz_32_dist = (((inst >> 70) & 0x1) << 0);
        let _sp2 = (((inst >> 71) & 0x3) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _nan = (((inst >> 81) & 0x1) << 0);
        let _rndsrc = (((inst >> 82) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lds(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldsm(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _sz = (((inst >> 75) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ldtram(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lea(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _scaleu5 = (((inst >> 75) & 0x1f) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lepc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lop(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lop3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn lop32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn match_(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _op = (((inst >> 79) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn membar(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn mov(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn mov32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn movm(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sz = (((inst >> 75) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn mufu(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_absolute = (((inst >> 62) & 0x1) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _mufuop = (((inst >> 74) & 0xf) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn nanosleep(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clear = (((inst >> 83) & 0x1) << 0);
        let _warp = (((inst >> 85) & 0x1) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn nanotrap(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn nop(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        self.ir.emit_nop();
    }
    pub fn out(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn p2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn pixld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _mode = (((inst >> 78) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn plop3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _up = (((inst >> 67) & 0x1) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _lop = (((inst >> 72) & 0x1f) << 11)
            | (((inst >> 64) & 0x7) << 8)
            | (((inst >> 16) & 0xff) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn pmtrig(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn popc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn prmt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pmode = (((inst >> 72) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn psetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _up = (((inst >> 67) & 0x1) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _lop = (((inst >> 72) & 0x1f) << 11)
            | (((inst >> 64) & 0x7) << 8)
            | (((inst >> 16) & 0xff) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn qspc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn r2b(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _barname = (((inst >> 54) & 0xf) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn r2p(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn r2ur(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _clear = (((inst >> 84) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn red(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn redux(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _mode = (((inst >> 78) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ret(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _simm = (((inst >> 34) & 0xffffffffffff) << 0);
        let _warp = (((inst >> 85) & 0x1) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn rpcmov(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn rtt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn s2r(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn s2ur(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn scatter(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _mode = (((inst >> 81) & 0x3) << 0);
        let _vecidx = (((inst >> 83) & 0x7f) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sel(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn setctaid(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn setlmembase(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sgxt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn shf(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn shfl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc = (((inst >> 40) & 0x1fff) << 0);
        let _sb = (((inst >> 53) & 0x1f) << 0);
        let _shflmd = (((inst >> 58) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn shl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn shr(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn spmetadata(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _mode = (((inst >> 81) & 0x3) << 0);
        let _vecidx = (((inst >> 83) & 0x7f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn st(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn stg(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn stl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sts(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffffff) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _stride = (((inst >> 78) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn suatom(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn suld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn suquery(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cas = (((inst >> 87) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sured(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn sust(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _clamp = (((inst >> 59) & 0x3) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _mem = (((inst >> 77) & 0xf) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tex(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tld4(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _scr = (((inst >> 60) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _comp = (((inst >> 87) & 0x3) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn tmml(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _satfinite = (((inst >> 77) & 0x1) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttucctl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttuclose(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttugo(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttuld(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttumacrofuse(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sb = (((inst >> 40) & 0x1f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttuopen(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ttust(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _ra_offset = (((inst >> 40) & 0xffff) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn txd(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _rb = (((inst >> 32) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _destidx = (((inst >> 61) & 0x7) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _sem = (((inst >> 79) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _cas = (((inst >> 87) & 0x1) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn txq(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _b = (((inst >> 59) & 0x1) << 0);
        let _vtgmode = (((inst >> 62) & 0x3) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _pixmasku04 = (((inst >> 72) & 0xf) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ubmsk(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ubrev(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uclea(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _sb = (((inst >> 32) & 0xffff) << 0);
        let _sz = (((inst >> 73) & 0xf) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uf2fp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _merge = (((inst >> 78) & 0x1) << 0);
        let _rndmode = (((inst >> 79) & 0x7) << 0);
        let _fmt = (((inst >> 84) & 0x3) << 2) | (((inst >> 75) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uflo(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uiadd3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uiadd3_64(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uimad(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uisetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uldc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_offset = (((inst >> 38) & 0xffff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _sz = (((inst >> 73) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ulea(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_absolute = (((inst >> 74) & 0x1) << 0);
        let _scaleu5 = (((inst >> 75) & 0x1f) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ulop(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ulop3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ulop32i(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _imm8 = (((inst >> 72) & 0xff) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn umov(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn up2ur(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uplop3(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _memdesc = (((inst >> 71) & 0x1) << 0);
        let _lop = (((inst >> 72) & 0x1f) << 11)
            | (((inst >> 64) & 0x7) << 8)
            | (((inst >> 16) & 0xff) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn upopc(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sc_negate = (((inst >> 63) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn uprmt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _e = (((inst >> 72) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn upsetp(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pr = (((inst >> 68) & 0x7) << 0);
        let _lop = (((inst >> 72) & 0x1f) << 11)
            | (((inst >> 64) & 0x7) << 8)
            | (((inst >> 16) & 0xff) << 0);
        let _pq = (((inst >> 77) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ur2up(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _dstfmt = (((inst >> 76) & 0x3) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn usel(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn usgxt(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ushf(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ushl(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_urb = (((inst >> 32) & 0x3f) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn ushr(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _cbu_state = (((inst >> 24) & 0x3f) << 0);
        let _ra_offset = (((inst >> 32) & 0xffffffff) << 0);
        let _ra_urc = (((inst >> 64) & 0x3f) << 0);
        let _sz = (((inst >> 73) & 0x3) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _memdesc = (((inst >> 76) & 0x1) << 0);
        let _ftz = (((inst >> 80) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn vabsdiff(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn vabsdiff4(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _ra = (((inst >> 24) & 0xff) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _rc = (((inst >> 64) & 0xff) << 0);
        let _sz = (((inst >> 73) & 0x1) << 0);
        let _sc_negate = (((inst >> 75) & 0x1) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn vote(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _rd = (((inst >> 16) & 0xff) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn voteu(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _urd = (((inst >> 16) & 0x3f) << 0);
        let _num = (((inst >> 72) & 0x3) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn vote_vtg(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sa = (((inst >> 32) & 0xfffffff) << 0);
        let _vtgmode = (((inst >> 62) & 0x3) << 0);
        let _bop = (((inst >> 74) & 0x3) << 0);
        let _dstfmt = (((inst >> 76) & 0x7) << 0);
        let _pu = (((inst >> 81) & 0x7) << 0);
        let _cop = (((inst >> 84) & 0x7) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn warpsync(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _sc_addr = (((inst >> 40) & 0x3fff) << 0);
        let _sc_bank = (((inst >> 54) & 0x1f) << 0);
        let _depth = (((inst >> 86) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
    pub fn yield_(&mut self, inst: u128) {
        let _pg = (((inst >> 12) & 0x7) << 0);
        let _pg_not = (((inst >> 15) & 0x1) << 0);
        let _pp = (((inst >> 87) & 0x7) << 0);
        let _input_reg_sz_32_dist = (((inst >> 90) & 0x1) << 0);
        let _pm_pred = (((inst >> 102) & 0x3) << 0);
        let _dst_wr_sb = (((inst >> 110) & 0x7) << 0);
        let _src_rel_sb = (((inst >> 113) & 0x7) << 0);
        let _req_bit_set = (((inst >> 116) & 0x3f) << 0);
        let _opex = (((inst >> 122) & 0x7) << 5) | (((inst >> 105) & 0x1f) << 0);
        todo!();
    }
}

include!("sm86_decoder_generated.rs");
