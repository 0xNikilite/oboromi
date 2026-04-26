#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "emulators require precise bit-level accuracy"
)]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("oboromi requires a 64-bit architecture to emulate 12GB of RAM.");

pub mod cpu;
pub mod fs;
pub mod gpu;
pub mod tests;
pub mod nn;
pub mod sys;
