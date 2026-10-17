//! CMIF requests and replies in a thread's 0x100-byte IPC buffer, laid out as libnx's sf/hipc.h and sf/cmif.h do: the
//! HIPC header, an optional special header with the pid placeholder and copy handles, X, A then B buffer descriptors, and
//! the raw data aligned to 16 bytes starting with the SFCI or SFCO header.
const REQUEST: u32 = 4;
const CONTROL: u32 = 5;
const SFCI: u32 = u32::from_le_bytes(*b"SFCI");
const SFCO: u32 = u32::from_le_bytes(*b"SFCO");

#[derive(Clone, Copy)]
pub struct Buffer {
    pub address: u64,
    pub size: u64,
}

#[derive(Default)]
pub struct Request<'a> {
    pub command: u32,
    pub send_pid: bool,
    pub copy_handles: &'a [u32],
    pub pointer: &'a [Buffer],
    pub send: &'a [Buffer],
    pub receive: &'a [Buffer],
    pub data: &'a [u32],
}

pub struct Reply<'a> {
    pub result: u32,
    pub copy_handles: &'a [u32],
    pub move_handles: &'a [u32],
    pub data: &'a [u32],
}

fn write(ipc: &mut [u32; 64], request: &Request, kind: u32, domain: Option<(u32, &[u32])>) {
    let special = request.send_pid || !request.copy_handles.is_empty();
    ipc[0] = kind | (request.pointer.len() as u32) << 16 | (request.send.len() as u32) << 20 | (request.receive.len() as u32) << 24;
    ipc[1] = (8 + request.data.len() as u32 + domain.map_or(0, |(_, objects)| 4 + objects.len() as u32)) | u32::from(special) << 31;
    let mut at = 2;
    if special {
        ipc[2] = u32::from(request.send_pid) | (request.copy_handles.len() as u32) << 1;
        at = 3 + 2 * usize::from(request.send_pid);
        ipc[at..][..request.copy_handles.len()].copy_from_slice(request.copy_handles);
        at += request.copy_handles.len();
    }
    for (index, buffer) in request.pointer.iter().enumerate() {
        let (address, size) = (buffer.address, buffer.size as u32);
        ipc[at..][..2].copy_from_slice(&[index as u32 | ((address >> 36) as u32 & 7) << 6 | ((address >> 32) as u32 & 0xf) << 12 | size << 16, address as u32]);
        at += 2;
    }
    for buffer in request.send.iter().chain(request.receive) {
        let (address, size) = (buffer.address, buffer.size);
        ipc[at..][..3].copy_from_slice(&[size as u32, address as u32, ((address >> 36) as u32 & 0x3f_ffff) << 2 | ((size >> 32) as u32 & 0xf) << 24 | ((address >> 32) as u32 & 0xf) << 28]);
        at += 3;
    }
    let start = at.next_multiple_of(4);
    let cmif = if let Some((object, objects)) = domain {
        ipc[start..][..4].copy_from_slice(&[1 | (objects.len() as u32) << 8 | ((4 + request.data.len()) as u32 * 4) << 16, object, 0, 0]);
        start + 4
    } else {
        start
    };
    ipc[cmif..][..4].copy_from_slice(&[SFCI, 0, request.command, 0]);
    ipc[cmif + 4..][..request.data.len()].copy_from_slice(request.data);
    if let Some((_, objects)) = domain {
        ipc[cmif + 4 + request.data.len()..][..objects.len()].copy_from_slice(objects);
    }
}

pub fn write_request(ipc: &mut [u32; 64], request: &Request) {
    write(ipc, request, REQUEST, None);
}

pub fn write_control(ipc: &mut [u32; 64], command: u32, data: &[u32]) {
    write(ipc, &Request { command, data, ..Default::default() }, CONTROL, None);
}

pub fn write_domain_request(ipc: &mut [u32; 64], object: u32, objects: &[u32], request: &Request) {
    write(ipc, request, REQUEST, Some((object, objects)));
}

/// The reply in `ipc`, or None when its raw data does not start with SFCO.
pub fn read_reply(ipc: &[u32; 64]) -> Option<Reply<'_>> {
    let field = |word: u32, shift: u32, bits: u32| (word >> shift & ((1 << bits) - 1)) as usize;
    let mut at = 2;
    let (mut copies, mut moves) = (0, 0);
    if ipc[1] >> 31 != 0 {
        at = 3 + 2 * field(ipc[2], 0, 1);
        (copies, moves) = (field(ipc[2], 1, 4), field(ipc[2], 5, 4));
    }
    let (copy_handles, rest) = ipc[at..].split_at(copies);
    let move_handles = &rest[..moves];
    at += copies + moves + 2 * field(ipc[0], 16, 4) + 3 * (field(ipc[0], 20, 4) + field(ipc[0], 24, 4) + field(ipc[0], 28, 4));
    let start = at.next_multiple_of(4);
    (ipc[start] == SFCO).then(|| Reply { result: ipc[start + 2], copy_handles, move_handles, data: &ipc[start + 4..] })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_and_replies_follow_the_libnx_layout() {
        let mut ipc = [0; 64];
        let path = Buffer { address: 0x12_3456_7890, size: 0x10 };
        write_request(&mut ipc, &Request { command: 0, send_pid: true, copy_handles: &[0xabcd], send: &[path], data: &[7], ..Default::default() });
        assert_eq!(ipc[..16], [4 | 1 << 20, 9 | 1 << 31, 1 | 1 << 1, 0, 0, 0xabcd, 0x10, 0x3456_7890, 1 << 2 | 2 << 28, 0, 0, 0, SFCI, 0, 0, 0]);
        assert_eq!(ipc[16], 7);
        let mut ipc = [0; 64];
        ipc[..9].copy_from_slice(&[0, 9 | 1 << 31, 1 << 5, 0x55, SFCO, 0, 0, 0, 3]);
        let reply = read_reply(&ipc).unwrap();
        assert_eq!((reply.result, reply.copy_handles, reply.move_handles, reply.data[0]), (0, &[][..], &[0x55][..], 3));
        let mut ipc = [0; 64];
        write_control(&mut ipc, 3, &[]);
        assert_eq!(ipc[..8], [5, 8, 0, 0, SFCI, 0, 3, 0]);
        write_domain_request(&mut ipc, 1, &[], &Request { data: &[0x1111, 0x2222], ..Default::default() });
        assert_eq!(ipc[..14], [4, 14, 0, 0, 1 | 24 << 16, 1, 0, 0, SFCI, 0, 0, 0, 0x1111, 0x2222]);
    }
}
