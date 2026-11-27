mod ampere_dma;
pub mod run;
pub mod gpu_test;

pub use run::run_tests;
pub use gpu_test::run_gpu_tests;

#[cfg(test)]
mod suites;
