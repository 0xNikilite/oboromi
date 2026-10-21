use memmap2::Mmap;
use std::fs;
use std::path::Path;
use std::ops::Deref;

pub struct File {
    map: Option<Mmap>,
}

impl File {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error>
    where
    {
        let file = fs::File::open(path)?;
        if file.metadata()?.len() == 0 {
            return Ok(Self { map: None });
        }

        let map = unsafe { Mmap::map(&file)? };
        Ok(Self { map: Some(map) })
    }
}

impl Deref for File {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match &self.map {
            Some(map) => map,
            None => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::File;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FILE_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestFile(PathBuf);

    impl TestFile {
        fn new(contents: &[u8]) -> Self {
            let id = NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("oboromi-fs-test-{}-{id}.bin", std::process::id()));
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .expect("create test file");
            file.write_all(contents).expect("write test file");
            Self(path)
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            fs::remove_file(&self.0).expect("remove test file");
        }
    }

    #[test]
    fn maps_file_contents() {
        let expected = b"oboromi filesystem test";
        let test_file = TestFile::new(expected);
        let mapped_file = File::open(&test_file.0).expect("open mapped file");

        assert_eq!(&*mapped_file, expected);
    }

    #[test]
    fn opens_empty_file_as_empty_slice() {
        let test_file = TestFile::new(&[]);
        let mapped_file = File::open(&test_file.0).expect("open empty file");

        assert!(mapped_file.is_empty());
    }

    #[test]
    fn returns_not_found_for_missing_file() {
        let id = NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("oboromi-fs-test-{}-{id}.missing", std::process::id()));

        let error = File::open(path)
            .err()
            .expect("missing file should fail to open");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}