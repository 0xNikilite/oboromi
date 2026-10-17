#[cfg(test)]
mod tests {
    use crate::gpu::memory::{MemoryError, MemoryTracker, PAGE, PageMap, Writer};
    use crate::gpu::{VkError, device};

    const VA: u64 = 0x10_0000_0000;

    #[test]
    fn resolve_across_a_page_boundary_returns_both_pages_in_va_order() {
        let mut space = PageMap::default();
        space.map(VA, 3 * PAGE, PAGE);
        space.map(VA + PAGE, PAGE, PAGE);
        let ranges: Vec<_> = space.resolve(VA + PAGE - 16, 32).unwrap().collect();
        assert_eq!(ranges, [4 * PAGE - 16..4 * PAGE, PAGE..PAGE + 16]);
    }

    #[test]
    fn resolve_of_an_unmapped_page_is_an_error() {
        let mut space = PageMap::default();
        space.map(VA, 0, 2 * PAGE);
        space.unmap(VA + PAGE, PAGE);
        assert!(matches!(space.resolve(VA + 8, PAGE), Err(MemoryError::Unmapped(va)) if va == VA + PAGE));
    }

    #[test]
    fn remapping_a_va_changes_the_bytes_its_window_holds() {
        let state = match device() {
            Err(e @ (VkError::Loader | VkError::NoDevice)) => return eprintln!("skipping: {e:?}"),
            state => state.unwrap(),
        };
        let mut ram = vec![0xaa; 2 * PAGE as usize];
        ram[PAGE as usize..].fill(0xbb);
        let (mut space, mut tracker) = (PageMap::default(), MemoryTracker::default());
        let mut window_bytes = |space: &PageMap| {
            let (mapping, base) = tracker.window(state, &ram, space, VA + 4, 8).unwrap();
            let mut bytes = [0; 12];
            mapping.buffer.read(&mut bytes).unwrap();
            assert_eq!(base, VA);
            bytes
        };
        space.map(VA, 0, PAGE);
        assert_eq!(window_bytes(&space), [0xaa; 12]);
        space.unmap(VA, PAGE);
        space.map(VA, PAGE, PAGE);
        assert_eq!(window_bytes(&space), [0xbb; 12]);
    }

    #[test]
    fn generations_advance_on_cpu_and_gpu_writes_and_follow_a_reused_page() {
        let mut tracker = MemoryTracker::default();
        tracker.mark_cpu_written(PAGE, 4);
        let cpu = tracker.generation(0, 2 * PAGE);
        tracker.mark_gpu_written(PAGE + 8, 4);
        let gpu = tracker.generation(PAGE, 1);
        assert!(gpu > cpu && cpu > 0);
        assert_eq!(tracker.writer(PAGE), Some(Writer::Gpu));
        tracker.mark_cpu_written(PAGE, PAGE);
        assert!(tracker.generation(PAGE, PAGE) > gpu);
        assert_eq!(tracker.writer(PAGE), Some(Writer::Cpu));
        assert_eq!(tracker.generation(0, PAGE), 0);
    }
}
