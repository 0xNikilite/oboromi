//! The guest program's side of Horizon: SVC wrappers, the CMIF message layout and the nvdrv ioctl arguments.
#![no_std]

pub mod cmif;
pub mod nvdrv;
#[cfg(target_arch = "aarch64")]
pub mod nv;
#[cfg(target_arch = "aarch64")]
pub mod svc;
