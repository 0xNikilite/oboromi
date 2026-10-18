#[cfg(test)]
mod tests {
    use crate::gpu::sm86::{Hints, ResourceKind, Stage, emit_module};
    use crate::tests::gpu::harness::spirv_val;
    use crate::tests::gpu_test::inst::{ald, ast, exit, fmul, ipa, mov_cbuf};

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
        let module = emit_module(None, stage, Hints::default(), |decoder| {
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
        assert_eq!(stage_interface(Stage::Vertex { uint: 0, sint: 0 }, &vertex_program()), expected);
    }

    #[test]
    fn fragment_program_reads_varying_and_writes_colour() {
        let expected = [ResourceKind::Varying { location: 0, flat: false }, ResourceKind::FragmentOutput { location: 0 }];
        assert_eq!(stage_interface(Stage::Fragment { flat: 0, screen_linear: 0, depth: false }, &fragment_program()), expected);
    }
}
