#![allow(unused_imports)]
pub mod nca;
pub mod nro;

pub use nca::{
    BootLoader, BootPackageInfo, Hfs0Archive, NcaImage, Pfs0Archive, Pfs0FileEntry, XciCartridge,
    HFS0_MAGIC, NCA3_MAGIC, PFS0_MAGIC, XCI_HEAD_MAGIC,
};
pub use nro::{LoadError, NroHeader, NroLoader};
