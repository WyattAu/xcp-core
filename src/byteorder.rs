//! Byte order (endianness) per the XCP protocol layer.
//!
//! A slave advertises its byte order in the CONNECT response
//! (`COMM_MODE_BASIC` bit 0: `0` = Intel / little-endian, `1` = Motorola /
//! big-endian). Every multi-byte field the master sends or receives —
//! `SET_MTA` addresses, `MAX_DTO`, DAQ list numbers, event channels,
//! timestamps — is encoded in that order. Address granularity is carried
//! alongside it (`BYTE`, `WORD`, `DWORD`).

use alloc::vec::Vec;
use core::fmt;

/// The two byte orders a slave may advertise, per XCP part 1, §CONNECT
/// (`COMM_MODE_BASIC` bit 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ByteOrder {
    /// Intel / little-endian: least significant byte first (`0x00800000`
    /// → `00 00 80 00`). The common case on CAN-attached ECUs; the crate's
    /// default where a builder is offered without an explicit order.
    #[default]
    Intel,
    /// Motorola / big-endian: most significant byte first (`0x00800000` →
    /// `00 80 00 00`).
    Motorola,
}

impl ByteOrder {
    /// Decode from the `COMM_MODE_BASIC` bit 0 value (`0` = Intel, `1` =
    /// Motorola).
    #[must_use]
    pub const fn from_bit(bit: u8) -> Self {
        if bit & 0x01 == 0x01 {
            Self::Motorola
        } else {
            Self::Intel
        }
    }

    /// Encode to the `COMM_MODE_BASIC` bit 0 value (`0` = Intel, `1` =
    /// Motorola).
    #[must_use]
    pub const fn to_bit(self) -> u8 {
        match self {
            Self::Intel => 0,
            Self::Motorola => 1,
        }
    }

    /// Append a `u16` in this byte order.
    pub fn put_u16(self, buf: &mut Vec<u8>, value: u16) {
        let [a, b] = value.to_be_bytes();
        if matches!(self, Self::Intel) {
            buf.extend_from_slice(&[b, a]);
        } else {
            buf.extend_from_slice(&[a, b]);
        }
    }

    /// Append a `u32` in this byte order.
    pub fn put_u32(self, buf: &mut Vec<u8>, value: u32) {
        let [a, b, c, d] = value.to_be_bytes();
        if matches!(self, Self::Intel) {
            buf.extend_from_slice(&[d, c, b, a]);
        } else {
            buf.extend_from_slice(&[a, b, c, d]);
        }
    }

    /// Read a `u16` from the front of `bytes` in this byte order. Returns
    /// `None` when fewer than two bytes are available.
    #[must_use]
    pub fn get_u16(self, bytes: &[u8]) -> Option<u16> {
        let &[first, second] = bytes.first_chunk::<2>()?;
        Some(match self {
            // Intel: least significant byte first.
            Self::Intel => u16::from_le_bytes([first, second]),
            // Motorola: most significant byte first.
            Self::Motorola => u16::from_be_bytes([first, second]),
        })
    }

    /// Read a `u32` from the front of `bytes` in this byte order. Returns
    /// `None` when fewer than four bytes are available.
    #[must_use]
    pub fn get_u32(self, bytes: &[u8]) -> Option<u32> {
        let b = bytes.first()?;
        let c = bytes.get(1)?;
        let d = bytes.get(2)?;
        let e = bytes.get(3)?;
        let (a0, a1, a2, a3) = match self {
            Self::Intel => (*e, *d, *c, *b),
            Self::Motorola => (*b, *c, *d, *e),
        };
        Some((u32::from(a0) << 24) | (u32::from(a1) << 16) | (u32::from(a2) << 8) | u32::from(a3))
    }
}

impl fmt::Display for ByteOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Intel => f.write_str("intel"),
            Self::Motorola => f.write_str("motorola"),
        }
    }
}

/// The address granularity a slave advertises in `COMM_MODE_BASIC` bits 1–2:
/// the unit in which `SET_MTA` addresses and element counts are counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AddressGranularity {
    /// Addresses count bytes (granularity code `0b00`).
    #[default]
    Byte,
    /// Addresses count 16-bit words (granularity code `0b01`).
    Word,
    /// Addresses count 32-bit dwords (granularity code `0b10`).
    Dword,
    /// Reserved granularity code (`0b11`) — accepted, never emitted.
    Reserved,
}

impl AddressGranularity {
    /// Decode from `COMM_MODE_BASIC` bits 1–2 (the two bits above the byte
    /// order bit).
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        match (bits >> 1) & 0x03 {
            0b01 => Self::Word,
            0b10 => Self::Dword,
            0b11 => Self::Reserved,
            _ => Self::Byte,
        }
    }

    /// Encode to `COMM_MODE_BASIC` bits 1–2.
    #[must_use]
    pub const fn to_bits(self) -> u8 {
        match self {
            Self::Byte => 0b00,
            Self::Word => 0b01,
            Self::Dword => 0b10,
            Self::Reserved => 0b11,
        }
    }
}
