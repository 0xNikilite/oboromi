//! fsp-srv (switchbrew Filesystem_services) as Ryujinx Services/Fs splits it: IFileSystemProxy hands out an
//! IFileSystem over the host directory standing in for the SD card and an IStorage over the program's RomFS.
use crate::cpu::cpu_manager::MEMORY_SIZE;
use crate::kernel::Error;
use crate::nn::ServiceTrait;
use crate::nn::hipc::{Buffer, Request, Response, Service};
use crate::sys;
use std::cell::RefCell;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

/// fs result descriptions (module 2), switchbrew Error_codes.
const PATH_NOT_FOUND: u32 = 2 | 1 << 9;
const PATH_ALREADY_EXISTS: u32 = 2 | 2 << 9;
/// FsDirectoryEntry: a 0x301-byte name, the entry type at 0x304 and the file size at 0x308.
const ENTRY_SIZE: usize = 0x310;

pub struct FileSystemProxy {
    pub sd: PathBuf,
    pub romfs: Rc<[u8]>,
}

pub type State = FileSystemProxy;

impl ServiceTrait for FileSystemProxy {
    fn run(state: &mut sys::State) {
        state.services.fsp_srv = Some(Self { sd: "sdmc".into(), romfs: Rc::from([]) });
    }
}

fn io_error(e: io::Error) -> u32 {
    if e.kind() == io::ErrorKind::AlreadyExists { PATH_ALREADY_EXISTS } else { PATH_NOT_FOUND }
}

fn arg(request: &Request, at: usize) -> u64 {
    request.args.get(at..at + 8).map_or(0, |b| u64::from_le_bytes(b.try_into().unwrap()))
}

fn data(bytes: impl Into<Vec<u8>>) -> Response {
    Response { data: bytes.into(), ..Response::default() }
}

fn session(service: impl Service + 'static) -> Response {
    Response { objects: vec![Rc::new(RefCell::new(service))], ..Response::default() }
}

/// The first `size` bytes of `buffer`, as (address, length) inside guest memory.
fn guest(buffer: Option<&Buffer>, size: u64) -> Result<(u64, usize), u32> {
    let buffer = buffer.ok_or(Error::InvalidPointer.code())?;
    let size = size.min(buffer.size);
    let fits = buffer.addr.checked_add(size).is_some_and(|end| end <= MEMORY_SIZE);
    if fits { Ok((buffer.addr, size as usize)) } else { Err(Error::InvalidPointer.code()) }
}

fn write_guest(request: &Request, (addr, len): (u64, usize), bytes: &[u8]) -> Result<(), u32> {
    request.mem.write_mem(addr, &bytes[..len.min(bytes.len())]).map_err(|_| Error::InvalidPointer.code())
}

fn respond(result: Result<Response, u32>) -> Response {
    result.unwrap_or_else(Response::error)
}

impl Service for FileSystemProxy {
    fn dispatch(&mut self, cmd: u32, _: &mut Request) -> Response {
        match cmd {
            1 | 1006 => Response::default(),
            18 => session(FileSystem(self.sd.clone())),
            200 => session(Storage(self.romfs.clone())),
            1005 => data(0u32.to_le_bytes()),
            _ => Response::unknown_command(cmd),
        }
    }
}

struct FileSystem(PathBuf);

impl FileSystem {
    /// The host path of the X buffer's path, which may not leave the root.
    fn path(&self, request: &Request) -> Result<PathBuf, u32> {
        let (addr, len) = guest(request.x.first(), u64::MAX)?;
        let mut bytes = vec![0; len];
        request.mem.read_mem(addr, &mut bytes).map_err(|_| Error::InvalidPointer.code())?;
        let path = String::from_utf8_lossy(bytes.split(|&b| b == 0).next().unwrap_or_default()).into_owned();
        let path = Path::new(path.trim_start_matches('/'));
        if path.components().all(|c| matches!(c, Component::Normal(_))) { Ok(self.0.join(path)) } else { Err(PATH_NOT_FOUND) }
    }

    fn call(&self, cmd: u32, request: &Request) -> Result<Response, u32> {
        if cmd == 10 {
            return Ok(Response::default());
        }
        let path = self.path(request)?;
        let mode = arg(request, 0) as u32;
        match cmd {
            0 => fs::File::create_new(&path).and_then(|f| f.set_len(arg(request, 8))).map(|()| Response::default()),
            1 => fs::remove_file(&path).map(|()| Response::default()),
            2 => fs::create_dir(&path).map(|()| Response::default()),
            7 => fs::metadata(&path).map(|m| data(u32::from(m.is_file()).to_le_bytes())),
            8 => fs::OpenOptions::new().read(mode & 1 != 0).write(mode & 2 != 0).open(&path).map(|f| session(File(f))),
            9 => Directory::open(&path, mode).map(session),
            _ => return Ok(Response::unknown_command(cmd)),
        }
        .map_err(io_error)
    }
}

impl Service for FileSystem {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        respond(self.call(cmd, request))
    }
}

struct File(fs::File);

impl File {
    fn call(&mut self, cmd: u32, request: &Request) -> Result<Response, u32> {
        let (offset, size) = (arg(request, 8), arg(request, 16));
        let file = &mut self.0;
        Ok(match cmd {
            0 => {
                let buffer = guest(request.b.first(), size)?;
                let mut bytes = vec![];
                file.seek(SeekFrom::Start(offset)).and_then(|_| file.take(buffer.1 as u64).read_to_end(&mut bytes)).map_err(io_error)?;
                write_guest(request, buffer, &bytes)?;
                data((bytes.len() as u64).to_le_bytes())
            }
            1 => {
                let (addr, len) = guest(request.a.first(), size)?;
                let mut bytes = vec![0; len];
                request.mem.read_mem(addr, &mut bytes).map_err(|_| Error::InvalidPointer.code())?;
                file.seek(SeekFrom::Start(offset)).and_then(|_| file.write_all(&bytes)).map_err(io_error)?;
                Response::default()
            }
            2 => Response::default(),
            3 => file.set_len(arg(request, 0)).map(|()| Response::default()).map_err(io_error)?,
            4 => file.metadata().map(|m| data(m.len().to_le_bytes())).map_err(io_error)?,
            _ => Response::unknown_command(cmd),
        })
    }
}

impl Service for File {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        respond(self.call(cmd, request))
    }
}

struct Directory {
    entries: Vec<u8>,
    read: usize,
}

impl Directory {
    /// Mode bit 0 lists directories, bit 1 files.
    fn open(path: &Path, mode: u32) -> io::Result<Self> {
        let mut entries = vec![];
        for entry in fs::read_dir(path)? {
            let (entry, mut bytes) = (entry?, [0; ENTRY_SIZE]);
            let meta = entry.metadata()?;
            if mode >> u32::from(meta.is_file()) & 1 == 0 {
                continue;
            }
            let name = entry.file_name();
            let name = name.as_encoded_bytes();
            bytes[..name.len().min(0x300)].copy_from_slice(&name[..name.len().min(0x300)]);
            bytes[0x304] = meta.is_file().into();
            bytes[0x308..].copy_from_slice(&meta.len().to_le_bytes());
            entries.extend(bytes);
        }
        Ok(Self { entries, read: 0 })
    }
}

impl Service for Directory {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        respond(match cmd {
            0 => guest(request.b.first(), u64::MAX).and_then(|buffer| {
                let rest = &self.entries[self.read..];
                let len = rest.len().min(buffer.1 / ENTRY_SIZE * ENTRY_SIZE);
                write_guest(request, buffer, &rest[..len])?;
                self.read += len;
                Ok(data(((len / ENTRY_SIZE) as u64).to_le_bytes()))
            }),
            1 => Ok(data(((self.entries.len() / ENTRY_SIZE) as u64).to_le_bytes())),
            _ => Ok(Response::unknown_command(cmd)),
        })
    }
}

struct Storage(Rc<[u8]>);

impl Service for Storage {
    fn dispatch(&mut self, cmd: u32, request: &mut Request) -> Response {
        respond(match cmd {
            0 => guest(request.b.first(), arg(request, 8)).and_then(|buffer| {
                let offset = usize::try_from(arg(request, 0)).unwrap_or(usize::MAX);
                write_guest(request, buffer, self.0.get(offset..).unwrap_or_default()).map(|()| Response::default())
            }),
            4 => Ok(data((self.0.len() as u64).to_le_bytes())),
            _ => Ok(Response::unknown_command(cmd)),
        })
    }
}
