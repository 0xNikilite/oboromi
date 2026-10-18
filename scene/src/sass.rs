//! The SM86 instruction encoders the scene programs need; the host tests build the rest on `alu` and `control`.

pub const PT_GUARD: u128 = 7 << 12;
pub const RC_RZ: u128 = 0xff << 64;
pub const PIXMASK_ALL: u128 = 0xf << 72;
pub const PP_PT: u128 = 7 << 87;
pub const NO_SCOREBOARD: u128 = 0x3f << 110;
pub const NO_WRITE_SCOREBOARD: u128 = 7 << 110;
pub const FMUL_NOSCALE: u128 = 4 << 84;
const MOV_RI: u128 = 0x802;
const MOV_RC: u128 = 0xa02;
const FMUL_RR: u128 = 0x220;
const FFMA_RRR: u128 = 0x223;
const ALD: u128 = 0x321 | 0xff << 24 | 0xff << 32;
const AST: u128 = 0x322 | 0xff << 24 | RC_RZ | NO_WRITE_SCOREBOARD;
const IPA: u128 = 0x326 | 0xff << 32 | 7 << 81;
const BRA: u128 = 0x947;
const EXIT: u128 = 0x94d;

pub const fn mov_imm(rd: u32, imm32: u32) -> u128 {
    MOV_RI | PT_GUARD | PIXMASK_ALL | ((rd as u128) & 0xff) << 16 | (imm32 as u128) << 32
}

pub const fn mov_cbuf(rd: u32, bank: u32, off: u32) -> u128 {
    MOV_RC | PT_GUARD | PIXMASK_ALL | ((rd as u128) & 0xff) << 16 | ((off as u128 >> 2) & 0x3fff) << 40 | ((bank as u128) & 0x1f) << 54
}

pub const fn control(op: u128, ra: u32, offset: i64) -> u128 {
    op | PT_GUARD | PP_PT | NO_SCOREBOARD | (ra as u128 & 0xff) << 24 | ((offset as u128 >> 2) & 0xffff_ffff_ffff) << 34
}

pub const fn bra(offset: i64) -> u128 {
    control(BRA, 0, offset)
}

pub const fn exit() -> u128 {
    control(EXIT, 0, 0)
}

pub const fn alu(op: u128, rd: u32, ra: u32, b: u32) -> u128 {
    op | PT_GUARD | (rd as u128) << 16 | (ra as u128) << 24 | (b as u128) << 32
}

pub const fn fmul(rd: u32, ra: u32, rb: u32) -> u128 {
    alu(FMUL_RR, rd, ra, rb) | FMUL_NOSCALE
}

pub const fn ffma(rd: u32, ra: u32, rb: u32, rc: u32) -> u128 {
    alu(FFMA_RRR, rd, ra, rb) | (rc as u128) << 64
}

pub const fn ald(rd: u32, attr: u32, words: u32) -> u128 {
    ALD | PT_GUARD | (rd as u128) << 16 | (attr as u128) << 40 | (words as u128 - 1) << 74
}

pub const fn ast(attr: u32, rb: u32, words: u32) -> u128 {
    AST | PT_GUARD | (rb as u128) << 32 | (attr as u128) << 40 | (words as u128 - 1) << 74
}

pub const fn ipa(rd: u32, attr: u32, mode: u128) -> u128 {
    IPA | PT_GUARD | mode | (rd as u128) << 16 | (attr as u128 >> 2) << 64
}
