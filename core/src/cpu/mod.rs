use std::alloc::Layout;
use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::hint::cold_path;
use std::marker::PhantomPinned;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::num::NonZero;
use std::pin::Pin;
use std::ptr::NonNull;
use unicorn_engine_sys::{uc_error, RegisterARM64};
use crate::u64_to_usize;


#[derive(Debug, thiserror::Error)]
pub enum CpuInitError {
    #[error("out of memory.")]
    OutOfMemory,
    #[error("emulator does not supported on this system.")]
    VersionUnsupported,

    #[error("unidentified error `{0}`.")]
    Unidentified(uc_error),
}


#[derive(Debug, thiserror::Error)]
#[error("invalid memory access")]
pub struct MemoryAccessViolation(());


#[derive(Debug, Copy, Clone)]
pub enum Fault {
    Segmentation,
    UnalignedAccess,
    InvalidInstruction,
    UnhandledException,
    /// Likely an OOM caused by JIT
    InsufficientResources,
}

impl Fault {
    pub fn from_uc_error(error: uc_error) -> Result<(), Fault> {
        match error {
            uc_error::OK => Ok(()),

            uc_error::READ_UNMAPPED
            | uc_error::WRITE_UNMAPPED
            | uc_error::FETCH_UNMAPPED
            | uc_error::READ_PROT
            | uc_error::WRITE_PROT
            | uc_error::FETCH_PROT => {
                cold_path();
                Err(Fault::Segmentation)
            }

            uc_error::READ_UNALIGNED
            | uc_error::WRITE_UNALIGNED
            | uc_error::FETCH_UNALIGNED => {
                cold_path();
                Err(Fault::UnalignedAccess)
            }

            uc_error::INSN_INVALID => {
                cold_path();
                Err(Fault::InvalidInstruction)
            }

            uc_error::RESOURCE => {
                cold_path();
                // if you are debugging this,
                // make sure uc_emu_start isn't called recursively with a high amount of depth
                Err(Fault::InsufficientResources)
            }


            uc_error::EXCEPTION => {
                cold_path();
                Err(Fault::UnhandledException)
            }

            _ => {
                cold_path();
                panic!("unknown execution error `{error}`")
            }
        }
    }
}


#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum CoreIndex {
    _0, _1, _2, _3,
    _4, _5, _6, _7,
}

impl CoreIndex {
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0..8 => Some(unsafe { std::mem::transmute::<u8, Self>(index) }),
            _ => None
        }
    }

    pub const fn get_index(self) -> u8 {
        self as u8
    }

    fn all() -> impl Iterator<Item=Self> {
        (0..).map_while(Self::from_index)
    }
}

impl Debug for CoreIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Core").field(&self.get_index()).finish()
    }
}


// 12GB Memory
pub const MEMORY_SIZE: u64 = 12 * 1024 * 1024 * 1024;
pub const PAGE_SIZE: u64 = 4 * 1024;
pub const STACK_SIZE: u64 = 2048 * PAGE_SIZE;
// reserve a full stack, and a page at the end to catch stack overflows
pub const STACK_RESERVATION: u64 = STACK_SIZE + PAGE_SIZE;


// https://github.com/unicorn-engine
// Unicorn claims Thread-safety by design
pub struct CpuCore(NonNull<unicorn_engine_sys::uc_engine>);

unsafe impl Send for CpuCore {}
unsafe impl Sync for CpuCore {}


impl CpuCore {
    pub const COUNT: u8 = 8;
}

impl Drop for CpuCore {
    fn drop(&mut self) {
        unsafe { unicorn_engine_sys::uc_close(self.0.as_ptr()) };
    }
}



// constructors
impl CpuCore {
    fn build(
        builder: impl FnOnce(
            NonNull<unicorn_engine_sys::uc_engine>
        ) -> Result<(), uc_error>,
    ) -> Result<Self, CpuInitError> {
        let builder = move || {
            let mut handle = std::ptr::null_mut();
            unsafe {
                Result::from(unicorn_engine_sys::uc_open(
                    unicorn_engine_sys::Arch::ARM64,
                    unicorn_engine_sys::Mode::LITTLE_ENDIAN,
                    &raw mut handle
                ))?
            }

            let handle = NonNull::new(handle).ok_or(uc_error::OK)?;
            builder(handle)?;
            Ok(handle)
        };

        match builder() {
            Ok(handle) => Ok(Self(handle)),
            Err(uc_error::ARCH | uc_error::MODE | uc_error::VERSION) => {
                Err(CpuInitError::VersionUnsupported)
            },
            Err(uc_error::NOMEM) => Err(CpuInitError::OutOfMemory),
            Err(err) => Err(CpuInitError::Unidentified(err)),
        }
    }

    /// # Safety
    ///
    /// * system_memory must live for as long as the returned core is alive
    unsafe fn internal_core(
        stack_pointer: u64,
        system_memory: Pin<&SystemMemory>
    ) -> Result<Self, CpuInitError> {
        const { assert!(MEMORY_SIZE.is_multiple_of(PAGE_SIZE)) }

        debug_assert!(stack_pointer.is_multiple_of(PAGE_SIZE));

        Self::build(move |handle| {
            unsafe {
                Result::from(unicorn_engine_sys::uc_mem_map_ptr(
                    handle.as_ptr(),
                    0,
                    MEMORY_SIZE,
                    u32::from(unicorn_engine_sys::Prot::ALL.0),
                    system_memory.get_ptr().cast()
                ))?;


                // FIXME currently doing requires us to break up the allocation;
                //
                // // the page directly after the stack should have 0 memory protections,
                // // just so stack overflows are detected
                // Result::from(unicorn_engine_sys::uc_mem_protect(
                //     handle.as_ptr(),
                //     stack_pointer.unchecked_sub(STACK_SIZE),
                //     PAGE_SIZE,
                //     unicorn_engine_sys::Prot::NONE.0,
                // ))?;
                //
                // // the page at null should also have
                // // no memory protections to catch null dereferences
                // Result::from(unicorn_engine_sys::uc_mem_protect(
                //     handle.as_ptr(),
                //     0,
                //     PAGE_SIZE,
                //     u32::from(unicorn_engine_sys::Prot::NONE.0),
                // ))?;


                Result::from(unicorn_engine_sys::uc_reg_write(
                    handle.as_ptr(),
                    i32::from(RegisterARM64::SP),
                    (&raw const stack_pointer).cast()
                ))
            }
        })
    }

    // FIXME
    // #[cfg(test)]
    pub fn new_mock(memory_size: u64) -> Self {
        let result = Self::build(|handle| {
            unsafe {
                Result::from(unicorn_engine_sys::uc_mem_map(
                    handle.as_ptr(),
                    0,
                    memory_size,
                    u32::from(unicorn_engine_sys::Prot::ALL.0)
                ))
            }
        });

        result.expect("failed to build mock cpu core")
    }
}

// register manipulation

const X_REGISTERS: [RegisterARM64; 31] = [
    RegisterARM64::X0,
    RegisterARM64::X1,
    RegisterARM64::X2,
    RegisterARM64::X3,
    RegisterARM64::X4,
    RegisterARM64::X5,
    RegisterARM64::X6,
    RegisterARM64::X7,
    RegisterARM64::X8,
    RegisterARM64::X9,
    RegisterARM64::X10,
    RegisterARM64::X11,
    RegisterARM64::X12,
    RegisterARM64::X13,
    RegisterARM64::X14,
    RegisterARM64::X15,
    RegisterARM64::X16,
    RegisterARM64::X17,
    RegisterARM64::X18,
    RegisterARM64::X19,
    RegisterARM64::X20,
    RegisterARM64::X21,
    RegisterARM64::X22,
    RegisterARM64::X23,
    RegisterARM64::X24,
    RegisterARM64::X25,
    RegisterARM64::X26,
    RegisterARM64::X27,
    RegisterARM64::X28,
    RegisterARM64::X29,
    RegisterARM64::X30,
];

impl CpuCore {
    fn load_from_reg(&self, reg: RegisterARM64) -> u64 {
        unsafe {
            let mut value = 0_u64;
            let load_result = unicorn_engine_sys::uc_reg_read(
                self.0.as_ptr(),
                i32::from(reg),
                (&raw mut value).cast()
            );

            load_result.and(Ok(value)).unwrap_or_else(|_| {
                panic!("failed to load register {reg:?}")
            })
        }
    }

    pub fn load_pc(&self) -> u64 {
        self.load_from_reg(RegisterARM64::PC)
    }

    pub fn load_sp(&self) -> u64 {
        self.load_from_reg(RegisterARM64::SP)
    }

    pub fn load_x<const X: usize>(&self) -> u64 {
        self.load_from_reg(const { X_REGISTERS[X] })
    }

    fn store_to_reg(&self, reg: RegisterARM64, value: u64) {
        unsafe {
            let store_result = unicorn_engine_sys::uc_reg_write(
                self.0.as_ptr(),
                i32::from(reg),
                (&raw const value).cast()
            );

            Result::from(store_result).unwrap_or_else(|_| {
                panic!("failed to store register {reg:?}")
            })
        }
    }

    pub fn store_pc(&self, pc: u64) {
        self.store_to_reg(RegisterARM64::PC, pc)
    }

    pub fn store_sp(&self, sp: u64) {
        self.store_to_reg(RegisterARM64::SP, sp)
    }

    pub fn store_x<const X: usize>(&self, value: u64) {
        self.store_to_reg(const { X_REGISTERS[X] }, value)
    }
}



impl CpuCore {
    pub fn execute_at_ext(
        &self,
        start: u64,
        count: Option<NonZero<usize>>
    ) -> Result<(), Fault> {
        unsafe {
            // no timeout
            let timeout = 0;
            // Safety:
            // when count = None; we need count to be 0 which means no limit
            // when count = Some(x) we need count to be x which emans execute x steps
            // so that means the actual bits of count are what we want
            // and Option<NonZero<T>> and T always have the exact same layout
            let count = std::mem::transmute::<Option<NonZero<usize>>, usize>(count);
            Fault::from_uc_error(unicorn_engine_sys::uc_emu_start(
                self.0.as_ptr(),
                start,
                u64::MAX,
                timeout,
                count
            ))
        }
    }

    pub fn execute_at(&self, start: u64) -> Result<(), Fault> {
        self.execute_at_ext(start, None)
    }

    pub fn step(&self) -> Result<(), Fault> {
        let steps = const { NonZero::new(1).unwrap() };
        let pc = self.load_pc();
        self.execute_at_ext(pc, Some(steps))
    }
}


impl CpuCore {
    fn mem_op(
        len: usize,
        op: impl FnOnce(u64) -> uc_error
    ) -> Result<(), MemoryAccessViolation> {
        let len = u64::try_from(len).map_err(|_| MemoryAccessViolation(()))?;
        match op(len) {
            uc_error::OK => Ok(()),
            _ => Err(MemoryAccessViolation(()))
        }
    }

    pub fn write_mem(&self, addr: u64, mem: &[u8]) -> Result<(), MemoryAccessViolation> {
        Self::mem_op(
            mem.len(),
            |len| unsafe {
                unicorn_engine_sys::uc_mem_write(
                    self.0.as_ptr(),
                    addr,
                    mem.as_ptr().cast(),
                    len,
                )
            }
        )
    }

    pub fn read_mem(&self, addr: u64, mem: &mut [u8]) -> Result<(), MemoryAccessViolation> {
        Self::mem_op(
            mem.len(),
            |len| unsafe {
                unicorn_engine_sys::uc_mem_read(
                    self.0.as_ptr(),
                    addr,
                    mem.as_mut_ptr().cast(),
                    len,
                )
            }
        )
    }
}


macro_rules! impl_read_write_ops {
    ($($ty: ty: { read: $read:ident, write: $write: ident }),+ $(,)?) => {
        impl CpuCore {
            $(
            pub fn $read(&self, addr: u64) -> Result<$ty, MemoryAccessViolation> {
                let mut bytes = [0; size_of::<$ty>()];
                self.read_mem(addr, &mut bytes)?;
                Ok(<$ty>::from_le_bytes(bytes))
            }

            pub fn $write(&self, addr: u64, value: $ty) -> Result<(), MemoryAccessViolation> {
                let bytes = value.to_le_bytes();
                self.write_mem(addr, &bytes)
            }
            )*
        }
    };
}

impl_read_write_ops! {
    u32: { read: read_u32, write: write_u32 },
    u64: { read: read_u64, write: write_u64 },
}


#[repr(transparent)]
struct SystemMemory {
    ram: UnsafeCell<[u8; u64_to_usize(MEMORY_SIZE)]>,
    _pinned: PhantomPinned
}

impl SystemMemory {
    pub fn alloc_new() -> Option<Pin<Box<Self>>> {
        const { assert!(size_of::<Self>() != 0) }
        // Safety: size is not zero
        let ptr = unsafe { std::alloc::alloc_zeroed(Layout::new::<Self>()) };
        // if ptr is null, we most likely hit OOM
        let ptr = NonNull::new(ptr)?.cast::<Self>();
        let this = unsafe { Box::from_raw(ptr.as_ptr()) };
        Some(Box::into_pin(this))
    }

    pub fn get_ptr(self: Pin<&Self>) -> *mut u8 {
        self.ram.get().cast::<u8>()
    }
}

pub struct Cpu {
    cores: [CpuCore; CpuCore::COUNT as usize],
    // hold this to keep the memory from deallocating
    // and it should keep being the last feild, so that it is the last thing dropped
    // (the last field is dropped last) https://doc.rust-lang.org/reference/destructors.html
    #[allow(dead_code)]
    shared_memory: Pin<Box<SystemMemory>>,
}

impl Cpu {
    pub fn new() -> Result<Self, CpuInitError> {
        // Allocate 12GB of zeroed memory
        // note: on modern OSs, this is **usually** lazily allocated (virtual memory)
        // and won't consume physical RAM until written to.

        // Safety: u8 can be zeroed safely

        let Some(shared_memory) = SystemMemory::alloc_new() else {
            return Err(CpuInitError::OutOfMemory);
        };

        struct CoresGuard {
            cores: [MaybeUninit<CpuCore>; CpuCore::COUNT as usize],
            initialized: usize
        }

        impl CoresGuard {
            pub unsafe fn finish(self) -> [CpuCore; CpuCore::COUNT as usize] {
                debug_assert!(self.initialized == self.cores.len());
                let this = ManuallyDrop::new(self);
                unsafe { std::mem::transmute_copy(&this.cores) }
            }
        }

        impl Drop for CoresGuard {
            fn drop(&mut self) {
                unsafe { self.cores[..self.initialized].assume_init_drop() }
            }
        }

        let mut guard = CoresGuard {
            cores: [const { MaybeUninit::uninit() }; CpuCore::COUNT as usize],
            initialized: 0
        };

        for core in CoreIndex::all() {
            let idx = core.get_index();
            let stack_address = {
                u64::from(core.get_index()).checked_mul(STACK_SIZE + PAGE_SIZE)
                    .and_then(|offset| MEMORY_SIZE.checked_sub(offset))
            };

            let stack_address = stack_address.filter(|&sp| sp != 0).unwrap();

            debug_assert_eq!(guard.initialized, idx as usize);
            let cpu_core = unsafe { CpuCore::internal_core(stack_address, shared_memory.as_ref())? };
            guard.cores[idx as usize].write(cpu_core);
            guard.initialized += 1;
        }

        let cores = unsafe { guard.finish() };

        Ok(Self {
            cores,
            shared_memory,
        })
    }

    pub fn get_core(&self, core: CoreIndex) -> &CpuCore {
        &self.cores[core.get_index() as usize]
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_memory_access() {
        println!("Testing shared memory between cores...");
        let cpu = Cpu::new().unwrap();

        let core0 = cpu.get_core(CoreIndex::_0);
        let core1 = cpu.get_core(CoreIndex::_1);

        // Write value using Core 0
        let test_addr = 0x1000;
        let test_val = 0xDEADBEEF;
        println!("Core 0 writing {:#x} to {:#x}", test_val, test_addr);
        core0.write_u32(test_addr, test_val).unwrap();

        // Read value using Core 1
        let read_val = core1.read_u32(test_addr).unwrap();
        println!("Core 1 read {:#x} from {:#x}", read_val, test_addr);

        assert_eq!(read_val, test_val, "Core 1 should see value written by Core 0");
    }
}
