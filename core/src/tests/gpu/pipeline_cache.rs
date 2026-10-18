#[cfg(test)]
mod tests {
    use crate::gpu::compute::ComputePipeline;
    use crate::gpu::pipeline_cache::{Disk, Lookup, PipelineBuilder, PipelinePolicy};
    use crate::gpu::sm86::ResourceKind;
    use crate::tests::gpu::graphics::{SIZE, triangle};
    use crate::tests::gpu::harness::{shared_device, translate_module, vk_state};
    use crate::tests::gpu_test::inst;
    use ash::vk::DescriptorType;
    use std::sync::mpsc;

    const REGISTER_IO: [(u32, DescriptorType); 2] = [(10, DescriptorType::STORAGE_BUFFER), (11, DescriptorType::STORAGE_BUFFER)];

    #[test]
    fn builder_skips_the_draw_until_its_pipeline_is_built_and_waits_when_told_to() {
        let Some(state) = shared_device() else { return };
        let words = translate_module(&[inst::mov_imm(0, 42)], true, Some(state), false).words();
        let build = |gate: Option<mpsc::Receiver<()>>| {
            let words = words.clone();
            move || {
                gate.map(|gate| gate.recv());
                ComputePipeline::new(state, &words, &REGISTER_IO)
            }
        };
        let (release, gate) = mpsc::channel();
        let mut builder = PipelineBuilder::new(state, PipelinePolicy::SkipDraw, None).unwrap();
        assert!(matches!(builder.get_or_request(&0, || build(Some(gate))), Ok(Lookup::Pending)));
        assert!(matches!(builder.get_or_request(&0, || build(None)), Ok(Lookup::Pending)));
        release.send(()).unwrap();
        assert!(matches!(builder.wait(&0), Ok(Lookup::Ready(_))));
        builder.policy = PipelinePolicy::Wait;
        assert!(matches!(builder.get_or_request(&1, || build(None)), Ok(Lookup::Ready(_))));
    }

    #[test]
    fn disk_serves_the_saved_spirv_and_a_pipeline_cache_this_device_accepts() {
        let Some(state) = shared_device() else { return };
        let device = unsafe { state.instance.get_physical_device_properties(state.physical_device) };
        let (root, title) = (std::env::temp_dir().join(format!("oboromi-disk-{}", std::process::id())), "0100000000010000");
        let words = translate_module(&[inst::mov_imm(0, 42)], true, Some(state), false).words();
        let disk = Disk::open(&root, title, &device).unwrap();
        let resources = [ResourceKind::Cbuf(2), ResourceKind::Varying { location: 3, flat: true }];
        disk.store_spirv(7, &words, &resources, true).unwrap();
        let mut builder = PipelineBuilder::new(state, PipelinePolicy::Wait, Some(disk)).unwrap();
        let build = || {
            let words = words.clone();
            move || ComputePipeline::new(state, &words, &REGISTER_IO)
        };
        assert!(matches!(builder.get_or_request(&0, build), Ok(Lookup::Ready(_))));
        builder.flush().unwrap();

        let disk = Disk::open(&root, title, &device).unwrap();
        assert_eq!(disk.spirv(7), Some((words, resources.to_vec(), true)));
        assert!(disk.pipeline_cache_data().is_some());
        disk.load(state).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn triangle_linked_from_pipeline_libraries_matches_the_monolithic_one() {
        let Some(mut state) = vk_state() else { return };
        if !state.graphics_pipeline_library {
            return eprintln!("skipping: no VK_EXT_graphics_pipeline_library");
        }
        let mut pixels = [(); 2].map(|_| vec![0; (SIZE * SIZE * 4) as usize]);
        for (linked, pixels) in [true, false].into_iter().zip(&mut pixels) {
            state.graphics_pipeline_library = linked;
            triangle(&state).unwrap()(pixels).unwrap();
        }
        assert!(pixels[0] == pixels[1]);
    }
}
