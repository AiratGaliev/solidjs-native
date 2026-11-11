use crate::{HandlerId, NodeId, PropertyValue, TreeError};
use thiserror::Error;

pub const HEADER_SIZE: usize = 64;
pub const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_OPERATIONS: usize = 65_535;
pub const MAX_STRING_BYTES: usize = 1024 * 1024;
const MAGIC: &[u8; 4] = b"SNHB";
const PROTOCOL_MAJOR: u16 = 1;
const PROTOCOL_MINOR: u16 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchHeader {
    pub major: u16,
    pub minor: u16,
    pub surface_id: u64,
    pub session_generation: u64,
    pub transaction_id: u64,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostOperation {
    CreateElement {
        id: NodeId,
        tag: String,
    },
    CreateText {
        id: NodeId,
        text: String,
    },
    ReplaceText {
        id: NodeId,
        text: String,
    },
    SetProperty {
        id: NodeId,
        name: String,
        value: PropertyValue,
    },
    RemoveProperty {
        id: NodeId,
        name: String,
    },
    InsertNode {
        parent: NodeId,
        child: NodeId,
        anchor: Option<NodeId>,
    },
    RemoveNode {
        id: NodeId,
    },
    BindEvent {
        id: NodeId,
        event: String,
        handler_id: HandlerId,
    },
    UnbindEvent {
        id: NodeId,
        event: String,
    },
    AttachResource {
        id: NodeId,
        name: String,
        resource_id: u64,
    },
    DetachResource {
        id: NodeId,
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostBatch {
    pub header: BatchHeader,
    pub operations: Vec<HostOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueKind {
    Null = 0,
    Boolean = 1,
    Number = 2,
    String = 3,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("batch is shorter than the fixed header")]
    TruncatedHeader,
    #[error("batch exceeds the configured byte limit")]
    OversizedBatch,
    #[error("invalid protocol magic")]
    InvalidMagic,
    #[error("unsupported host protocol version {major}.{minor}")]
    UnsupportedVersion { major: u16, minor: u16 },
    #[error("invalid header size or reserved flags")]
    InvalidHeader,
    #[error("batch contains no operations")]
    EmptyBatch,
    #[error("operation count exceeds the configured limit")]
    TooManyOperations,
    #[error("payload length does not match the batch length")]
    InvalidPayloadLength,
    #[error("string arena is outside the batch or overlaps the operation section")]
    InvalidStringArena,
    #[error("operation record is truncated or has an invalid size")]
    InvalidRecord,
    #[error("unknown operation code {0}")]
    UnknownOpcode(u8),
    #[error("invalid value kind {0}")]
    UnknownValueKind(u8),
    #[error("reserved bits must be zero")]
    ReservedBits,
    #[error("invalid string reference")]
    InvalidStringReference,
    #[error("string is not valid UTF-8")]
    InvalidUtf8,
    #[error("numeric values must be finite")]
    NonFiniteNumber,
    #[error("invalid boolean encoding")]
    InvalidBoolean,
    #[error("invalid zero identifier")]
    InvalidIdentifier,
    #[error("invalid host protocol field: {0}")]
    InvalidField(&'static str),
}

impl From<TreeError> for ProtocolError {
    fn from(value: TreeError) -> Self {
        Self::InvalidField(match value {
            TreeError::InvalidName(_) => "name",
            TreeError::InvalidNodeId => "node id",
            TreeError::InvalidHandlerId => "handler id",
            TreeError::InvalidResourceId => "resource id",
            _ => "tree operation",
        })
    }
}

pub fn decode_batch(bytes: &[u8]) -> Result<HostBatch, ProtocolError> {
    if bytes.len() < HEADER_SIZE {
        return Err(ProtocolError::TruncatedHeader);
    }
    if bytes.len() > MAX_BATCH_BYTES {
        return Err(ProtocolError::OversizedBatch);
    }
    if bytes.get(0..4) != Some(MAGIC.as_slice()) {
        return Err(ProtocolError::InvalidMagic);
    }

    let major = read_u16(bytes, 4)?;
    let minor = read_u16(bytes, 6)?;
    if major != PROTOCOL_MAJOR || minor > PROTOCOL_MINOR {
        return Err(ProtocolError::UnsupportedVersion { major, minor });
    }
    if read_u16(bytes, 8)? as usize != HEADER_SIZE
        || read_u16(bytes, 10)? != 0
        || read_u32(bytes, 60)? != 0
    {
        return Err(ProtocolError::InvalidHeader);
    }

    let surface_id = read_u64(bytes, 12)?;
    let session_generation = read_u64(bytes, 20)?;
    let transaction_id = read_u64(bytes, 28)?;
    let expected_revision = read_u64(bytes, 36)?;
    if surface_id == 0 || session_generation == 0 || transaction_id == 0 {
        return Err(ProtocolError::InvalidIdentifier);
    }
    let operation_count = read_u32(bytes, 44)? as usize;
    if operation_count == 0 {
        return Err(ProtocolError::EmptyBatch);
    }
    if operation_count > MAX_OPERATIONS {
        return Err(ProtocolError::TooManyOperations);
    }

    let payload_size = read_u32(bytes, 48)? as usize;
    if payload_size
        != bytes
            .len()
            .checked_sub(HEADER_SIZE)
            .ok_or(ProtocolError::InvalidPayloadLength)?
    {
        return Err(ProtocolError::InvalidPayloadLength);
    }
    let string_offset = read_u32(bytes, 52)? as usize;
    let string_length = read_u32(bytes, 56)? as usize;
    let string_end = string_offset
        .checked_add(string_length)
        .ok_or(ProtocolError::InvalidStringArena)?;
    if string_offset < HEADER_SIZE || string_end != bytes.len() {
        return Err(ProtocolError::InvalidStringArena);
    }
    let mut cursor = HEADER_SIZE;
    let mut operations = Vec::with_capacity(operation_count);
    for _ in 0..operation_count {
        let (opcode, record) = next_record(bytes, &mut cursor, string_offset)?;
        operations.push(decode_operation(
            opcode,
            record,
            bytes,
            string_offset,
            string_length,
        )?);
    }
    if cursor != string_offset {
        return Err(ProtocolError::InvalidRecord);
    }

    Ok(HostBatch {
        header: BatchHeader {
            major,
            minor,
            surface_id,
            session_generation,
            transaction_id,
            expected_revision,
        },
        operations,
    })
}

fn next_record<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    records_end: usize,
) -> Result<(u8, &'a [u8]), ProtocolError> {
    let header_end = (*cursor)
        .checked_add(8)
        .ok_or(ProtocolError::InvalidRecord)?;
    if header_end > records_end {
        return Err(ProtocolError::InvalidRecord);
    }
    let opcode = bytes[*cursor];
    if bytes[*cursor + 1] != 0 || read_u16(bytes, *cursor + 2)? != 0 {
        return Err(ProtocolError::ReservedBits);
    }
    let size = read_u32(bytes, *cursor + 4)? as usize;
    if size < 8 {
        return Err(ProtocolError::InvalidRecord);
    }
    let end = (*cursor)
        .checked_add(size)
        .ok_or(ProtocolError::InvalidRecord)?;
    if end > records_end {
        return Err(ProtocolError::InvalidRecord);
    }
    let record = &bytes[*cursor..end];
    *cursor = end;
    Ok((opcode, record))
}

fn decode_operation(
    opcode: u8,
    record: &[u8],
    bytes: &[u8],
    arena_offset: usize,
    arena_length: usize,
) -> Result<HostOperation, ProtocolError> {
    match opcode {
        1 => {
            expect_size(record, 24)?;
            Ok(HostOperation::CreateElement {
                id: read_node_id(record, 8)?,
                tag: read_string_ref(record, 16, bytes, arena_offset, arena_length)?,
            })
        }
        2 => {
            expect_size(record, 24)?;
            Ok(HostOperation::CreateText {
                id: read_node_id(record, 8)?,
                text: read_string_ref(record, 16, bytes, arena_offset, arena_length)?,
            })
        }
        3 => {
            expect_size(record, 24)?;
            Ok(HostOperation::ReplaceText {
                id: read_node_id(record, 8)?,
                text: read_string_ref(record, 16, bytes, arena_offset, arena_length)?,
            })
        }
        4 => {
            expect_size(record, 40)?;
            let id = read_node_id(record, 8)?;
            let name = read_string_ref(record, 16, bytes, arena_offset, arena_length)?;
            let kind = record[24];
            if record[25..32].iter().any(|byte| *byte != 0) {
                return Err(ProtocolError::ReservedBits);
            }
            let raw = read_u64(record, 32)?;
            let value = match kind {
                0 if raw == 0 => PropertyValue::Null,
                1 if raw <= 1 => PropertyValue::Boolean(raw == 1),
                1 => return Err(ProtocolError::InvalidBoolean),
                2 => {
                    let number = f64::from_bits(raw);
                    if !number.is_finite() {
                        return Err(ProtocolError::NonFiniteNumber);
                    }
                    PropertyValue::Number(number)
                }
                3 => {
                    let offset = raw as u32;
                    let length = (raw >> 32) as u32;
                    PropertyValue::String(read_string_at(
                        offset,
                        length,
                        bytes,
                        arena_offset,
                        arena_length,
                    )?)
                }
                other => return Err(ProtocolError::UnknownValueKind(other)),
            };
            Ok(HostOperation::SetProperty { id, name, value })
        }
        5 => {
            expect_size(record, 24)?;
            Ok(HostOperation::RemoveProperty {
                id: read_node_id(record, 8)?,
                name: read_string_ref(record, 16, bytes, arena_offset, arena_length)?,
            })
        }
        6 => {
            expect_size(record, 32)?;
            let parent = read_node_id(record, 8)?;
            let child = read_node_id(record, 16)?;
            let anchor_raw = read_u64(record, 24)?;
            let anchor = if anchor_raw == 0 {
                None
            } else {
                Some(NodeId::new(anchor_raw))
            };
            Ok(HostOperation::InsertNode {
                parent,
                child,
                anchor,
            })
        }
        7 => {
            expect_size(record, 16)?;
            Ok(HostOperation::RemoveNode {
                id: read_node_id(record, 8)?,
            })
        }
        8 => {
            expect_size(record, 32)?;
            let id = read_node_id(record, 8)?;
            let event = read_string_ref(record, 16, bytes, arena_offset, arena_length)?;
            let handler = read_u32(record, 24)?;
            if read_u32(record, 28)? != 0 {
                return Err(ProtocolError::ReservedBits);
            }
            if handler == 0 {
                return Err(ProtocolError::InvalidIdentifier);
            }
            Ok(HostOperation::BindEvent {
                id,
                event,
                handler_id: HandlerId::new(handler),
            })
        }
        9 => {
            expect_size(record, 24)?;
            Ok(HostOperation::UnbindEvent {
                id: read_node_id(record, 8)?,
                event: read_string_ref(record, 16, bytes, arena_offset, arena_length)?,
            })
        }
        10 => {
            expect_size(record, 32)?;
            let id = read_node_id(record, 8)?;
            let name = read_string_ref(record, 16, bytes, arena_offset, arena_length)?;
            let resource_id = read_u64(record, 24)?;
            if resource_id == 0 {
                return Err(ProtocolError::InvalidIdentifier);
            }
            Ok(HostOperation::AttachResource {
                id,
                name,
                resource_id,
            })
        }
        11 => {
            expect_size(record, 24)?;
            Ok(HostOperation::DetachResource {
                id: read_node_id(record, 8)?,
                name: read_string_ref(record, 16, bytes, arena_offset, arena_length)?,
            })
        }
        other => Err(ProtocolError::UnknownOpcode(other)),
    }
}

fn read_string_ref(
    record: &[u8],
    offset: usize,
    bytes: &[u8],
    arena_offset: usize,
    arena_length: usize,
) -> Result<String, ProtocolError> {
    read_string_at(
        read_u32(record, offset)?,
        read_u32(record, offset + 4)?,
        bytes,
        arena_offset,
        arena_length,
    )
}

fn read_string_at(
    offset: u32,
    length: u32,
    bytes: &[u8],
    arena_offset: usize,
    arena_length: usize,
) -> Result<String, ProtocolError> {
    let offset = offset as usize;
    let length = length as usize;
    if length > MAX_STRING_BYTES {
        return Err(ProtocolError::InvalidStringReference);
    }
    let relative_end = offset
        .checked_add(length)
        .ok_or(ProtocolError::InvalidStringReference)?;
    if relative_end > arena_length {
        return Err(ProtocolError::InvalidStringReference);
    }
    let start = arena_offset
        .checked_add(offset)
        .ok_or(ProtocolError::InvalidStringReference)?;
    let end = start
        .checked_add(length)
        .ok_or(ProtocolError::InvalidStringReference)?;
    let value = bytes
        .get(start..end)
        .ok_or(ProtocolError::InvalidStringReference)?;
    let value = std::str::from_utf8(value).map_err(|_| ProtocolError::InvalidUtf8)?;
    if value.contains('\0') {
        return Err(ProtocolError::InvalidStringReference);
    }
    Ok(value.to_owned())
}

fn read_node_id(record: &[u8], offset: usize) -> Result<NodeId, ProtocolError> {
    let value = read_u64(record, offset)?;
    if value == 0 {
        return Err(ProtocolError::InvalidIdentifier);
    }
    Ok(NodeId::new(value))
}

fn expect_size(record: &[u8], expected: usize) -> Result<(), ProtocolError> {
    if record.len() != expected {
        return Err(ProtocolError::InvalidRecord);
    }
    let declared = read_u32(record, 4)? as usize;
    if declared != expected {
        return Err(ProtocolError::InvalidRecord);
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ProtocolError> {
    let slice = bytes
        .get(offset..offset.checked_add(2).ok_or(ProtocolError::InvalidRecord)?)
        .ok_or(ProtocolError::InvalidRecord)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ProtocolError> {
    let slice = bytes
        .get(offset..offset.checked_add(4).ok_or(ProtocolError::InvalidRecord)?)
        .ok_or(ProtocolError::InvalidRecord)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ProtocolError> {
    let slice = bytes
        .get(offset..offset.checked_add(8).ok_or(ProtocolError::InvalidRecord)?)
        .ok_or(ProtocolError::InvalidRecord)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_batch() -> Vec<u8> {
        let mut bytes = vec![0u8; HEADER_SIZE + 24 + 4];
        bytes[0..4].copy_from_slice(MAGIC);
        bytes[4..6].copy_from_slice(&PROTOCOL_MAJOR.to_le_bytes());
        bytes[6..8].copy_from_slice(&PROTOCOL_MINOR.to_le_bytes());
        bytes[8..10].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
        bytes[12..20].copy_from_slice(&1u64.to_le_bytes());
        bytes[20..28].copy_from_slice(&1u64.to_le_bytes());
        bytes[28..36].copy_from_slice(&1u64.to_le_bytes());
        bytes[36..44].copy_from_slice(&0u64.to_le_bytes());
        bytes[44..48].copy_from_slice(&1u32.to_le_bytes());
        bytes[48..52].copy_from_slice(&(28u32).to_le_bytes());
        bytes[52..56].copy_from_slice(&(HEADER_SIZE as u32 + 24).to_le_bytes());
        bytes[56..60].copy_from_slice(&(4u32).to_le_bytes());
        bytes[64] = 1; // CreateElement
        bytes[68..72].copy_from_slice(&(24u32).to_le_bytes());
        bytes[72..80].copy_from_slice(&1u64.to_le_bytes());
        bytes[80..84].copy_from_slice(&0u32.to_le_bytes());
        bytes[84..88].copy_from_slice(&4u32.to_le_bytes());
        bytes[88..92].copy_from_slice(b"root");
        bytes
    }

    #[test]
    fn decodes_golden_create_element_fixture() {
        let batch = decode_batch(&sample_batch()).unwrap();
        assert_eq!(
            batch.operations,
            vec![HostOperation::CreateElement {
                id: NodeId::new(1),
                tag: "root".into()
            }]
        );
    }

    #[test]
    fn rejects_truncated_and_oversized_batches() {
        assert_eq!(decode_batch(&[0; 20]), Err(ProtocolError::TruncatedHeader));
        let mut huge = vec![0; MAX_BATCH_BYTES + 1];
        assert_eq!(decode_batch(&huge), Err(ProtocolError::OversizedBatch));
        huge.clear();
    }

    #[test]
    fn rejects_unknown_version_before_operations() {
        let mut bytes = sample_batch();
        bytes[4..6].copy_from_slice(&2u16.to_le_bytes());
        assert!(matches!(
            decode_batch(&bytes),
            Err(ProtocolError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn rejects_malformed_payload_and_string_ranges() {
        let mut bad_length = sample_batch();
        bad_length[48..52].copy_from_slice(&27u32.to_le_bytes());
        assert_eq!(
            decode_batch(&bad_length),
            Err(ProtocolError::InvalidPayloadLength)
        );

        let mut bad_range = sample_batch();
        bad_range[80..84].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            decode_batch(&bad_range),
            Err(ProtocolError::InvalidStringReference)
        );
    }

    #[test]
    fn rejects_unknown_opcode_and_reserved_flags() {
        let mut opcode = sample_batch();
        opcode[64] = 99;
        assert_eq!(decode_batch(&opcode), Err(ProtocolError::UnknownOpcode(99)));

        let mut flags = sample_batch();
        flags[65] = 1;
        assert_eq!(decode_batch(&flags), Err(ProtocolError::ReservedBits));
    }

    #[test]
    fn rejects_invalid_utf8() {
        let mut bytes = sample_batch();
        bytes[88] = 0xff;
        assert_eq!(decode_batch(&bytes), Err(ProtocolError::InvalidUtf8));
    }
}
