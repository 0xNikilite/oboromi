#[cfg(test)]
mod tests {
    use crate::gpu::memory::{MemoryError, PAGE, PageMap};

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
}
