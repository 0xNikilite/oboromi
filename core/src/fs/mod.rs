use memmap2::Mmap;
use std::fs;
use std::path::Path;
use std::ops::Deref;

pub struct File {
    map: Mmap,
}

impl File {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error>
    where
    {
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

#[cfg(test)]
mod tests {
    use super::File;
    use std::fs;
    use std::io::ErrorKind;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            loop {
                let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "oboromi-fs-tests-{}-{id}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("failed to create test directory: {error}"),
                }
            }
        }

        fn file_path(&self) -> PathBuf {
            self.0.join("input.bin")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            // The mapped file is dropped before this directory on Windows too.
            let _ = fs::remove_file(self.file_path());
            let _ = fs::remove_dir(&self.0);
        }
    }

    #[test]
    fn open_preserves_binary_contents() {
        let directory = TestDirectory::new();
        let path = directory.file_path();
        let expected: Vec<u8> = (0u8..=255).cycle().take(8193).collect();
        fs::write(&path, &expected).expect("write binary fixture");

        let mapped = File::open(&path).expect("open binary fixture");

        assert_eq!(&*mapped, expected.as_slice());
    }

    #[test]
    fn open_missing_file_returns_not_found() {
        let directory = TestDirectory::new();

        match File::open(directory.file_path()) {
            Err(error) => assert_eq!(error.kind(), ErrorKind::NotFound),
            Ok(_) => panic!("opening a missing file unexpectedly succeeded"),
        }
    }

    #[test]
    fn open_empty_file_returns_empty_slice() {
        let directory = TestDirectory::new();
        let path = directory.file_path();
        fs::write(&path, []).expect("write empty fixture");

        let mapped = File::open(&path).expect("open empty fixture");

        assert!(mapped.is_empty());
    }
}
