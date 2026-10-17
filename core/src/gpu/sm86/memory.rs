use super::*;

pub enum BitSize {
    B32 = 0b00,
    B64 = 0b01,
    B96 = 0b10,
    B128 = 0b11,
}

impl<'a> Decoder<'a> {
    // %rd := %ra + $ra_offset
    pub fn al2p(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x7ff) << 0) as u32;
        let bop = (((inst >> 74) & 0x3) << 0) as u32;
        assert!(bop == BitSize::B32 as u32);
        let base = self.load_register(ra);
        let offset = self.ir.emit_constant_typed(self.type_u32[1], ra_offset as u32);
        let dst_val = self.ir.emit_iadd(self.type_u32[1], base, offset);
        self.store_register(rd, dst_val);
    }
    // Attribute addresses as the SPH IMAP/OMAP bits order them (clc797sph.h): primitive id, layer and viewport 0x60 to 0x68, position
    // 0x70, generic n 0x80 + 16n, tessellation coordinate 0x2f0, instance and vertex id 0x2f8 and 0x2fc; front facing 0x3fc is NAK's.
    // ALD.P and AST.P address the patch instead: outer and inner tessellation levels at 0x0 and 0x10, generic n at 0x20 + 16n (NAK
    // nak_private.h).
    pub fn ald(&mut self, inst: u128) {
        use spirv::{BuiltIn, storage_class::INPUT};
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x3ff) << 0) as u32;
        let words = (((inst >> 74) & 0x3) << 0) as u32 + 1;
        let (patch, output) = (inst >> 76 & 1 != 0, inst >> 79 & 1 != 0);
        assert!(opcode(inst) == 0x321 && inst >> 24 & 0xff == RZ as u128 && inst >> 77 & 1 == 0, "only ALD at an immediate attribute address is implemented");
        let vertex = self.load_register(rb);
        let primitives = matches!(self.stage, Stage::Geometry { .. } | Stage::TessControl { .. } | Stage::TessEval { .. });
        for i in 0..words {
            let attr = ra_offset + 4 * i;
            let val = match (self.stage, patch, attr) {
                (Stage::Vertex { .. }, false, 0x80..0x280) if !output => {
                    let kind = ResourceKind::VertexInput { location: (attr - 0x80) / 16 };
                    let ptr = self.interface_component(kind, INPUT, attr);
                    let ty = self.interface_type(kind, 1);
                    let val = self.ir.emit_load(ty, ptr);
                    if ty == self.type_u32[1] { val } else { self.bitcast_u32(val) }
                }
                (Stage::Vertex { .. }, false, 0x2f8) => self.builtin(BuiltIn::InstanceIndex),
                (Stage::Vertex { .. }, false, 0x2fc) => self.builtin(BuiltIn::VertexIndex),
                (_, false, 0x60) if primitives => self.builtin(BuiltIn::PrimitiveId),
                (Stage::TessEval { .. }, false, 0x2f0 | 0x2f4) => {
                    let coord = self.builtin(BuiltIn::TessCoord);
                    let val = self.ir.emit_composite_extract(self.type_f32[1], coord, &[attr / 4 % 4]);
                    self.bitcast_u32(val)
                }
                _ => {
                    let ptr = self.attribute_component(attr, patch, output, vertex);
                    let ptr = ptr.unwrap_or_else(|| panic!("ALD a[{attr:#x}] in a {:?} module is not implemented", self.stage));
                    let val = self.ir.emit_load(self.type_f32[1], ptr);
                    self.bitcast_u32(val)
                }
            };
            self.store_register(nth_reg(rd, RZ, i), val);
        }
    }
    pub fn ast(&mut self, inst: u128) {
        use spirv::{BuiltIn, storage_class::OUTPUT};
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0x3ff) << 0) as u32;
        let rc = (((inst >> 64) & 0xff) << 0) as u32;
        let words = (((inst >> 74) & 0x3) << 0) as u32 + 1;
        let patch = inst >> 76 & 1 != 0;
        assert!(opcode(inst) == 0x322 && inst >> 24 & 0xff == RZ as u128 && inst >> 77 & 1 == 0, "only AST at an immediate attribute address is implemented");
        let vertex = self.load_register(rc);
        for i in 0..words {
            let attr = ra_offset + 4 * i;
            let val = self.load_register(nth_reg(rb, RZ, i));
            if matches!(self.stage, Stage::Geometry { .. }) && !patch && matches!(attr, 0x64 | 0x68) {
                let builtin = if attr == 0x64 { BuiltIn::Layer } else { BuiltIn::ViewportIndex };
                let var = self.interface_var(ResourceKind::BuiltIn(builtin as u32), OUTPUT);
                self.ir.emit_store(var, val);
                continue;
            }
            let ptr = self.attribute_component(attr, patch, true, vertex);
            let ptr = ptr.unwrap_or_else(|| panic!("AST a[{attr:#x}] in a {:?} module is not implemented", self.stage));
            let val = self.bitcast_f32(val);
            self.ir.emit_store(ptr, val);
        }
    }
    // The f32 interface component at `attr`, of `vertex` where the stage keeps that attribute per vertex.
    fn attribute_component(&mut self, attr: u32, patch: bool, output: bool, vertex: u32) -> Option<u32> {
        use spirv::{BuiltIn, storage_class::{INPUT, OUTPUT}};
        let storage = if output { OUTPUT } else { INPUT };
        let (tessellation, per_vertex) = match self.stage {
            Stage::TessControl { .. } => (true, true),
            Stage::TessEval { .. } => (true, !output),
            Stage::Geometry { .. } => (false, !output),
            _ => (false, false),
        };
        let kind = match (patch, attr) {
            (true, _) if !tessellation => return None,
            (true, 0..0x10) => ResourceKind::BuiltIn(BuiltIn::TessLevelOuter as u32),
            (true, 0x10..0x18) => ResourceKind::BuiltIn(BuiltIn::TessLevelInner as u32),
            (true, 0x20..) => ResourceKind::Patch { location: (attr - 0x20) / 16 },
            (false, 0x70..0x280) if per_vertex => {
                let kind = ResourceKind::PerVertex { attr: attr & !0xf, output };
                return Some(self.vertex_component(kind, storage, &[vertex], attr));
            }
            (false, 0x70..0x80) if output => ResourceKind::BuiltIn(BuiltIn::Position as u32),
            (false, 0x80..0x280) if output => ResourceKind::Varying { location: (attr - 0x80) / 16, flat: false },
            _ => return None,
        };
        Some(self.interface_component(kind, storage, attr))
    }
    pub fn cctl(&mut self, _inst: u128) {
        // Vulkan buffers are coherent without guest cache control.
    }
    pub fn cctll(&mut self, inst: u128) {
        self.cctl(inst)
    }
    pub fn cctlt(&mut self, _inst: u128) {}
    pub fn errbar(&mut self, _inst: u128) {}
    pub fn getlmembase(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let value = self.cached_const_u32(0xff00_0000);
        let ty = self.type_u64();
        let value = self.ir.emit_u_convert(ty, value);
        self.store_pair(rd, value);
    }
    pub fn ipa(&mut self, inst: u128) {
        use spirv::{BuiltIn, storage_class::INPUT};
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let attr = (((inst >> 64) & 0xff) << 2) as u32;
        let constant = (((inst >> 78) & 0x3) << 0) == 1;
        assert!(opcode(inst) == 0x326 && inst >> 76 & 3 == 0 && inst >> 78 & 3 < 2 && inst >> 81 & 7 == PT as u128, "only IPA.PASS and IPA.CONSTANT at the pixel centre are implemented");
        let Stage::Fragment { flat, .. } = self.stage else { panic!("IPA in a {:?} module is not implemented", self.stage) };
        let val = match attr {
            0x70..0x80 => {
                let coord = self.builtin(BuiltIn::FragCoord);
                let val = self.ir.emit_composite_extract(self.type_f32[1], coord, &[attr / 4 % 4]);
                self.bitcast_u32(val)
            }
            0x80..0x280 => {
                let location = (attr - 0x80) / 16;
                let kind = ResourceKind::Varying { location, flat: constant || flat >> location & 1 != 0 };
                let ptr = self.interface_component(kind, INPUT, attr);
                let val = self.ir.emit_load(self.type_f32[1], ptr);
                self.bitcast_u32(val)
            }
            0x3fc => {
                let front = self.builtin(BuiltIn::FrontFacing);
                let all = self.cached_const_u32(u32::MAX);
                self.ir.emit_select(self.type_u32[1], front, all, self.const_u32_0)
            }
            _ => panic!("IPA a[{attr:#x}] is not implemented"),
        };
        self.store_register(rd, val);
    }
    // ISBERD maps a vertex index to the handle ALD takes, which here is the index (NAK nak_nir_lower_vtg_io.c); the other forms
    // are mesh and task shader storage (NAK nak_nir_lower_mesh_intrinsics.c) apart from PATCH, a patch attribute load (eden).
    pub fn isberd(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 40) & 0xffff) << 0) as u32;
        let base = (((inst >> 76) & 0x3) << 0) as usize;
        let (skew, output) = (inst >> 78 & 1 != 0, inst >> 79 & 1 != 0);
        let form = format!("ISBERD{}.{}{}", if output { ".O" } else { "" }, ["MAP", "PATCH", "PRIM", "ATTR"][base], if skew { ".SKEW" } else { "" });
        let val = match (base, skew, output) {
            (0, false, false) => {
                let index = self.load_register(ra);
                let offset = self.cached_const_u32(ra_offset);
                self.ir.emit_iadd(self.type_u32[1], index, offset)
            }
            (1, false, _) if ra == RZ && inst >> 74 & 3 == 2 => {
                let ptr = self.attribute_component(ra_offset, true, output, self.const_u32_0);
                let ptr = ptr.unwrap_or_else(|| panic!("{form} [{ra_offset:#x}] in a {:?} module is not implemented", self.stage));
                let val = self.ir.emit_load(self.type_f32[1], ptr);
                self.bitcast_u32(val)
            }
            _ => panic!("{form} R{rd}, [R{ra}+{ra_offset:#x}] is not implemented"),
        };
        self.store_register(rd, val);
    }
    pub fn isbewr(&mut self, inst: u128) {
        let base = ["MAP", "INVALID1", "INVALID2", "ATTR"][(inst >> 76 & 3) as usize];
        panic!("ISBEWR.{base} writes mesh and task shader storage (NAK nak_nir_lower_mesh_intrinsics.c), which has no stage here");
    }
    pub fn ld(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 32);
        self.load_memory(ResourceKind::Global, address, sz, rd);
    }
    pub fn ldc(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let ra_offset = (((inst >> 38) & 0xffff) << 0) as u16 as i16 as u32;
        let sc_bank = (((inst >> 54) & 0x1f) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let stride = (((inst >> 78) & 0x3) << 0) as u32;
        const LDC_UR_BANK: u32 = 0x1582;
        let bindless = opcode(inst) == LDC_UR_BANK;
        let base = self.load_register(if bindless { (inst >> 64) as u32 & 0xff } else { ra });
        let mut bank = if bindless { self.load_ureg(ra & 0x3f) } else { self.cached_const_u32(sc_bank) };
        let base = if stride >= 2 {
            let sixteen = self.cached_const_u32(16);
            let segment = self.ir.emit_shift_right_logical(self.type_u32[1], base, sixteen);
            bank = self.ir.emit_iadd(self.type_u32[1], bank, segment);
            let mask = self.cached_const_u32(0xffff);
            self.ir.emit_bitwise_and(self.type_u32[1], base, mask)
        } else {
            base
        };
        for i in 0..access_words(sz) {
            let offset = self.cached_const_u32(ra_offset.wrapping_add(4 * i));
            let addr = self.ir.emit_iadd(self.type_u32[1], base, offset);
            let word = if bindless || stride >= 2 {
                self.load_dynamic_cbuf(bank, addr)
            } else {
                self.load_cbuf(ResourceKind::Cbuf(sc_bank), addr)
            };
            let value = if sz <= 4 { self.load_narrow(word, addr, sz) } else { word };
            self.store_register(nth_reg(rd, RZ, i), value);
        }
    }
    pub fn ldg(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 40);
        self.load_memory(ResourceKind::Global, address, sz, rd);
    }
    pub fn ldgdepbar(&mut self, _inst: u128) {
        // LDGSTS completes synchronously in this model.
    }
    pub fn ldl(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 32, 40);
        self.load_memory(ResourceKind::Local, address, sz, rd);
    }
    pub fn ldtram(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let attr = (((inst >> 64) & 0xff) << 2) as u32;
        let c = (((inst >> 72) & 0x1) << 0) as usize;
        let vertices = ["vertices 0 and 1", "vertex 2"][c];
        panic!("LDTRAM.{} R{rd}, a[{attr:#x}] reads the attribute at {vertices} of the primitive (NAK load_input_vertex), which needs VK_KHR_fragment_shader_barycentric and is not implemented", ["AB", "C"][c]);
    }
    pub fn membar(&mut self, inst: u128) {
        let dstfmt = (((inst >> 76) & 0x7) << 0) as u32;
        use spirv::memory_semantics::{ACQUIRE_RELEASE, UNIFORM_MEMORY, WORKGROUP_MEMORY};
        let (scope, semantics) = match dstfmt {
            0 | 1 => (spirv::scope::WORKGROUP, ACQUIRE_RELEASE | WORKGROUP_MEMORY),
            _ => (spirv::scope::DEVICE, ACQUIRE_RELEASE | UNIFORM_MEMORY | WORKGROUP_MEMORY),
        };
        let scope = self.cached_const_u32(scope);
        let semantics = self.cached_const_u32(semantics);
        self.ir.emit_memory_barrier(scope, semantics);
    }
    // OUT returns the handle of the next vertex for AST, which here writes the one current vertex of a geometry shader.
    pub fn out(&mut self, inst: u128) {
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let ra = (((inst >> 24) & 0xff) << 0) as u32;
        let (emit, cut) = (inst >> 78 & 1 != 0, inst >> 79 & 1 != 0);
        let other_stream = match opcode(inst) {
            0x324 => (inst >> 32 & 0xff) as u32 != RZ,
            0x1924 => (inst >> 32 & 0x3f) as u32 != URZ,
            _ => (inst >> 32) as u32 != 0,
        };
        assert!(matches!(self.stage, Stage::Geometry { .. }), "OUT in a {:?} module is not implemented", self.stage);
        assert!(!other_stream, "OUT to a vertex stream other than 0 is not implemented");
        let handle = self.load_register(ra);
        if emit {
            self.ir.emit_emit_vertex();
        }
        if cut {
            self.ir.emit_end_primitive();
        }
        self.store_register(rd, handle);
    }
    pub fn pixld(&mut self, inst: u128) {
        use spirv::{BuiltIn, storage_class::INPUT};
        let rd = (((inst >> 16) & 0xff) << 0) as u32;
        let mode = (((inst >> 78) & 0x7) << 0) as usize;
        let name = ["MSCOUNT", "COVMASK", "CENTROID_OFFSET", "MY_INDEX", "INNER_COVERAGE"].get(mode).unwrap_or(&"INVALID");
        assert!(matches!(self.stage, Stage::Fragment { .. }), "PIXLD in a {:?} module is not implemented", self.stage);
        assert!(inst >> 81 & 7 == PT as u128, "PIXLD.{name} into a predicate is not implemented");
        let val = match mode {
            1 => {
                let mask = self.interface_var(ResourceKind::BuiltIn(BuiltIn::SampleMask as u32), INPUT);
                let ptr = self.ir.emit_type_pointer(INPUT, self.type_u32[1]);
                let ptr = self.ir.emit_access_chain(ptr, mask, &[self.const_u32_0]);
                self.ir.emit_load(self.type_u32[1], ptr)
            }
            3 => self.builtin(BuiltIn::SampleId),
            _ => panic!("PIXLD.{name} is not implemented"),
        };
        self.store_register(rd, val);
    }
    pub fn setlmembase(&mut self, _inst: u128) {}
    pub fn st(&mut self, inst: u128) {
        let rc = (((inst >> 64) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 32);
        self.store_memory(ResourceKind::Global, address, sz, rc);
    }
    pub fn stg(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 40);
        self.store_memory(ResourceKind::Global, address, sz, rb);
    }
    pub fn stl(&mut self, inst: u128) {
        let rb = (((inst >> 32) & 0xff) << 0) as u32;
        let sz = (((inst >> 73) & 0x7) << 0) as u32;
        let address = self.global_address(inst, 64, 40);
        self.store_memory(ResourceKind::Local, address, sz, rb);
    }
}
