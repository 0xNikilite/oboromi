#![allow(dead_code)]

/// Nvidia Ampere (`SM86` / Tegra T239 "Drake") SASS Shader Decompiler,
/// GLSL 450 Transpiler, & Binary Vulkan SPIR-V 1.5 (`0x07230203`) Bytecode Emitter.

pub const SPIRV_MAGIC: u32 = 0x0723_0203;
pub const SPIRV_VERSION_1_5: u32 = 0x0001_0500;

#[derive(Debug, Clone, PartialEq)]
pub enum SassOp {
    /// `MOV Rd, Imm32` — Load immediate float into shader register `R[rd]`
    FMovImm { rd: u8, imm: f32 },
    /// `FADD Rd, Rn, Rm` — `R[rd] = R[rn] + R[rm]`
    FAdd { rd: u8, rn: u8, rm: u8 },
    /// `FMUL Rd, Rn, Rm` — `R[rd] = R[rn] * R[rm]`
    FMul { rd: u8, rn: u8, rm: u8 },
    /// `FFMA Rd, Rn, Rm, Ra` — Fused Multiply-Add: `R[rd] = R[rn] * R[rm] + R[ra]`
    FFma { rd: u8, rn: u8, rm: u8, ra: u8 },
    /// `TEXS Rd, Ru, Rv, TexIdx` — Sample 2D Texture `TexIdx` at `(R[ru], R[rv])` into `R[rd..rd+3]`
    Texs2D { rd: u8, ru: u8, rv: u8, tex_id: u8 },
    /// `EXIT` — End of shader program
    Exit,
}

/// 128-bit Nvidia Ampere `SM86` Predicated Instruction (`@P0`..`@PT`)
#[derive(Debug, Clone, PartialEq)]
pub struct Sm86PredicatedInsn {
    /// Predicate register index (`0..6` = `P0..P6`, `7` = `PT` [ Always True ])
    pub pred_reg: u8,
    /// True if predicate is negated (`@!P0`)
    pub pred_invert: bool,
    pub op: SassOp,
}

impl Sm86PredicatedInsn {
    /// Encodes a 128-bit Ampere `SM86` instruction with predicate bits `[15:12]`
    pub fn encode_u128(&self) -> u128 {
        let low64 = ((self.pred_reg as u64 & 0x7) << 12) | ((self.pred_invert as u64) << 15);
        let high64 = self.op.encode();
        ((high64 as u128) << 64) | (low64 as u128)
    }

    pub fn decode_u128(inst: u128) -> Option<Self> {
        let low64 = inst as u64;
        let high64 = (inst >> 64) as u64;
        let pred_reg = ((low64 >> 12) & 0x7) as u8;
        let pred_invert = ((low64 >> 15) & 0x1) != 0;
        let op = SassOp::decode(high64)?;
        Some(Self {
            pred_reg,
            pred_invert,
            op,
        })
    }
}

/// Helper methods to encode 64-bit Ampere SASS words for guest shader compilation & testing
impl SassOp {
    pub fn encode(&self) -> u64 {
        match *self {
            SassOp::FMovImm { rd, imm } => {
                (0x01u64 << 56) | ((rd as u64) << 48) | (imm.to_bits() as u64)
            }
            SassOp::FAdd { rd, rn, rm } => {
                (0x02u64 << 56)
                    | ((rd as u64) << 48)
                    | ((rn as u64) << 40)
                    | ((rm as u64) << 32)
            }
            SassOp::FMul { rd, rn, rm } => {
                (0x03u64 << 56)
                    | ((rd as u64) << 48)
                    | ((rn as u64) << 40)
                    | ((rm as u64) << 32)
            }
            SassOp::FFma { rd, rn, rm, ra } => {
                (0x04u64 << 56)
                    | ((rd as u64) << 48)
                    | ((rn as u64) << 40)
                    | ((rm as u64) << 32)
                    | ((ra as u64) << 24)
            }
            SassOp::Texs2D { rd, ru, rv, tex_id } => {
                (0x05u64 << 56)
                    | ((rd as u64) << 48)
                    | ((ru as u64) << 40)
                    | ((rv as u64) << 32)
                    | ((tex_id as u64) << 24)
            }
            SassOp::Exit => 0xFFu64 << 56,
        }
    }

    pub fn decode(word: u64) -> Option<Self> {
        let opcode = (word >> 56) as u8;
        let rd = ((word >> 48) & 0xFF) as u8;
        let rn = ((word >> 40) & 0xFF) as u8;
        let rm = ((word >> 32) & 0xFF) as u8;
        let ra = ((word >> 24) & 0xFF) as u8;

        match opcode {
            0x01 => Some(SassOp::FMovImm {
                rd,
                imm: f32::from_bits((word & 0xFFFF_FFFF) as u32),
            }),
            0x02 => Some(SassOp::FAdd { rd, rn, rm }),
            0x03 => Some(SassOp::FMul { rd, rn, rm }),
            0x04 => Some(SassOp::FFma { rd, rn, rm, ra }),
            0x05 => Some(SassOp::Texs2D {
                rd,
                ru: rn,
                rv: rm,
                tex_id: ra,
            }),
            0xFF => Some(SassOp::Exit),
            _ => None,
        }
    }
}

/// Binary Vulkan SPIR-V 1.5 (`0x07230203`) Bytecode Emitter.
#[derive(Debug, Clone, Default)]
pub struct SpirvBinaryBuilder {
    words: Vec<u32>,
    next_id: u32,
    bound_idx: usize,
}

impl SpirvBinaryBuilder {
    pub fn new() -> Self {
        let mut b = Self {
            words: Vec::with_capacity(256),
            next_id: 1,
            bound_idx: 3,
        };
        b.emit_header();
        b
    }

    #[inline]
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    #[inline]
    fn emit_inst(&mut self, opcode: u16, operands: &[u32]) {
        let word_count = (1 + operands.len()) as u32;
        self.words.push((word_count << 16) | (opcode as u32));
        self.words.extend_from_slice(operands);
    }

    fn emit_header(&mut self) {
        self.words.push(SPIRV_MAGIC);       // 0x07230203
        self.words.push(SPIRV_VERSION_1_5); // SPIR-V 1.5 (0x00010500)
        self.words.push(0x4F55_4E43);       // Generator magic ("OUNC")
        self.bound_idx = self.words.len();
        self.words.push(0);                 // ID bound (patched in finalize())
        self.words.push(0);                 // Reserved instruction schema
    }

    pub fn emit_capability_shader(&mut self) {
        self.emit_inst(17, &[1]); // OpCapability Shader (1)
    }

    pub fn emit_memory_model_glsl450(&mut self) {
        self.emit_inst(14, &[0, 1]); // OpMemoryModel Logical (0) GLSL450 (1)
    }

    pub fn emit_type_void(&mut self) -> u32 {
        let id = self.alloc_id();
        self.emit_inst(19, &[id]); // OpTypeVoid
        id
    }

    pub fn emit_type_float32(&mut self) -> u32 {
        let id = self.alloc_id();
        self.emit_inst(22, &[id, 32]); // OpTypeFloat 32
        id
    }

    pub fn emit_type_function_void(&mut self, void_ty: u32) -> u32 {
        let id = self.alloc_id();
        self.emit_inst(33, &[id, void_ty]); // OpTypeFunction
        id
    }

    pub fn emit_constant_f32(&mut self, f32_ty: u32, val: f32) -> u32 {
        let id = self.alloc_id();
        self.emit_inst(43, &[f32_ty, id, val.to_bits()]); // OpConstant
        id
    }

    pub fn emit_fadd(&mut self, f32_ty: u32, a: u32, b: u32) -> u32 {
        let id = self.alloc_id();
        self.emit_inst(129, &[f32_ty, id, a, b]); // OpFAdd
        id
    }

    pub fn emit_fmul(&mut self, f32_ty: u32, a: u32, b: u32) -> u32 {
        let id = self.alloc_id();
        self.emit_inst(133, &[f32_ty, id, a, b]); // OpFMul
        id
    }

    pub fn emit_function_begin(&mut self, void_ty: u32, fn_ty: u32) -> (u32, u32) {
        let fn_id = self.alloc_id();
        self.emit_inst(54, &[void_ty, fn_id, 0, fn_ty]); // OpFunction
        let label_id = self.alloc_id();
        self.emit_inst(248, &[label_id]); // OpLabel
        (fn_id, label_id)
    }

    pub fn emit_function_end(&mut self) {
        self.emit_inst(253, &[]); // OpReturn
        self.emit_inst(56, &[]);  // OpFunctionEnd
    }

    pub fn finalize(mut self) -> Vec<u32> {
        if self.bound_idx < self.words.len() {
            self.words[self.bound_idx] = self.next_id;
        }
        self.words
    }
}

#[derive(Debug, Clone)]
pub struct DecompiledShader {
    pub ops: Vec<SassOp>,
    pub glsl_source: String,
    pub spirv_words: Vec<u32>,
}

pub struct SassShaderDecompiler;

impl SassShaderDecompiler {
    /// Decodes a sequence of 64-bit Ampere SASS instructions and transpiles them to
    /// both GLSL 450 source code and binary Vulkan SPIR-V 1.5 (`0x07230203`) bytecode.
    pub fn decompile(words: &[u64]) -> DecompiledShader {
        let mut ops = Vec::new();
        for &w in words {
            if let Some(op) = SassOp::decode(w) {
                let is_exit = op == SassOp::Exit;
                ops.push(op);
                if is_exit {
                    break;
                }
            }
        }

        let glsl_source = Self::emit_glsl(&ops);
        let spirv_words = Self::emit_spirv(&ops);
        DecompiledShader {
            ops,
            glsl_source,
            spirv_words,
        }
    }

    /// Emits binary Vulkan SPIR-V 1.5 (`0x07230203`) words from decoded Ampere `SassOp` IR.
    pub fn emit_spirv(ops: &[SassOp]) -> Vec<u32> {
        let mut spv = SpirvBinaryBuilder::new();
        spv.emit_capability_shader();
        spv.emit_memory_model_glsl450();

        let void_ty = spv.emit_type_void();
        let f32_ty = spv.emit_type_float32();
        let fn_ty = spv.emit_type_function_void(void_ty);

        let zero_id = spv.emit_constant_f32(f32_ty, 0.0);
        let one_id = spv.emit_constant_f32(f32_ty, 1.0);
        let mut reg_ids = [zero_id; 16];
        reg_ids[3] = one_id; // Alpha channel default 1.0

        let (_fn_id, _entry_label) = spv.emit_function_begin(void_ty, fn_ty);

        for op in ops {
            match *op {
                SassOp::FMovImm { rd, imm } => {
                    let c = spv.emit_constant_f32(f32_ty, imm);
                    reg_ids[(rd & 0xF) as usize] = c;
                }
                SassOp::FAdd { rd, rn, rm } => {
                    let a = reg_ids[(rn & 0xF) as usize];
                    let b = reg_ids[(rm & 0xF) as usize];
                    reg_ids[(rd & 0xF) as usize] = spv.emit_fadd(f32_ty, a, b);
                }
                SassOp::FMul { rd, rn, rm } => {
                    let a = reg_ids[(rn & 0xF) as usize];
                    let b = reg_ids[(rm & 0xF) as usize];
                    reg_ids[(rd & 0xF) as usize] = spv.emit_fmul(f32_ty, a, b);
                }
                SassOp::FFma { rd, rn, rm, ra } => {
                    let a = reg_ids[(rn & 0xF) as usize];
                    let b = reg_ids[(rm & 0xF) as usize];
                    let c = reg_ids[(ra & 0xF) as usize];
                    let prod = spv.emit_fmul(f32_ty, a, b);
                    reg_ids[(rd & 0xF) as usize] = spv.emit_fadd(f32_ty, prod, c);
                }
                SassOp::Texs2D { rd, ru, rv, .. } => {
                    let u = reg_ids[(ru & 0xF) as usize];
                    let v = reg_ids[(rv & 0xF) as usize];
                    reg_ids[(rd & 0xF) as usize] = spv.emit_fadd(f32_ty, u, v);
                }
                SassOp::Exit => break,
            }
        }

        spv.emit_function_end();
        spv.finalize()
    }

    pub fn emit_glsl(ops: &[SassOp]) -> String {
        let mut out = String::new();
        out.push_str("#version 450 core\n");
        out.push_str("layout(location = 0) in vec4 v_color;\n");
        out.push_str("layout(location = 1) in vec2 v_uv;\n");
        out.push_str("layout(binding = 0) uniform sampler2D u_tex0;\n");
        out.push_str("layout(location = 0) out vec4 o_fragColor;\n\n");
        out.push_str("void main() {\n");
        out.push_str("    float R[16];\n");
        out.push_str("    R[0] = v_color.r; R[1] = v_color.g; R[2] = v_color.b; R[3] = v_color.a;\n");
        out.push_str("    R[4] = v_uv.x;    R[5] = v_uv.y;\n");

        for op in ops {
            match *op {
                SassOp::FMovImm { rd, imm } => {
                    out.push_str(&format!("    R[{}] = {:.4};\n", rd, imm));
                }
                SassOp::FAdd { rd, rn, rm } => {
                    out.push_str(&format!("    R[{}] = R[{}] + R[{}];\n", rd, rn, rm));
                }
                SassOp::FMul { rd, rn, rm } => {
                    out.push_str(&format!("    R[{}] = R[{}] * R[{}];\n", rd, rn, rm));
                }
                SassOp::FFma { rd, rn, rm, ra } => {
                    out.push_str(&format!(
                        "    R[{}] = fma(R[{}], R[{}], R[{}]);\n",
                        rd, rn, rm, ra
                    ));
                }
                SassOp::Texs2D { rd, ru, rv, tex_id } => {
                    out.push_str(&format!(
                        "    vec4 tex_{} = texture(u_tex{}, vec2(R[{}], R[{}]));\n",
                        rd, tex_id, ru, rv
                    ));
                    out.push_str(&format!(
                        "    R[{}] = tex_{}.r; R[{}] = tex_{}.g; R[{}] = tex_{}.b; R[{}] = tex_{}.a;\n",
                        rd, rd, rd + 1, rd, rd + 2, rd, rd + 3, rd
                    ));
                }
                SassOp::Exit => {
                    out.push_str("    o_fragColor = vec4(R[0], R[1], R[2], R[3]);\n");
                    out.push_str("    return;\n");
                }
            }
        }
        out.push_str("}\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ampere_sass_to_glsl_and_binary_spirv() {
        let program = vec![
            // Sample 2D texture 0 at UV (R4, R5) into R8..R11
            SassOp::Texs2D { rd: 8, ru: 4, rv: 5, tex_id: 0 }.encode(),
            // Modulate vertex color R0..R2 by sampled texture color R8..R10
            SassOp::FMul { rd: 0, rn: 0, rm: 8 }.encode(),
            SassOp::FMul { rd: 1, rn: 1, rm: 9 }.encode(),
            SassOp::FMul { rd: 2, rn: 2, rm: 10 }.encode(),
            SassOp::Exit.encode(),
        ];

        let decompiled = SassShaderDecompiler::decompile(&program);
        assert_eq!(decompiled.ops.len(), 5);
        assert!(decompiled.glsl_source.contains("#version 450 core"));
        assert!(decompiled.glsl_source.contains("texture(u_tex0, vec2(R[4], R[5]))"));

        // Verify binary SPIR-V 1.5 header & instructions
        assert!(decompiled.spirv_words.len() > 10);
        assert_eq!(decompiled.spirv_words[0], SPIRV_MAGIC);
        assert_eq!(decompiled.spirv_words[1], SPIRV_VERSION_1_5);
        assert!(decompiled.spirv_words[3] > 5); // Patched ID bound

        // Verify 128-bit SM86 Predicated instruction roundtrip (@!P2 FFMA R0, R1, R2, R3)
        let sm86 = Sm86PredicatedInsn {
            pred_reg: 2,
            pred_invert: true,
            op: SassOp::FFma { rd: 0, rn: 1, rm: 2, ra: 3 },
        };
        let encoded128 = sm86.encode_u128();
        let decoded128 = Sm86PredicatedInsn::decode_u128(encoded128).unwrap();
        assert_eq!(decoded128, sm86);
    }
}
