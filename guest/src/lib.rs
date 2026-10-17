//! The guest program's side of Horizon: SVC wrappers, the CMIF message layout, the hid start-up and the nvdrv ioctl
//! arguments.
#![no_std]

pub mod cmif;
#[cfg(target_arch = "aarch64")]
pub mod hid;
pub mod nvdrv;
#[cfg(target_arch = "aarch64")]
pub mod svc;
