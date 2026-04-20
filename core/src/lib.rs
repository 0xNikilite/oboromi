#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(
    // F32 -> F64, Int -> Float, etc.
    clippy::cast_precision_loss,
    // U32 -> U8, etc.
    clippy::cast_possible_truncation,
    // Signed -> Unsigned, etc.
    clippy::cast_possible_wrap,
    // Signed -> Unsigned
    clippy::cast_sign_loss,
    reason = "emulators require precise bit-level accuracy; \
              implicit casts can introduce subtle, hard-to-debug architectural discrepancies"
)]


const _: () = assert!(
    usize::BITS >= 64,
    "oboromi requires a at least a 64-bit architecture to emulate 12GB of RAM."
);

#[allow(clippy::cast_possible_truncation, reason = "usize is at least 64 bits")]
pub const fn u64_to_usize(x: u64) -> usize {
    x as usize
}

pub mod cpu;
pub mod fs;
pub mod gpu;
pub mod tests;
pub mod nn;
pub mod sys;
