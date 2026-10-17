//! The machine's guest-physical RAM: one anonymous mapping every CPU core maps at guest address 0 and the GPU reads
//! and writes through its page map.
use memmap2::MmapMut;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct GuestRam {
    base: *mut u8,
    len: usize,
    _map: MmapMut,
    /// Held while a core executes and while the GPU runs, so CPU and GPU never touch RAM at once.
    turn: Mutex<()>,
}

unsafe impl Send for GuestRam {}
unsafe impl Sync for GuestRam {}

impl GuestRam {
    /// Lazily committed: a page costs memory once something touches it.
    pub fn new(bytes: usize) -> Arc<Self> {
        let mut map = MmapMut::map_anon(bytes).expect("guest RAM mapping");
        Arc::new(Self { base: map.as_mut_ptr(), len: bytes, _map: map, turn: Mutex::default() })
    }

    pub fn size(&self) -> usize {
        self.len
    }

    pub fn base(&self) -> *mut u8 {
        self.base
    }

    pub fn turn(&self) -> MutexGuard<'_, ()> {
        self.turn.lock().unwrap()
    }

    /// # Safety
    /// Cores write through `base` while they run, so a slice lives only inside one turn and is dropped before the
    /// next access to its bytes, and no slice of those bytes is alive while a `slice_mut` of them is.
    pub unsafe fn slice(&self, pa: u64, len: usize) -> &[u8] {
        assert!(pa as usize + len <= self.len, "{pa:#x} + {len:#x} is outside guest RAM");
        unsafe { std::slice::from_raw_parts(self.base.add(pa as usize), len) }
    }

    /// # Safety
    /// As for `slice`.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn slice_mut(&self, pa: u64, len: usize) -> &mut [u8] {
        assert!(pa as usize + len <= self.len, "{pa:#x} + {len:#x} is outside guest RAM");
        unsafe { std::slice::from_raw_parts_mut(self.base.add(pa as usize), len) }
    }

    pub fn read(&self, pa: u64, out: &mut [u8]) {
        let _turn = self.turn();
        out.copy_from_slice(unsafe { self.slice(pa, out.len()) });
    }

    pub fn write(&self, pa: u64, bytes: &[u8]) {
        let _turn = self.turn();
        unsafe { self.slice_mut(pa, bytes.len()) }.copy_from_slice(bytes);
    }
}
