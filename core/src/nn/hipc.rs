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
pub enum HipcSection {
    Header,
    SpecialHeader,
    PointerDescriptors,
    SendDescriptors,
    ReceiveDescriptors,
    ExchangeDescriptors,
    RawData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HipcParseError {
    Truncated {
        section: HipcSection,
        expected_at_least: usize,
        actual: usize,
    },
    LengthOverflow {
        section: HipcSection,
    },
}

impl std::fmt::Display for HipcParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated {
                section,
                expected_at_least,
                actual,
            } => write!(
                f,
                "truncated {section:?}: expected at least {expected_at_least} bytes, got {actual}"
            ),
            Self::LengthOverflow { section } => {
                write!(f, "{section:?} length overflow")
            }
        }
    }
}

impl std::error::Error for HipcParseError {}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HipcSpecialHeader {
    pub process_id: Option<u64>,
    pub copy_handles: Vec<u32>,
    pub move_handles: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HipcPointerDescriptor {
    pub index: u8,
    pub address: u64,
    pub size: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HipcMapDescriptor {
    pub address: u64,
    pub size: u64,
    pub allows_non_secure: bool,
    pub allows_non_device: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HipcMessage<'a> {
    pub header: HipcHeader,
    pub special_header: Option<HipcSpecialHeader>,
    pub pointer_descriptors: Vec<HipcPointerDescriptor>,
    pub send_descriptors: Vec<HipcMapDescriptor>,
    pub receive_descriptors: Vec<HipcMapDescriptor>,
    pub exchange_descriptors: Vec<HipcMapDescriptor>,
    pub raw_data: &'a [u8],
    pub trailing_data: &'a [u8],
}

fn checked_end(
    offset: usize,
    length: usize,
    section: HipcSection,
) -> std::result::Result<usize, HipcParseError> {
    offset
        .checked_add(length)
        .ok_or(HipcParseError::LengthOverflow { section })
}

fn require_range(
    data: &[u8],
    offset: usize,
    length: usize,
    section: HipcSection,
) -> std::result::Result<&[u8], HipcParseError> {
    let end = checked_end(offset, length, section)?;
    data.get(offset..end).ok_or(HipcParseError::Truncated {
        section,
        expected_at_least: end,
        actual: data.len(),
    })
}

fn read_u32(
    data: &[u8],
    offset: usize,
    section: HipcSection,
) -> std::result::Result<u32, HipcParseError> {
    Ok(u32::from_le_bytes(
        require_range(data, offset, 4, section)?
            .try_into()
            .expect("range has the requested length"),
    ))
}

fn read_u64(
    data: &[u8],
    offset: usize,
    section: HipcSection,
) -> std::result::Result<u64, HipcParseError> {
    Ok(u64::from_le_bytes(
        require_range(data, offset, 8, section)?
            .try_into()
            .expect("range has the requested length"),
    ))
}

pub fn parse_header(data: &[u8]) -> std::result::Result<HipcHeader, HipcParseError> {
    require_range(data, 0, 8, HipcSection::Header)?;
    let header0 = read_u32(data, 0, HipcSection::Header)?;
    let header1 = read_u32(data, 4, HipcSection::Header)?;

    Ok(HipcHeader {
        tag: (header0 & 0xffff) as u16,
        pointer_count: ((header0 >> 16) & 0xf) as u8,
        send_count: ((header0 >> 20) & 0xf) as u8,
        receive_count: ((header0 >> 24) & 0xf) as u8,
        exchange_count: ((header0 >> 28) & 0xf) as u8,
        raw_count: (header1 & 0x3ff) as u16,
        receive_list_count: ((header1 >> 10) & 0xf) as u8,
        receive_list_offset: ((header1 >> 20) & 0x7ff) as u16,
        special_header: (header1 >> 31) != 0,
    })
}

fn parse_special_header(
    data: &[u8],
    offset: &mut usize,
) -> std::result::Result<HipcSpecialHeader, HipcParseError> {
    let tag = read_u32(data, *offset, HipcSection::SpecialHeader)?;
    *offset = checked_end(*offset, 4, HipcSection::SpecialHeader)?;

    let has_process_id = tag & 1 != 0;
    let copy_handle_count = ((tag >> 1) & 0xf) as usize;
    let move_handle_count = ((tag >> 5) & 0xf) as usize;
    let process_id = if has_process_id {
        let process_id = read_u64(data, *offset, HipcSection::SpecialHeader)?;
        *offset = checked_end(*offset, 8, HipcSection::SpecialHeader)?;
        Some(process_id)
    } else {
        None
    };

    let mut copy_handles = Vec::with_capacity(copy_handle_count);
    for _ in 0..copy_handle_count {
        copy_handles.push(read_u32(data, *offset, HipcSection::SpecialHeader)?);
        *offset = checked_end(*offset, 4, HipcSection::SpecialHeader)?;
    }

    let mut move_handles = Vec::with_capacity(move_handle_count);
    for _ in 0..move_handle_count {
        move_handles.push(read_u32(data, *offset, HipcSection::SpecialHeader)?);
        *offset = checked_end(*offset, 4, HipcSection::SpecialHeader)?;
    }

    Ok(HipcSpecialHeader {
        process_id,
        copy_handles,
        move_handles,
    })
}

fn parse_pointer_descriptor(
    data: &[u8],
    offset: &mut usize,
) -> std::result::Result<HipcPointerDescriptor, HipcParseError> {
    let tag = read_u32(data, *offset, HipcSection::PointerDescriptors)?;
    let address_low = read_u32(
        data,
        checked_end(*offset, 4, HipcSection::PointerDescriptors)?,
        HipcSection::PointerDescriptors,
    )?;
    *offset = checked_end(*offset, 8, HipcSection::PointerDescriptors)?;

    let address = u64::from(address_low)
        | (u64::from((tag >> 12) & 0xf) << 32)
        | (u64::from((tag >> 6) & 0x7) << 36);
    Ok(HipcPointerDescriptor {
        index: (tag & 0xf) as u8,
        address,
        size: (tag >> 16) as u16,
    })
}

fn parse_map_descriptor(
    data: &[u8],
    offset: &mut usize,
    section: HipcSection,
) -> std::result::Result<HipcMapDescriptor, HipcParseError> {
    let size_low = read_u32(data, *offset, section)?;
    let address_low = read_u32(data, checked_end(*offset, 4, section)?, section)?;
    let tag = read_u32(data, checked_end(*offset, 8, section)?, section)?;
    *offset = checked_end(*offset, 12, section)?;

    let address = u64::from(address_low)
        | (u64::from((tag >> 28) & 0xf) << 32)
        | (u64::from((tag >> 2) & 0x7) << 36);
    let size = u64::from(size_low) | (u64::from((tag >> 24) & 0xf) << 32);
    Ok(HipcMapDescriptor {
        address,
        size,
        allows_non_secure: tag & 1 != 0,
        allows_non_device: tag & 2 != 0,
    })
}

/// Parses the HIPC header, descriptors, and raw data.
///
/// Receive-list data is left in `trailing_data`; its placement is controlled by
/// the receive-list offset and alignment rules and is not decoded here.
pub fn parse_message(data: &[u8]) -> std::result::Result<HipcMessage<'_>, HipcParseError> {
    let header = parse_header(data)?;
    let mut offset = 8;
    let special_header = if header.special_header {
        Some(parse_special_header(data, &mut offset)?)
    } else {
        None
    };

    let mut pointer_descriptors = Vec::with_capacity(header.pointer_count as usize);
    for _ in 0..header.pointer_count {
        pointer_descriptors.push(parse_pointer_descriptor(data, &mut offset)?);
    }

    let mut send_descriptors = Vec::with_capacity(header.send_count as usize);
    for _ in 0..header.send_count {
        send_descriptors.push(parse_map_descriptor(
            data,
            &mut offset,
            HipcSection::SendDescriptors,
        )?);
    }

    let mut receive_descriptors = Vec::with_capacity(header.receive_count as usize);
    for _ in 0..header.receive_count {
        receive_descriptors.push(parse_map_descriptor(
            data,
            &mut offset,
            HipcSection::ReceiveDescriptors,
        )?);
    }

    let mut exchange_descriptors = Vec::with_capacity(header.exchange_count as usize);
    for _ in 0..header.exchange_count {
        exchange_descriptors.push(parse_map_descriptor(
            data,
            &mut offset,
            HipcSection::ExchangeDescriptors,
        )?);
    }

    let raw_length =
        usize::from(header.raw_count)
            .checked_mul(4)
            .ok_or(HipcParseError::LengthOverflow {
                section: HipcSection::RawData,
            })?;
    let raw_end = checked_end(offset, raw_length, HipcSection::RawData)?;
    let raw_data = require_range(data, offset, raw_length, HipcSection::RawData)?;

    Ok(HipcMessage {
        header,
        special_header,
        pointer_descriptors,
        send_descriptors,
        receive_descriptors,
        exchange_descriptors,
        raw_data,
        trailing_data: &data[raw_end..],
    })
}

pub fn invoke_method<F>(data: &[u8], f: F) -> Result
where
    F: FnOnce(HipcHeader) -> Result,
{
    let Ok(header) = parse_header(data) else {
        return Result::Failure;
    };

    f(header)
}

#[cfg(test)]
mod tests {
    use super::{
        HipcHeader, HipcMapDescriptor, HipcParseError, HipcPointerDescriptor, HipcSection,
        HipcSpecialHeader, Result, invoke_method, parse_header, parse_message,
    };

    fn encode_header(header0: u32, header1: u32) -> [u8; 8] {
        let mut data = [0; 8];
        data[..4].copy_from_slice(&header0.to_le_bytes());
        data[4..].copy_from_slice(&header1.to_le_bytes());
        data
    }

    fn request_header() -> [u8; 8] {
        let header0 = 0x1234u32 | (2 << 16) | (3 << 20) | (4 << 24) | (5 << 28);
        let header1 = 0x2aau32 | (6 << 10) | (0x555 << 20) | (1 << 31);
        encode_header(header0, header1)
    }

    fn message_with_descriptors() -> Vec<u8> {
        let header0 = 0x42u32 | (1 << 16) | (1 << 20) | (1 << 24) | (1 << 28);
        let header1 = 1 | (0x321 << 20) | (1 << 31);
        let mut data = encode_header(header0, header1).to_vec();

        let special_tag = 1u32 | (1 << 1) | (1 << 5);
        data.extend_from_slice(&special_tag.to_le_bytes());
        data.extend_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes());
        data.extend_from_slice(&0x11u32.to_le_bytes());
        data.extend_from_slice(&0x22u32.to_le_bytes());

        let pointer_tag = (0x1234u32 << 16) | (3 << 12) | (5 << 6) | 7;
        data.extend_from_slice(&pointer_tag.to_le_bytes());
        data.extend_from_slice(&0x89ab_cdefu32.to_le_bytes());

        let map0 = 0x5678_9abcu32;
        let map1 = 0x7654_3210u32;
        let map2 = 2u32 | (6 << 2) | (0xa << 24) | (0xb << 28);
        for _ in 0..3 {
            data.extend_from_slice(&map0.to_le_bytes());
            data.extend_from_slice(&map1.to_le_bytes());
            data.extend_from_slice(&map2.to_le_bytes());
        }

        data.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        data.extend_from_slice(&[0xee, 0xff]);
        data
    }

    #[test]
    fn parses_header_fields_and_receive_list_offset() {
        assert_eq!(
            parse_header(&request_header()),
            Ok(HipcHeader {
                tag: 0x1234,
                pointer_count: 2,
                send_count: 3,
                receive_count: 4,
                exchange_count: 5,
                raw_count: 0x2aa,
                receive_list_count: 6,
                receive_list_offset: 0x555,
                special_header: true,
            })
        );
    }

    #[test]
    fn reports_truncated_header_details() {
        assert_eq!(
            parse_header(&[0; 7]),
            Err(HipcParseError::Truncated {
                section: HipcSection::Header,
                expected_at_least: 8,
                actual: 7,
            })
        );
    }

    #[test]
    fn parses_zeroed_header() {
        assert_eq!(
            parse_header(&[0; 8]),
            Ok(HipcHeader {
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
        let header1 = 0x3ffu32 | (0xf << 10) | (0x7ff << 20) | (1 << 31);
        let data = encode_header(u32::MAX, header1);

        assert_eq!(
            parse_header(&data).expect("complete header"),
            HipcHeader {
                tag: 0xffff,
                pointer_count: 0xf,
                send_count: 0xf,
                receive_count: 0xf,
                exchange_count: 0xf,
                raw_count: 0x3ff,
                receive_list_count: 0xf,
                receive_list_offset: 0x7ff,
                special_header: true,
            }
        );
    }

    #[test]
    fn parses_special_header_descriptors_and_raw_data() {
        let data = message_with_descriptors();
        let message = parse_message(&data).expect("valid message");

        assert_eq!(
            message.special_header,
            Some(HipcSpecialHeader {
                process_id: Some(0x1122_3344_5566_7788),
                copy_handles: vec![0x11],
                move_handles: vec![0x22],
            })
        );
        assert_eq!(
            message.pointer_descriptors,
            [HipcPointerDescriptor {
                index: 7,
                address: 0x0053_89ab_cdef,
                size: 0x1234,
            }]
        );
        let expected_map = HipcMapDescriptor {
            address: 0x6b_7654_3210,
            size: 0xa_5678_9abc,
            allows_non_secure: false,
            allows_non_device: true,
        };
        assert_eq!(message.send_descriptors, [expected_map]);
        assert_eq!(message.receive_descriptors, [expected_map]);
        assert_eq!(message.exchange_descriptors, [expected_map]);
        assert_eq!(message.raw_data, [0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(message.trailing_data, [0xee, 0xff]);
    }

    #[test]
    fn parses_message_without_optional_sections() {
        let data = encode_header(0x1234, 0).to_vec();
        let message = parse_message(&data).expect("valid empty message");

        assert_eq!(message.header.tag, 0x1234);
        assert_eq!(message.special_header, None);
        assert!(message.pointer_descriptors.is_empty());
        assert!(message.send_descriptors.is_empty());
        assert!(message.receive_descriptors.is_empty());
        assert!(message.exchange_descriptors.is_empty());
        assert!(message.raw_data.is_empty());
        assert!(message.trailing_data.is_empty());
    }

    #[test]
    fn reports_truncated_special_header_and_descriptors() {
        let mut data = encode_header(0, 1 << 31).to_vec();
        assert!(matches!(
            parse_message(&data),
            Err(HipcParseError::Truncated {
                section: HipcSection::SpecialHeader,
                ..
            })
        ));

        data.extend_from_slice(&0u32.to_le_bytes());
        let header = encode_header(1 << 16, 0);
        data[..8].copy_from_slice(&header);
        assert!(matches!(
            parse_message(&data),
            Err(HipcParseError::Truncated {
                section: HipcSection::PointerDescriptors,
                ..
            })
        ));
    }

    #[test]
    fn reports_truncated_special_header_pid_and_handles() {
        let mut data = encode_header(0, 1 << 31).to_vec();
        data.extend_from_slice(&1u32.to_le_bytes());
        assert!(matches!(
            parse_message(&data),
            Err(HipcParseError::Truncated {
                section: HipcSection::SpecialHeader,
                ..
            })
        ));

        data.truncate(8);
        data.extend_from_slice(&(1u32 << 1).to_le_bytes());
        assert!(matches!(
            parse_message(&data),
            Err(HipcParseError::Truncated {
                section: HipcSection::SpecialHeader,
                ..
            })
        ));
    }

    #[test]
    fn reports_truncated_map_descriptor_sections() {
        for (header0, section) in [
            (1 << 20, HipcSection::SendDescriptors),
            (1 << 24, HipcSection::ReceiveDescriptors),
            (1 << 28, HipcSection::ExchangeDescriptors),
        ] {
            assert!(matches!(
                parse_message(&encode_header(header0, 0)),
                Err(HipcParseError::Truncated {
                    section: actual_section,
                    ..
                }) if actual_section == section
            ));
        }
    }

    #[test]
    fn reports_truncated_raw_data() {
        let data = encode_header(0, 1);
        assert!(matches!(
            parse_message(&data),
            Err(HipcParseError::Truncated {
                section: HipcSection::RawData,
                ..
            })
        ));
    }

    #[test]
    fn rejects_every_truncated_header_without_invoking_handler() {
        let called = std::cell::Cell::new(false);
        let data = request_header();
        for length in 0..data.len() {
            assert!(parse_header(&data[..length]).is_err());
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
