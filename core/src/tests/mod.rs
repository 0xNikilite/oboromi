pub mod run;
pub mod multicore_test;
pub mod gpu;
#[cfg(test)]
mod audio;
pub mod gpu_test;
#[cfg(test)]
mod kernel;

pub use run::run_tests;
pub use gpu_test::run_gpu_tests;

#[cfg(test)]
mod threads;
