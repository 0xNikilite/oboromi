//! The machine's guest-physical RAM: one anonymous mapping every CPU core maps at guest address 0 and the GPU reads
//! and writes through its page map.
use crate::gpu::memory::{PAGE, pages};
use memmap2::MmapMut;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct GuestRam {
    base: *mut u8,
    len: usize,
    _map: MmapMut,
    /// Held while a core executes and while the GPU runs, so CPU and GPU never touch RAM at once.
    turn: Mutex<()>,
    /// One bit per PAGE the CPU wrote since the GPU last took them.
    cpu_written: Box<[AtomicU64]>,
    /// The guest-physical ranges the GPU maps, whose CPU writes every core hooks.
    watched: Mutex<Vec<Range<u64>>>,
}

unsafe impl Send for GuestRam {}
unsafe impl Sync for GuestRam {}

impl GuestRam {
    pub fn new(bytes: usize) -> Arc<Self> {
        let mut map = MmapMut::map_anon(bytes).expect("guest RAM mapping");
        let cpu_written = (0..(bytes as u64).div_ceil(64 * PAGE)).map(|_| AtomicU64::new(0)).collect();
        Arc::new(Self { base: map.as_mut_ptr(), len: bytes, _map: map, turn: Mutex::default(), cpu_written, watched: Mutex::default() })
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
        self.mark_cpu_written(pa, bytes.len() as u64);
    }

    pub fn mark_cpu_written(&self, pa: u64, len: u64) {
        for page in pages(pa, len) {
            self.cpu_written[(page / 64) as usize].fetch_or(1 << (page % 64), Ordering::Relaxed);
        }
    }

    /// The pages written since the last call, by guest-physical address.
    pub fn take_cpu_written(&self) -> impl Iterator<Item = u64> + '_ {
        (0..).zip(&self.cpu_written).flat_map(|(word, bits)| {
            let bits = bits.swap(0, Ordering::Relaxed);
            (0..64).filter(move |bit| bits >> bit & 1 != 0).map(move |bit| (word * 64 + bit) * PAGE)
        })
    }

    pub fn watch(&self, range: Range<u64>) {
        let mut watched = self.watched();
        if !range.is_empty() && !watched.contains(&range) {
            watched.push(range);
        }
    }

    pub fn watched(&self) -> MutexGuard<'_, Vec<Range<u64>>> {
        self.watched.lock().unwrap()
    }
}
