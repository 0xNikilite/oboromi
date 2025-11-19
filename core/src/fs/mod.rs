use std::str::FromStr;

pub struct File<'a> {
    pub handle: std::ffi::c_int,
    pub slice: Option<&'a mut [u8]>,
}
impl<'a> File<'a> {
    unsafe fn open_impl<const W: bool, P>(path: P) -> Result<(std::ffi::c_int, &'a mut [u8]), Box<dyn std::error::Error>>
    where
        P: AsRef<std::path::Path>
    {
        unsafe {
            let c_path = std::ffi::CString::from_str(path.as_ref().to_str().unwrap_or(""))?;
            let fd = libc::open(c_path.as_c_str().as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK);
            assert!(fd >= 0);
            let len = {
                let mut sb = core::mem::MaybeUninit::<libc::stat>::uninit();
                let ret = libc::fstat(fd, sb.as_mut_ptr());
                assert!(ret >= 0);
                sb.assume_init().st_size
            } as usize;
            let ptr = libc::mmap(core::ptr::null_mut(), len, libc::PROT_READ, libc::MAP_PRIVATE, fd, 0);
            assert_ne!(ptr, libc::MAP_FAILED);
            Ok((fd, core::slice::from_raw_parts_mut(ptr as *mut u8, len)))
        }
    }
    pub async fn open<const W: bool, P>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        P: AsRef<std::path::Path>
    {
        let (fd, slice) = unsafe { Self::open_impl::<W, P>(path) }?;
        Ok(Self { handle: fd, slice: Some(slice) })
    }
}
impl<'a> Drop for File<'a> {
    fn drop(&mut self) {
        if let Some(_) = self.slice {
            let len = self.slice.as_ref().unwrap().len();
            unsafe { libc::munmap(self.slice.as_mut().unwrap().as_mut_ptr() as *mut std::ffi::c_void, len) };
        }
        if self.handle > 0 {
            unsafe { libc::close(self.handle) };
        }
    }
}
