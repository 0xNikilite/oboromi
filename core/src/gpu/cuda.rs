use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

const SEED: &[u8] = include_bytes!("../../tests/fixtures/cuda-seed.cubin");
const NOP: u128 = 0x000fc000000000000000000000007918;

#[derive(Debug, PartialEq, Eq)]
pub enum CudaError {
    MissingLibrary,
    NoDevice,
    Driver(i32),
    Cubin(&'static str),
}

type Device = c_int;
type Context = *mut c_void;
type Module = *mut c_void;
type Function = *mut c_void;
type DevicePtr = u64;

struct Api {
    library: *mut c_void,
    init: unsafe extern "C" fn(c_uint) -> c_int,
    device_get: unsafe extern "C" fn(*mut Device, c_int) -> c_int,
    ctx_create: unsafe extern "C" fn(*mut Context, c_uint, Device) -> c_int,
    ctx_destroy: unsafe extern "C" fn(Context) -> c_int,
    ctx_push: unsafe extern "C" fn(Context) -> c_int,
    ctx_pop: unsafe extern "C" fn(*mut Context) -> c_int,
    module_load: unsafe extern "C" fn(*mut Module, *const c_void) -> c_int,
    module_get_function: unsafe extern "C" fn(*mut Function, Module, *const c_char) -> c_int,
    module_unload: unsafe extern "C" fn(Module) -> c_int,
    mem_alloc: unsafe extern "C" fn(*mut DevicePtr, usize) -> c_int,
    mem_free: unsafe extern "C" fn(DevicePtr) -> c_int,
    _mem_host_register: unsafe extern "C" fn(*mut c_void, usize, c_uint) -> c_int,
    _mem_host_get_device_pointer:
        unsafe extern "C" fn(*mut DevicePtr, *mut c_void, c_uint) -> c_int,
    memcpy_htod: unsafe extern "C" fn(DevicePtr, *const c_void, usize) -> c_int,
    memcpy_dtoh: unsafe extern "C" fn(*mut c_void, DevicePtr, usize) -> c_int,
    launch: unsafe extern "C" fn(
        Function,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        *mut c_void,
        *mut *mut c_void,
        *mut *mut c_void,
    ) -> c_int,
    synchronize: unsafe extern "C" fn() -> c_int,
}

impl Api {
    unsafe fn load() -> Result<Self, CudaError> {
        let library = unsafe { libc::dlopen(c"libcuda.so.1".as_ptr(), libc::RTLD_NOW) };
        if library.is_null() {
            return Err(CudaError::MissingLibrary);
        }
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let p = unsafe { libc::dlsym(library, concat!($name, "\0").as_ptr().cast()) };
                if p.is_null() {
                    unsafe { libc::dlclose(library) };
                    return Err(CudaError::MissingLibrary);
                }
                unsafe { std::mem::transmute::<*mut c_void, $ty>(p) }
            }};
        }
        Ok(Self {
            library,
            init: symbol!("cuInit", unsafe extern "C" fn(c_uint) -> c_int),
            device_get: symbol!(
                "cuDeviceGet",
                unsafe extern "C" fn(*mut Device, c_int) -> c_int
            ),
            ctx_create: symbol!(
                "cuCtxCreate_v2",
                unsafe extern "C" fn(*mut Context, c_uint, Device) -> c_int
            ),
            ctx_destroy: symbol!("cuCtxDestroy_v2", unsafe extern "C" fn(Context) -> c_int),
            ctx_push: symbol!(
                "cuCtxPushCurrent_v2",
                unsafe extern "C" fn(Context) -> c_int
            ),
            ctx_pop: symbol!(
                "cuCtxPopCurrent_v2",
                unsafe extern "C" fn(*mut Context) -> c_int
            ),
            module_load: symbol!(
                "cuModuleLoadData",
                unsafe extern "C" fn(*mut Module, *const c_void) -> c_int
            ),
            module_get_function: symbol!(
                "cuModuleGetFunction",
                unsafe extern "C" fn(*mut Function, Module, *const c_char) -> c_int
            ),
            module_unload: symbol!("cuModuleUnload", unsafe extern "C" fn(Module) -> c_int),
            mem_alloc: symbol!(
                "cuMemAlloc_v2",
                unsafe extern "C" fn(*mut DevicePtr, usize) -> c_int
            ),
            mem_free: symbol!("cuMemFree_v2", unsafe extern "C" fn(DevicePtr) -> c_int),
            _mem_host_register: symbol!(
                "cuMemHostRegister_v2",
                unsafe extern "C" fn(*mut c_void, usize, c_uint) -> c_int
            ),
            _mem_host_get_device_pointer: symbol!(
                "cuMemHostGetDevicePointer_v2",
                unsafe extern "C" fn(*mut DevicePtr, *mut c_void, c_uint) -> c_int
            ),
            memcpy_htod: symbol!(
                "cuMemcpyHtoD_v2",
                unsafe extern "C" fn(DevicePtr, *const c_void, usize) -> c_int
            ),
            memcpy_dtoh: symbol!(
                "cuMemcpyDtoH_v2",
                unsafe extern "C" fn(*mut c_void, DevicePtr, usize) -> c_int
            ),
            launch: symbol!(
                "cuLaunchKernel",
                unsafe extern "C" fn(
                    Function,
                    c_uint,
                    c_uint,
                    c_uint,
                    c_uint,
                    c_uint,
                    c_uint,
                    c_uint,
                    *mut c_void,
                    *mut *mut c_void,
                    *mut *mut c_void,
                ) -> c_int
            ),
            synchronize: symbol!("cuCtxSynchronize", unsafe extern "C" fn() -> c_int),
        })
    }
}

pub struct CudaDevice {
    api: Api,
    context: Context,
}

impl CudaDevice {
    pub fn new() -> Result<Self, CudaError> {
        let api = unsafe { Api::load()? };
        let mut device = 0;
        let mut context = ptr::null_mut();
        let opened = (|| unsafe {
            check((api.init)(0)).map_err(no_device)?;
            check((api.device_get)(&mut device, 0)).map_err(no_device)?;
            check((api.ctx_create)(&mut context, 0, device)).map_err(no_device)?;
            let mut current = ptr::null_mut();
            check((api.ctx_pop)(&mut current)).map_err(no_device)
        })();
        if let Err(error) = opened {
            if !context.is_null() {
                unsafe { (api.ctx_destroy)(context) };
            }
            unsafe { libc::dlclose(api.library) };
            return Err(error);
        }
        Ok(Self { api, context })
    }

    pub fn run_native(
        &self,
        program: &[u128],
        grid: [u32; 3],
        block: [u32; 3],
        shared_bytes: u32,
        params: &[u8],
        buffers: &mut [&mut [u8]],
    ) -> Result<(), CudaError> {
        unsafe { check((self.api.ctx_push)(self.context))? };
        let mut allocations = Vec::with_capacity(buffers.len());
        let result = (|| unsafe {
            for buffer in buffers.iter() {
                let mut pointer = 0;
                check((self.api.mem_alloc)(&mut pointer, buffer.len()))?;
                allocations.push(pointer);
                check((self.api.memcpy_htod)(
                    pointer,
                    buffer.as_ptr().cast(),
                    buffer.len(),
                ))?;
            }
            let mut packed = Vec::with_capacity(8 * allocations.len() + params.len());
            for pointer in &allocations {
                packed.extend_from_slice(&pointer.to_le_bytes());
            }
            packed.extend_from_slice(params);
            let cubin = cubin(program, shared_bytes, allocations.len(), params.len())?;
            let mut module = ptr::null_mut();
            check((self.api.module_load)(&mut module, cubin.as_ptr().cast()))?;
            let launched = (|| {
                let mut function = ptr::null_mut();
                check((self.api.module_get_function)(
                    &mut function,
                    module,
                    c"guest".as_ptr(),
                ))?;
                let mut size = packed.len();
                let mut extra = [
                    1usize as *mut c_void,
                    packed.as_mut_ptr().cast(),
                    2usize as *mut c_void,
                    (&mut size as *mut usize).cast(),
                    ptr::null_mut(),
                ];
                check((self.api.launch)(
                    function,
                    grid[0],
                    grid[1],
                    grid[2],
                    block[0],
                    block[1],
                    block[2],
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    extra.as_mut_ptr(),
                ))?;
                check((self.api.synchronize)())
            })();
            let unloaded = check((self.api.module_unload)(module));
            launched.and(unloaded)?;
            for (buffer, pointer) in buffers.iter_mut().zip(&allocations) {
                check((self.api.memcpy_dtoh)(
                    buffer.as_mut_ptr().cast(),
                    *pointer,
                    buffer.len(),
                ))?;
            }
            Ok(())
        })();
        for pointer in allocations.into_iter().rev() {
            unsafe { (self.api.mem_free)(pointer) };
        }
        let mut current = ptr::null_mut();
        let popped = unsafe { check((self.api.ctx_pop)(&mut current)) };
        result.and(popped)
    }
}

impl Drop for CudaDevice {
    fn drop(&mut self) {
        unsafe {
            (self.api.ctx_destroy)(self.context);
            libc::dlclose(self.api.library);
        }
    }
}

fn check(result: c_int) -> Result<(), CudaError> {
    if result == 0 {
        Ok(())
    } else {
        Err(CudaError::Driver(result))
    }
}

fn no_device(error: CudaError) -> CudaError {
    match error {
        CudaError::Driver(100) => CudaError::NoDevice,
        error => error,
    }
}

#[derive(Clone)]
struct Section {
    name: String,
    offset: usize,
    size: usize,
    info: u32,
    align: usize,
}

#[derive(Clone)]
struct Segment {
    ty: u32,
    offset: usize,
    filesz: usize,
    memsz: usize,
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}
fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes())
}
fn put_u64(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 8].copy_from_slice(&(value as u64).to_le_bytes())
}

fn cubin(
    program: &[u128],
    shared_bytes: u32,
    pointers: usize,
    params: usize,
) -> Result<Vec<u8>, CudaError> {
    let parameter_bytes = pointers * 8 + params;
    if program.is_empty() || parameter_bytes > 0x10000 - 0x160 {
        return Err(CudaError::Cubin("invalid kernel image"));
    }
    let mut image = SEED.to_vec();
    if &image[..4] != b"\x7fELF" || image[4] != 2 || image[5] != 1 {
        return Err(CudaError::Cubin("invalid seed cubin"));
    }
    let mut shoff = u64_at(&image, 40) as usize;
    let mut phoff = u64_at(&image, 32) as usize;
    let shnum = u16_at(&image, 60) as usize;
    let phnum = u16_at(&image, 56) as usize;
    let shstrndx = u16_at(&image, 62) as usize;
    let mut sections = (0..shnum)
        .map(|i| {
            let h = shoff + i * 64;
            Section {
                name: u32_at(&image, h).to_string(),
                offset: u64_at(&image, h + 24) as usize,
                size: u64_at(&image, h + 32) as usize,
                info: u32_at(&image, h + 44),
                align: u64_at(&image, h + 48) as usize,
            }
        })
        .collect::<Vec<_>>();
    let strings = sections[shstrndx].clone();
    let names = &image[strings.offset..strings.offset + strings.size];
    for section in &mut sections {
        let start: usize = section.name.parse().unwrap();
        let end = names[start..].iter().position(|&b| b == 0).unwrap() + start;
        section.name = String::from_utf8_lossy(&names[start..end]).into_owned();
    }
    let mut segments = (0..phnum)
        .map(|i| {
            let h = phoff + i * 56;
            Segment {
                ty: u32_at(&image, h),
                offset: u64_at(&image, h + 8) as usize,
                filesz: u64_at(&image, h + 32) as usize,
                memsz: u64_at(&image, h + 40) as usize,
            }
        })
        .collect::<Vec<_>>();
    let text = find(&sections, ".text.guest")?;
    let info = find(&sections, ".nv.info.guest")?;
    let shared = find(&sections, ".nv.shared.guest")?;
    let constant = find(&sections, ".nv.constant0.guest")?;
    let global_info = find(&sections, ".nv.info")?;
    let symtab = find(&sections, ".symtab")?;
    let mut code = program
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    while code.len() % 128 != 0 {
        code.extend_from_slice(&NOP.to_le_bytes());
    }
    let exits = program
        .iter()
        .enumerate()
        .filter_map(|(i, word)| ((*word as u16 & 0xfff) == 0x94d).then_some((i * 16) as u32))
        .collect::<Vec<_>>();
    let metadata = metadata(
        &image[sections[info].offset..][..sections[info].size],
        pointers,
        params,
        &exits,
    )?;
    let mut constant_data = image[sections[constant].offset..][..sections[constant].size].to_vec();
    constant_data.resize(0x160 + parameter_bytes, 0);
    let mut replacements = vec![
        (sections[text].offset, sections[text].size, code, text),
        (sections[info].offset, sections[info].size, metadata, info),
        (
            sections[constant].offset,
            sections[constant].size,
            constant_data,
            constant,
        ),
    ];
    replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.0));
    for (offset, old, data, index) in replacements {
        splice(
            &mut image,
            &mut sections,
            &mut segments,
            &mut shoff,
            &mut phoff,
            offset,
            old,
            &data,
        );
        sections[index].size = data.len();
    }
    let padding = (sections[text].align - sections[text].offset % sections[text].align)
        % sections[text].align;
    if padding != 0 {
        let offset = sections[text].offset;
        splice(
            &mut image,
            &mut sections,
            &mut segments,
            &mut shoff,
            &mut phoff,
            offset,
            0,
            &vec![0; padding],
        );
    }
    sections[shared].size = shared_bytes as usize;
    sections[text].info = sections[text].info & 0x00ff_ffff | 0xff00_0000;
    for segment in &mut segments {
        if segment.ty == 1 && segment.offset == sections[shared].offset && segment.filesz == 0 {
            segment.memsz = shared_bytes as usize;
        }
    }
    let gi = sections[global_info].offset;
    let mut at = gi;
    while at < gi + sections[global_info].size {
        let len = if image[at] == 4 {
            4 + u16_at(&image, at + 2) as usize
        } else {
            4
        };
        if image[at + 1] == 0x2f {
            put_u32(&mut image, at + 8, 255);
        }
        at += len;
    }
    let sym = sections[symtab].offset;
    for at in (sym..sym + sections[symtab].size).step_by(24) {
        if image[at + 4] & 0xf == 2 && u16_at(&image, at + 6) as usize == text {
            put_u64(&mut image, at + 16, sections[text].size);
        }
    }
    put_u64(&mut image, 32, phoff);
    put_u64(&mut image, 40, shoff);
    for (i, section) in sections.iter().enumerate() {
        let h = shoff + i * 64;
        put_u64(&mut image, h + 24, section.offset);
        put_u64(&mut image, h + 32, section.size);
        put_u32(&mut image, h + 44, section.info);
    }
    for (i, segment) in segments.iter().enumerate() {
        let h = phoff + i * 56;
        put_u64(&mut image, h + 8, segment.offset);
        put_u64(&mut image, h + 32, segment.filesz);
        put_u64(&mut image, h + 40, segment.memsz);
    }
    super::cubin::Kernel::load(&image, "guest")
        .map_err(|_| CudaError::Cubin("rewritten cubin is invalid"))?;
    Ok(image)
}

fn find(sections: &[Section], name: &str) -> Result<usize, CudaError> {
    sections
        .iter()
        .position(|section| section.name == name)
        .ok_or(CudaError::Cubin("seed section missing"))
}

fn splice(
    image: &mut Vec<u8>,
    sections: &mut [Section],
    segments: &mut [Segment],
    shoff: &mut usize,
    phoff: &mut usize,
    offset: usize,
    old: usize,
    data: &[u8],
) {
    let end = offset + old;
    let delta = data.len() as isize - old as isize;
    image.splice(offset..end, data.iter().copied());
    let shift = |value: &mut usize| *value = (*value as isize + delta) as usize;
    if *shoff >= end {
        shift(shoff);
    }
    if *phoff >= end {
        shift(phoff);
    }
    for section in sections {
        if section.offset != 0 && section.offset >= end {
            shift(&mut section.offset);
        }
    }
    for segment in segments {
        if segment.offset >= end {
            shift(&mut segment.offset);
        } else if segment.offset <= offset && segment.offset + segment.filesz >= end {
            shift(&mut segment.filesz);
            shift(&mut segment.memsz);
        }
    }
}

fn metadata(
    seed: &[u8],
    pointers: usize,
    params: usize,
    exits: &[u32],
) -> Result<Vec<u8>, CudaError> {
    let mut records = Vec::new();
    let mut at = 0;
    let mut cbank_symbol = None;
    while at < seed.len() {
        let len = if seed[at] == 4 {
            4 + u16_at(seed, at + 2) as usize
        } else {
            4
        };
        if at + len > seed.len() {
            return Err(CudaError::Cubin("invalid seed metadata"));
        }
        if seed[at + 1] == 0x0a {
            cbank_symbol = Some(u32_at(seed, at + 4));
        }
        if !matches!(seed[at + 1], 0x0a | 0x19 | 0x17 | 0x1c) {
            records.push(seed[at..at + len].to_vec());
        }
        at += len;
    }
    let total = pointers * 8 + params;
    let mut out = records
        .drain(..2.min(records.len()))
        .flatten()
        .collect::<Vec<_>>();
    if total != 0 {
        sval(
            &mut out,
            0x0a,
            &[
                cbank_symbol.ok_or(CudaError::Cubin("parameter bank missing"))?,
                ((total as u32) << 16) | 0x160,
            ],
        );
        out.extend_from_slice(&[3, 0x19]);
        out.extend_from_slice(&(total as u16).to_le_bytes());
        let mut fields = (0..pointers).map(|i| (i * 8, 8)).collect::<Vec<_>>();
        let mut offset = pointers * 8;
        while offset < total {
            let remaining = total - offset;
            let size = [8, 4, 2, 1]
                .into_iter()
                .find(|size| offset % size == 0 && *size <= remaining)
                .unwrap();
            fields.push((offset, size));
            offset += size;
        }
        for (ordinal, &(offset, size)) in fields.iter().enumerate().rev() {
            sval(
                &mut out,
                0x17,
                &[
                    0,
                    ordinal as u32 | (offset as u32) << 16,
                    ((4 * size + 1) as u32) << 16 | 0xf000,
                ],
            );
        }
    }
    out.extend(records.into_iter().flatten());
    if !exits.is_empty() {
        sval(&mut out, 0x1c, exits);
    }
    Ok(out)
}

fn sval(out: &mut Vec<u8>, attribute: u8, values: &[u32]) {
    out.extend_from_slice(&[4, attribute]);
    out.extend_from_slice(&((values.len() * 4) as u16).to_le_bytes());
    out.extend(values.iter().flat_map(|value| value.to_le_bytes()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn seed_rewrite_updates_program_parameters_and_shared_memory() {
        let image = cubin(&[0x000fea0003800000000000000000794d], 1024, 5, 4).unwrap();
        assert!(
            image
                .windows(16)
                .any(|bytes| bytes == 0x000fea0003800000000000000000794du128.to_le_bytes())
        );
        let path = std::env::temp_dir().join(format!("oboromi-cuda-{}.cubin", std::process::id()));
        std::fs::write(&path, image).unwrap();
        for (tool, args) in [
            (
                "/usr/local/cuda-13.1/bin/cuobjdump",
                vec!["-elf", path.to_str().unwrap()],
            ),
            (
                "/usr/local/cuda-13.1/bin/nvdisasm",
                vec![path.to_str().unwrap()],
            ),
        ] {
            if !std::path::Path::new(tool).is_file() {
                continue;
            }
            let output = Command::new(tool).args(args).output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            if tool.ends_with("cuobjdump") {
                let text = String::from_utf8_lossy(&output.stdout);
                assert!(text.contains("smem=1024"));
                assert!(text.contains("Ordinal : 0x5\tOffset  : 0x28\tSize    : 0x4"));
            }
        }
        std::fs::remove_file(path).unwrap();
    }
}
