//! CTO response parsing: RES (`0xFF`), ERR (`0xFE`), EV (`0xFD`), SERV
//! (`0xFC`).
//!
//! A positive (`RES`) response carries no echo of the command that
//! produced it — the layout is known only from the *outstanding* command.
//! [`parse_response_with`] takes that expectation explicitly
//! ([`Expect`]); [`parse_response`] infers it with a documented
//! exact-length heuristic (see its rustdoc for the collision table).

use crate::byteorder::{AddressGranularity, ByteOrder};
use crate::error::XcpError;
use crate::resource::{ResourceMode, SessionStatus};
use alloc::vec::Vec;

/// The response PID classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::exhaustive_enums)]
pub enum Pid {
    /// `0xFF` — positive response (RES).
    Res,
    /// `0xFE` — error response (ERR).
    Err,
    /// `0xFD` — asynchronous event (EV).
    Ev,
    /// `0xFC` — service request (SERV).
    Serv,
}

impl Pid {
    /// Decode a response PID byte.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0xFF => Self::Res,
            0xFE => Self::Err,
            0xFD => Self::Ev,
            0xFC => Self::Serv,
            _ => return None,
        })
    }
}

/// The command whose response to expect — disambiguates `RES` layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(clippy::exhaustive_enums)]
pub enum Expect {
    /// Response to CONNECT.
    Connect,
    /// Response to GET_STATUS.
    GetStatus,
    /// Response to GET_VERSION.
    GetVersion,
    /// Response to GET_ID.
    GetId,
    /// Response to UPLOAD / SHORT_UPLOAD (payload is the uploaded data).
    Upload,
    /// Response to START_STOP_DAQ_LIST (first PID of the list).
    StartStopDaqList,
    /// Response to GET_DAQ_CLOCK (free-running slave timestamp).
    GetDaqClock,
    /// No known outstanding command: apply the documented inference
    /// heuristic.
    #[default]
    Generic,
}

/// A parsed XCP CTO response packet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum XcpResponse {
    /// Positive CONNECT response: resource availability, basic comm mode,
    /// `MAX_CTO`/`MAX_DTO`, protocol and transport layer versions.
    Connect(ConnectInfo),
    /// Positive GET_STATUS response.
    GetStatus(StatusInfo),
    /// Positive GET_VERSION response (XCP ≥ 1.1).
    GetVersion(VersionInfo),
    /// Positive GET_ID response (identification string length; the string
    /// itself follows via UPLOAD in block mode).
    GetId(IdInfo),
    /// Positive UPLOAD / SHORT_UPLOAD response: the uploaded bytes.
    Upload(Vec<u8>),
    /// Positive START_STOP_DAQ_LIST response: the first PID assigned to
    /// the DAQ list's ODTs.
    StartStopDaqList {
        /// First packet identifier the slave assigned to the list.
        first_pid: u8,
    },
    /// Positive GET_DAQ_CLOCK response: the free-running slave timestamp.
    GetDaqClock {
        /// Slave DAQ clock in the slave's timestamp unit.
        timestamp: u32,
    },
    /// Any other positive response (DISCONNECT, SET_MTA, DOWNLOAD acks,
    /// GET_CAL_PAGE, …) with the bytes after the PID preserved — empty for
    /// a bare ack.
    Generic {
        /// Payload bytes after the RES PID.
        data: Vec<u8>,
    },
    /// Asynchronous slave event (EV, PID `0xFD`).
    Event {
        /// Event code (e.g. `0x06` DAQ overload, `0x05` CMD_PENDING).
        code: u8,
        /// Payload bytes after the event code.
        data: Vec<u8>,
    },
    /// Service request from the slave (SERV, PID `0xFC`).
    Service {
        /// Service code (e.g. `0x01` request reset).
        code: u8,
        /// Payload bytes after the service code.
        data: Vec<u8>,
    },
}

/// The CONNECT response payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectInfo {
    /// Resources the slave offers (CAL/PAG, DAQ, STIM, PGM, DBG).
    pub resource: ResourceMode,
    /// Basic communication mode: byte order, address granularity, block
    /// mode support.
    pub comm_mode: CommModeBasic,
    /// Maximum CTO (command/response) packet size in bytes — 8 for classic
    /// XCP on CAN.
    pub max_cto: u8,
    /// Maximum DTO (DAQ/STIM) packet size in bytes.
    pub max_dto: u16,
    /// XCP protocol layer version (major, minor) — the CONNECT response
    /// carries only the major; minor is `0`.
    pub protocol_version: (u8, u8),
    /// XCP transport layer version (major, minor) — `(1, 0)` for XCP on
    /// CAN; the response carries only the major.
    pub transport_version: (u8, u8),
}

/// The `COMM_MODE_BASIC` byte of the CONNECT response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommModeBasic {
    /// Byte order for all multi-byte fields (Intel = little-endian /
    /// Motorola = big-endian).
    pub byte_order: ByteOrder,
    /// Address granularity (byte / word / dword).
    pub address_granularity: AddressGranularity,
    /// Slave block mode supported on the current transport.
    pub slave_block_mode: bool,
    /// `true` when the slave supports GET_COMM_MODE_INFO (optional info
    /// available).
    pub optional_info: bool,
}

impl CommModeBasic {
    /// Decode from the raw `COMM_MODE_BASIC` byte.
    #[must_use]
    pub const fn from_raw(raw: u8) -> Self {
        Self {
            byte_order: ByteOrder::from_bit(raw),
            address_granularity: AddressGranularity::from_bits(raw),
            slave_block_mode: raw & 0x40 != 0,
            optional_info: raw & 0x80 != 0,
        }
    }

    /// Encode to the raw `COMM_MODE_BASIC` byte.
    #[must_use]
    pub const fn to_raw(self) -> u8 {
        self.byte_order.to_bit()
            | (self.address_granularity.to_bits() << 1)
            | if self.slave_block_mode { 0x40 } else { 0 }
            | if self.optional_info { 0x80 } else { 0 }
    }
}

/// The GET_STATUS response payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusInfo {
    /// Session status flags (store/clear requests, DAQ running, resume).
    pub session_status: SessionStatus,
    /// Resources currently protected (seed & key required to unlock).
    pub protection_status: ResourceMode,
    /// Slave session configuration ID.
    pub session_cfg_id: u16,
}

/// The GET_VERSION response payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VersionInfo {
    /// XCP protocol layer version (major, minor).
    pub protocol_version: (u8, u8),
    /// XCP transport layer version (major, minor).
    pub transport_version: (u8, u8),
}

/// The GET_ID response payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdInfo {
    /// Echo of the request mode (`0` = ASCII text).
    pub mode: u8,
    /// Length of the identification data in bytes; retrieve with UPLOAD.
    pub length: u32,
}

/// Parse an XCP CTO response packet with Intel (little-endian) byte order
/// and no outstanding-command context — the inference heuristic of
/// [`Expect::Generic`].
///
/// Prefer [`parse_response_with`] when the outstanding command is known;
/// the heuristic is length-based and documented in [`Expect`].
///
/// # Errors
///
/// - [`XcpError::BufferTooShort`] — the packet is empty, or an ERR/EV/
///   SERV packet without its code byte, or a `RES` payload too short for
///   the expected layout.
/// - [`XcpError::UnknownPid`] — the first byte is not a response PID.
/// - Any slave ERR code variant ([`XcpError::CmdUnknown`], …) — the slave
///   answered with a negative response.
pub fn parse_response(bytes: &[u8]) -> Result<XcpResponse, XcpError> {
    parse_response_with(ByteOrder::Intel, Expect::Generic, bytes)
}

/// Parse an XCP CTO response packet with explicit byte order and the
/// outstanding command ([`Expect`]).
///
/// The parser is **total**: trailing bytes beyond a layout are tolerated
/// (CAN frames may carry DLC padding), and every input resolves to a
/// typed response or a typed error.
///
/// # Errors
///
/// See [`parse_response`].
pub fn parse_response_with(
    byte_order: ByteOrder,
    expect: Expect,
    bytes: &[u8],
) -> Result<XcpResponse, XcpError> {
    let Some((&pid, rest)) = bytes.split_first() else {
        return Err(XcpError::BufferTooShort { need: 1, have: 0 });
    };
    match pid {
        0xFF => parse_res(byte_order, expect, rest),
        0xFE => Err(crate::error::slave_error(rest)?),
        0xFD => {
            let Some((&code, data)) = rest.split_first() else {
                return Err(XcpError::BufferTooShort { need: 2, have: 1 });
            };
            Ok(XcpResponse::Event {
                code,
                data: Vec::from(data),
            })
        }
        0xFC => {
            let Some((&code, data)) = rest.split_first() else {
                return Err(XcpError::BufferTooShort { need: 2, have: 1 });
            };
            Ok(XcpResponse::Service {
                code,
                data: Vec::from(data),
            })
        }
        other => Err(XcpError::UnknownPid(other)),
    }
}

/// Parse a positive (RES, `0xFF`) payload according to `expect`, or by
/// heuristic when [`Expect::Generic`].
fn parse_res(byte_order: ByteOrder, expect: Expect, rest: &[u8]) -> Result<XcpResponse, XcpError> {
    let short = |need: usize| XcpError::BufferTooShort {
        need,
        have: rest.len() + 1,
    };
    match expect {
        Expect::Connect => connect_info(byte_order, rest)
            .map(XcpResponse::Connect)
            .ok_or_else(|| short(8)),
        Expect::GetStatus => status_info(byte_order, rest)
            .map(XcpResponse::GetStatus)
            .ok_or_else(|| short(6)),
        Expect::GetVersion => version_info(rest)
            .map(XcpResponse::GetVersion)
            .ok_or_else(|| short(6)),
        Expect::GetId => id_info(byte_order, rest)
            .map(XcpResponse::GetId)
            .ok_or_else(|| short(8)),
        Expect::Upload => Ok(XcpResponse::Upload(Vec::from(rest))),
        Expect::StartStopDaqList => match rest.first_chunk::<1>() {
            Some(&[first_pid]) => Ok(XcpResponse::StartStopDaqList { first_pid }),
            None => Err(short(2)),
        },
        Expect::GetDaqClock => daq_clock(byte_order, rest)
            .map(|timestamp| XcpResponse::GetDaqClock { timestamp })
            .ok_or_else(|| short(8)),
        Expect::Generic => Ok(generic_res(byte_order, rest)),
    }
}

/// The documented inference heuristic for context-free RES packets:
///
/// | total length | inferred response | collision risk |
/// |---|---|---|
/// | 8 | CONNECT ([`ConnectInfo`]) | UPLOAD of 7 bytes, GET_ID, GET_DAQ_CLOCK |
/// | 6, first payload byte `0x00` | GET_VERSION | UPLOAD of 5 zero bytes |
/// | 6 | GET_STATUS | UPLOAD of 5 bytes |
/// | anything else | `Generic { data }` | bare acks, small uploads |
///
/// The first payload byte of a GET_VERSION response is the reserved
/// `0x00`, which a GET_STATUS `SESSION_STATUS` byte never is when any
/// flag is set. Masters driving GET_ID, GET_DAQ_CLOCK, or large UPLOADs
/// should use [`Expect`] instead of the heuristic.
fn generic_res(byte_order: ByteOrder, rest: &[u8]) -> XcpResponse {
    let generic = || XcpResponse::Generic {
        data: Vec::from(rest),
    };
    match rest.len() {
        7 => connect_info(byte_order, rest).map_or_else(generic, XcpResponse::Connect),
        5 => match rest.first() {
            Some(0x00) => version_info(rest).map_or_else(generic, XcpResponse::GetVersion),
            _ => status_info(byte_order, rest).map_or_else(generic, XcpResponse::GetStatus),
        },
        _ => generic(),
    }
}

/// Decode the CONNECT payload (7 bytes after the PID).
fn connect_info(byte_order: ByteOrder, rest: &[u8]) -> Option<ConnectInfo> {
    let &[resource, cmb, max_cto, d0, d1, pver, tver] = rest.first_chunk::<7>()?;
    Some(ConnectInfo {
        resource: ResourceMode::from_raw(resource),
        comm_mode: CommModeBasic::from_raw(cmb),
        max_cto,
        max_dto: word16(byte_order, d0, d1),
        protocol_version: (pver, 0),
        transport_version: (tver, 0),
    })
}

/// Decode the GET_STATUS payload (5 bytes after the PID).
fn status_info(byte_order: ByteOrder, rest: &[u8]) -> Option<StatusInfo> {
    let &[session, protection, _reserved, c0, c1] = rest.first_chunk::<5>()?;
    Some(StatusInfo {
        session_status: SessionStatus::from_raw(session),
        protection_status: ResourceMode::from_raw(protection),
        session_cfg_id: word16(byte_order, c0, c1),
    })
}

/// Decode the GET_VERSION payload (5 bytes after the PID: reserved, then
/// the four version bytes).
fn version_info(rest: &[u8]) -> Option<VersionInfo> {
    let &[_reserved, pmaj, pmin, tmaj, tmin] = rest.first_chunk::<5>()?;
    Some(VersionInfo {
        protocol_version: (pmaj, pmin),
        transport_version: (tmaj, tmin),
    })
}

/// Decode the GET_ID payload (7 bytes after the PID).
fn id_info(byte_order: ByteOrder, rest: &[u8]) -> Option<IdInfo> {
    let &[mode, _r0, _r1, a0, a1, a2, a3] = rest.first_chunk::<7>()?;
    Some(IdInfo {
        mode,
        length: word32(byte_order, [a0, a1, a2, a3]),
    })
}

/// Decode the GET_DAQ_CLOCK payload (7 bytes after the PID: three
/// reserved, then the timestamp).
fn daq_clock(byte_order: ByteOrder, rest: &[u8]) -> Option<u32> {
    let &[_r0, _r1, _r2, t0, t1, t2, t3] = rest.first_chunk::<7>()?;
    Some(word32(byte_order, [t0, t1, t2, t3]))
}

/// Assemble a `u16` from its wire bytes in the given order.
const fn word16(byte_order: ByteOrder, b0: u8, b1: u8) -> u16 {
    match byte_order {
        ByteOrder::Intel => u16::from_le_bytes([b0, b1]),
        ByteOrder::Motorola => u16::from_be_bytes([b0, b1]),
    }
}

/// Assemble a `u32` from its wire bytes in the given order.
const fn word32(byte_order: ByteOrder, b: [u8; 4]) -> u32 {
    match byte_order {
        ByteOrder::Intel => u32::from_le_bytes(b),
        ByteOrder::Motorola => u32::from_be_bytes(b),
    }
}
