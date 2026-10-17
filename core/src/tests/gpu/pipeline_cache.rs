#[cfg(test)]
mod tests {
    use crate::gpu::compute::ComputePipeline;
    use crate::gpu::pipeline_cache::{Lookup, PipelineBuilder, PipelinePolicy};
    use crate::gpu::{VkError, VkState, device};
    use crate::tests::gpu::harness::translate_module;
    use crate::tests::gpu_test::inst;
    use ash::vk::DescriptorType;
    use std::sync::mpsc;

    fn shared_device() -> Option<&'static VkState> {
        match device() {
            Err(e @ (VkError::Loader | VkError::NoDevice)) => {
                eprintln!("skipping: {e:?}");
                None
            }
            state => Some(state.unwrap()),
        }
    }

    #[test]
    fn builder_skips_the_draw_until_its_pipeline_is_built_and_waits_when_told_to() {
        let Some(state) = shared_device() else { return };
        let words = translate_module(&[inst::mov_imm(0, 42)], true, Some(state)).words();
        let build = |gate: Option<mpsc::Receiver<()>>| {
            let words = words.clone();
            move || {
                gate.map(|gate| gate.recv());
                ComputePipeline::new(state, &words, &[(10, DescriptorType::STORAGE_BUFFER), (11, DescriptorType::STORAGE_BUFFER)])
            }
        };
        let (release, gate) = mpsc::channel();
        let mut builder = PipelineBuilder::new(PipelinePolicy::SkipDraw);
        assert!(matches!(builder.get_or_request(&0, || build(Some(gate))), Ok(Lookup::Pending)));
        assert!(matches!(builder.get_or_request(&0, || build(None)), Ok(Lookup::Pending)));
        release.send(()).unwrap();
        assert!(matches!(builder.wait(&0), Ok(Lookup::Ready(_))));
        builder.policy = PipelinePolicy::Wait;
        assert!(matches!(builder.get_or_request(&1, || build(None)), Ok(Lookup::Ready(_))));
    }
}
