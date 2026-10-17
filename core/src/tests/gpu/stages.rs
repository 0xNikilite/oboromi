#[cfg(test)]
mod tests {
    use crate::gpu::sm86::{ResourceKind, Stage, emit_module};
    use crate::tests::gpu::harness::spirv_val;
    use crate::gpu::spirv::BuiltIn as B;
    use crate::gpu::threed::PrimitiveTopology;
    use crate::tests::gpu_test::inst::*;

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
        assert_eq!(stage_interface(Stage::Vertex, &vertex_program()), expected);
    }

    #[test]
    fn fragment_program_reads_varying_and_writes_colour() {
        let expected = [ResourceKind::Varying { location: 0, flat: false }, ResourceKind::FragmentOutput { location: 0 }];
        assert_eq!(stage_interface(Stage::Fragment { flat: 0, screen_linear: 0, depth: false }, &fragment_program()), expected);
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
        let program = [s2r(1, 17), isberd(2, 1, 0), ald_vertex(4, 0x70, 4, 2), ast_vertex(0x70, 4, 4, 1), ast(0x0, 4, 4) | ATTR_PATCH, ast(0x10, 4, 2) | ATTR_PATCH, ast(0x20, 4, 1) | ATTR_PATCH, exit()];
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
    fn fragment_program_reads_sample_id_and_coverage() {
        let expected = [ResourceKind::BuiltIn(B::SampleId as u32), ResourceKind::BuiltIn(B::SampleMask as u32), ResourceKind::FragmentOutput { location: 0 }];
        let program = [pixld(0, PIXLD_MY_INDEX), pixld(1, PIXLD_COVMASK), exit()];
        assert_eq!(stage_interface(Stage::Fragment { flat: 0, screen_linear: 0, depth: false }, &program), expected);
    }
}
