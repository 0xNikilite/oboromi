#[repr(C)]
pub struct HeaderData {
    pub header: [u32; 2],
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HipcHeader {
    pub tag: u16,
    pub pointer_count: u8,
    pub send_count: u8,
    pub receive_count: u8,
    pub exchange_count: u8,
    pub raw_count: u16,
    pub receive_list_count: u8,
    pub receive_list_offset: u16,
    pub special_header: bool,
}

pub fn parse_header(data: &[u8]) -> Option<HipcHeader> {
    let header = data.get(..8)?;
    let header0 = u32::from_le_bytes(header[..4].try_into().ok()?);
    let header1 = u32::from_le_bytes(header[4..].try_into().ok()?);

    Some(HipcHeader {
        tag: (header0 & 0xffff) as u16,
        pointer_count: ((header0 >> 16) & 0xf) as u8,
        send_count: ((header0 >> 20) & 0xf) as u8,
        receive_count: ((header0 >> 24) & 0xf) as u8,
        exchange_count: ((header0 >> 28) & 0xf) as u8,
        raw_count: (header1 & 0x3ff) as u16,
        receive_list_count: ((header1 >> 10) & 0xf) as u8,
        receive_list_offset: ((header1 >> 14) & 0x3ff) as u16,
        special_header: (header1 >> 31) != 0,
    })
}

pub fn invoke_method<F>(data: &[u8], f: F) -> Result
where
    F: FnOnce(HipcHeader) -> Result,
{
    let Some(header) = parse_header(data) else {
        return Result::Failure;
    };

    f(header)
}

#[cfg(test)]
mod tests {
    use super::{HipcHeader, Result, invoke_method, parse_header};

    fn request_header() -> [u8; 8] {
        let header0 = 0x1234u32 | (2 << 16) | (3 << 20) | (4 << 24) | (5 << 28);
        let header1 = 0x2aau32 | (6 << 10) | (0x155 << 14) | (1 << 31);
        let mut data = [0; 8];
        data[..4].copy_from_slice(&header0.to_le_bytes());
        data[4..].copy_from_slice(&header1.to_le_bytes());
        data
    }

    #[test]
    fn parses_request_header_fields() {
        assert_eq!(
            parse_header(&request_header()),
            Some(HipcHeader {
                tag: 0x1234,
                pointer_count: 2,
                send_count: 3,
                receive_count: 4,
                exchange_count: 5,
                raw_count: 0x2aa,
                receive_list_count: 6,
                receive_list_offset: 0x155,
                special_header: true,
            })
        );
    }

    #[test]
    fn parses_zeroed_header() {
        assert_eq!(
            parse_header(&[0; 8]),
            Some(HipcHeader {
                tag: 0,
                pointer_count: 0,
                send_count: 0,
                receive_count: 0,
                exchange_count: 0,
                raw_count: 0,
                receive_list_count: 0,
                receive_list_offset: 0,
                special_header: false,
            })
        );
    }

    #[test]
    fn parses_maximum_header_field_values() {
        let header0 = u32::MAX;
        let header1 = 0x3ffu32 | (0xf << 10) | (0x3ff << 14) | (1 << 31);
        let mut data = [0; 8];
        data[..4].copy_from_slice(&header0.to_le_bytes());
        data[4..].copy_from_slice(&header1.to_le_bytes());

        assert_eq!(
            parse_header(&data),
            Some(HipcHeader {
                tag: 0xffff,
                pointer_count: 0xf,
                send_count: 0xf,
                receive_count: 0xf,
                exchange_count: 0xf,
                raw_count: 0x3ff,
                receive_list_count: 0xf,
                receive_list_offset: 0x3ff,
                special_header: true,
            })
        );
    }

    #[test]
    fn rejects_truncated_header_without_invoking_handler() {
        let called = std::cell::Cell::new(false);
        let data = request_header();
        for length in 0..data.len() {
            assert_eq!(parse_header(&data[..length]), None);
            let result = invoke_method(&data[..length], |_| {
                called.set(true);
                Result::Success
            });

            assert!(matches!(result, Result::Failure));
            assert!(!called.get());
        }
    }

    #[test]
    fn passes_parsed_header_to_handler() {
        let expected = parse_header(&request_header()).expect("valid request header");
        let result = invoke_method(&request_header(), |header| {
            assert_eq!(header, expected);
            Result::Success
        });
        assert!(matches!(result, Result::Success));
    }

    #[test]
    fn parses_header_before_request_payload() {
        let mut request = request_header().to_vec();
        request.extend_from_slice(&[0xaa, 0xbb, 0xcc]);

        assert_eq!(parse_header(&request), parse_header(&request_header()));
    }

    #[test]
    fn parses_header_from_unaligned_slice() {
        let mut data = [0; 9];
        data[1..].copy_from_slice(&request_header());

        assert_eq!(parse_header(&data[1..]), parse_header(&request_header()));
    }

    #[test]
    fn propagates_handler_failure() {
        let result = invoke_method(&request_header(), |_| Result::Failure);

        assert!(matches!(result, Result::Failure));
    }
}
