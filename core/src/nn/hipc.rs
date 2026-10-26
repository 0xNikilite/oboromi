#[repr(C)]
pub struct HeaderData {
    pub header: [u32; 2],
}

impl HeaderData {
    /// Decodes the fixed eight-byte header, ignoring any following payload.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let bytes = data.get(..8)?;
        let first = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let second = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        Some(Self {
            header: [first, second],
        })
    }
}

#[repr(C)]
pub struct MapData {
    pub data: [u32; 3],
}

#[repr(C)]
pub struct PointerData {
    pub data: [u32; 2],
}

#[repr(C)]
pub struct ReceiveListData {
    pub data: [u32; 2],
}

#[repr(u32)]
pub enum Result {
    Success = 0,
    Failure = 1,
}

/// Invokes the method only when the fixed HIPC header is present.
/// Descriptor and payload validation is not yet implemented.
pub fn invoke_method<F>(data: &[u8], f: F) -> Result
where
    F: Fn() -> Result,
{
    let Some(header) = HeaderData::from_bytes(data) else {
        return Result::Failure;
    };

    let [hdr0, hdr1] = header.header;
    let _tag          = (hdr0 >>  0) & 0xffff;
    let _ptrs_count   = (hdr0 >> 16) & 0xf;
    let _send_count   = (hdr0 >> 20) & 0xf;
    let _recv_count   = (hdr0 >> 24) & 0xf;
    let _xchg_count   = (hdr0 >> 28) & 0xf;

    let _raw_count       = (hdr1 >>  0) & ((1 << 10) - 1);
    let _recv_list_count = (hdr1 >> 10) & 0xf;
    let _recv_list_offs  = (hdr1 >> 14) & 0x3ff;
    let _special_count   = (hdr1 >> 31) & 0x1;

    f()
}

#[cfg(test)]
mod tests {
    use super::{HeaderData, Result, invoke_method};
    use std::cell::Cell;

    const HEADER_BYTES: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
    const HEADER_WORDS: [u32; 2] = [0x7856_3412, 0xf0de_bc9a];

    #[repr(align(4))]
    struct AlignedBytes([u8; 12]);

    #[test]
    fn header_decodes_little_endian_words() {
        let header = HeaderData::from_bytes(&HEADER_BYTES).expect("complete header");
        assert_eq!(header.header, HEADER_WORDS);
    }

    #[test]
    fn header_rejects_every_truncated_length() {
        for length in 0..HEADER_BYTES.len() {
            assert!(HeaderData::from_bytes(&HEADER_BYTES[..length]).is_none());
        }
    }

    #[test]
    fn header_ignores_following_payload() {
        let mut message = HEADER_BYTES.to_vec();
        message.extend_from_slice(&[0xff, 0x00, 0x81]);

        let header = HeaderData::from_bytes(&message).expect("header with payload");
        assert_eq!(header.header, HEADER_WORDS);
    }

    #[test]
    fn header_accepts_unaligned_input() {
        let mut storage = AlignedBytes([0; 12]);
        storage.0[1..9].copy_from_slice(&HEADER_BYTES);
        let bytes = &storage.0[1..9];
        assert_ne!((bytes.as_ptr() as usize) % std::mem::align_of::<u32>(), 0);

        let header = HeaderData::from_bytes(bytes).expect("unaligned header");
        assert_eq!(header.header, HEADER_WORDS);
    }

    #[test]
    fn invoke_rejects_truncated_headers_without_calling_method() {
        for length in 0..HEADER_BYTES.len() {
            let called = Cell::new(false);
            let result = invoke_method(&HEADER_BYTES[..length], || {
                called.set(true);
                Result::Success
            });

            assert!(matches!(result, Result::Failure));
            assert!(!called.get(), "method called for a {length}-byte header");
        }
    }

    #[test]
    fn invoke_calls_method_once_for_complete_header() {
        let calls = Cell::new(0);
        let result = invoke_method(&[0; 8], || {
            calls.set(calls.get() + 1);
            Result::Success
        });

        assert!(matches!(result, Result::Success));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn invoke_preserves_method_failure() {
        let calls = Cell::new(0);
        let result = invoke_method(&[0; 8], || {
            calls.set(calls.get() + 1);
            Result::Failure
        });

        assert!(matches!(result, Result::Failure));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn invoke_accepts_unaligned_header_with_payload() {
        let mut storage = AlignedBytes([0; 12]);
        storage.0[9..].copy_from_slice(&[0xff, 0x00, 0x81]);
        let bytes = &storage.0[1..];
        assert_ne!((bytes.as_ptr() as usize) % std::mem::align_of::<u32>(), 0);
        let called = Cell::new(false);

        let result = invoke_method(bytes, || {
            called.set(true);
            Result::Success
        });

        assert!(matches!(result, Result::Success));
        assert!(called.get());
    }
}
