use memmap2::Mmap;
use std::fs;
use std::ops::Deref;
use std::path::Path;

pub struct File {
    map: Mmap,
}

impl File {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error>
where {
        let file = fs::File::open(path)?;
        let map = unsafe { Mmap::map(&file)? };
        Ok(Self { map })
    }
}

impl Deref for File {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
