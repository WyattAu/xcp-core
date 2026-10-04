//! Master-side CTO command frame builders.
//!
//! Every builder returns the wire bytes of the command packet (CTO),
//! ready to be carried in a CAN frame by [`transport`](crate::transport)
//! or any other XCP transport. The calibration sequence builders
//! ([`upload`], [`download`]) return the **concatenated frame sequence** —
//! `SET_MTA` followed by the data-transfer command(s) — because XCP
//! addresses every transfer through the MTA. For per-frame transmission
//! use [`set_mta`] plus the single-frame builders ([`upload_frame`],
//! [`download_frames`]).
//!
//! Addresses are encoded in the byte order the slave advertised in the
//! CONNECT response ([`ByteOrder`]: Intel = little-endian, Motorola =
//! big-endian). The short-form builders [`upload`] / [`download`] use
//! Intel byte order and address extension `0x00` (the common case);
//! [`upload_with`] / [`download_with`] take both explicitly.
//!
//! # Example
//!
//! ```
//! use xcp_core::{connect, download, download_with, ByteOrder, ResourceMode};
//!
//! // CONNECT, normal mode:
//! assert_eq!(connect(ResourceMode::CONNECT_NORMAL), [0xFF, 0x00]);
//!
//! // SET_MTA + DOWNLOAD in Intel byte order (address little-endian):
//! let frames = download(0x4000_0000, &[0xDE, 0xAD]);
//! assert_eq!(
//!     frames[..10],
//!     [0xF6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xF0, 0x02][..]
//! );
//! assert_eq!(&frames[10..], &[0xDE, 0xAD]);
//!
//! // Same transfer with Motorola (big-endian) addressing:
//! let be = download_with(ByteOrder::Motorola, 0x4000_0000, 0, &[0xDE, 0xAD]);
//! assert_eq!(&be[4..8], &[0x40, 0x00, 0x00, 0x00]);
//! ```

use crate::byteorder::ByteOrder;
use crate::resource::ResourceMode;
use alloc::vec::Vec;

/// Largest `DOWNLOAD` payload in one CTO: the absolute protocol cap is
/// `MAX_CTO = 255`, minus the PID and the element-count byte.
pub const DOWNLOAD_MAX_CHUNK: usize = 253;
/// Largest `DOWNLOAD_NEXT` payload in one CTO: PID plus element-count byte.
pub const DOWNLOAD_NEXT_MAX_CHUNK: usize = 254;

/// Build the CONNECT command frame: `[0xFF, MODE]`.
///
/// Per the spec the mode byte is `0x00` (normal) or `0x01` (user-defined);
/// pass [`ResourceMode::CONNECT_NORMAL`] /
/// [`ResourceMode::CONNECT_USER_DEFINED`] (or a user-defined bit pattern).
#[must_use]
pub fn connect(mode: ResourceMode) -> Vec<u8> {
    alloc::vec![crate::XcpCommand::Connect.code(), mode.bits()]
}

/// Build the DISCONNECT command frame: `[0xFE]`.
#[must_use]
pub fn disconnect() -> Vec<u8> {
    alloc::vec![crate::XcpCommand::Disconnect.code()]
}

/// Build the GET_STATUS command frame: `[0xFD]`.
#[must_use]
pub fn get_status() -> Vec<u8> {
    alloc::vec![crate::XcpCommand::GetStatus.code()]
}

/// Build the SYNCH command frame: `[0xFC]`. The slave answers with ERR
/// `0x00` ([`XcpError::CmdSynch`](crate::XcpError::CmdSynch)) once the
/// command stream is synchronised.
#[must_use]
pub fn synch() -> Vec<u8> {
    alloc::vec![crate::XcpCommand::Synch.code()]
}

/// Build the GET_VERSION command frame: `[0xC0, 0x00]` (XCP ≥ 1.1).
#[must_use]
pub fn get_version() -> Vec<u8> {
    alloc::vec![crate::XcpCommand::GetVersion.code(), 0x00]
}

/// Build the GET_ID command frame: `[0xFA, MODE, 0x00, 0x00]`.
///
/// Mode `0` returns the ASCII ECU identification text.
#[must_use]
pub fn get_id(mode: u8) -> Vec<u8> {
    alloc::vec![crate::XcpCommand::GetId.code(), mode, 0x00, 0x00]
}

/// Build the GET_COMM_MODE_INFO command frame: `[0xFB]`.
#[must_use]
pub fn get_comm_mode_info() -> Vec<u8> {
    alloc::vec![crate::XcpCommand::GetCommModeInfo.code()]
}

/// Build a SET_MTA command frame: `[0xF6, 0x00, 0x00, EXT, ADDR₃…₀]`.
///
/// The address is encoded in `byte_order` — Intel (little-endian) or
/// Motorola (big-endian) — as advertised by the slave in the CONNECT
/// response.
#[must_use]
pub fn set_mta(byte_order: ByteOrder, addr: u32, addr_extension: u8) -> Vec<u8> {
    let mut f = alloc::vec![crate::XcpCommand::SetMta.code(), 0x00, 0x00, addr_extension];
    byte_order.put_u32(&mut f, addr);
    f
}

/// Build the SET_MTA + UPLOAD frame sequence for a plain memory read with
/// Intel (little-endian) addressing and address extension `0x00`.
///
/// `len` is the number of elements to read (address-granularity units;
/// bytes at byte granularity). It must fit one CTO — at most 254 elements
/// for a 255-byte `MAX_CTO`. Parse the slave's positive response with
/// [`parse_response_with`](crate::parse_response_with) and
/// [`Expect::Upload`](crate::Expect::Upload).
#[must_use]
pub fn upload(addr: u32, len: u8) -> Vec<u8> {
    upload_with(ByteOrder::Intel, addr, 0x00, len)
}

/// Build the SET_MTA + UPLOAD sequence with explicit byte order and
/// address extension.
#[must_use]
pub fn upload_with(byte_order: ByteOrder, addr: u32, addr_extension: u8, len: u8) -> Vec<u8> {
    let mut seq = set_mta(byte_order, addr, addr_extension);
    seq.extend_from_slice(&upload_frame(len));
    seq
}

/// Build the UPLOAD command frame: `[0xF5, LEN]` (reads from the current
/// MTA).
#[must_use]
pub fn upload_frame(len: u8) -> Vec<u8> {
    alloc::vec![crate::XcpCommand::Upload.code(), len]
}

/// Build the SHORT_UPLOAD command frame:
/// `[0xF4, LEN, 0x00, EXT, ADDR₃…₀]` — read without changing the MTA.
#[must_use]
pub fn short_upload(byte_order: ByteOrder, addr: u32, addr_extension: u8, len: u8) -> Vec<u8> {
    let mut f = alloc::vec![
        crate::XcpCommand::ShortUpload.code(),
        len,
        0x00,
        addr_extension
    ];
    byte_order.put_u32(&mut f, addr);
    f
}

/// Build the SET_MTA + DOWNLOAD frame sequence for a plain memory write
/// with Intel (little-endian) addressing and address extension `0x00`.
///
/// `data` is split into block-mode frames: one `DOWNLOAD`
/// (`[0xF0, N, …]`, up to [`DOWNLOAD_MAX_CHUNK`] bytes) followed by as
/// many `DOWNLOAD_NEXT` (`[0xEF, N, …]`, up to [`DOWNLOAD_NEXT_MAX_CHUNK`]
/// bytes) as needed, so a calibration table of any length encodes in one
/// call. The MTA advances with every chunk, exactly as the protocol does.
#[must_use]
pub fn download(addr: u32, data: &[u8]) -> Vec<u8> {
    download_with(ByteOrder::Intel, addr, 0x00, data)
}

/// Build the SET_MTA + DOWNLOAD sequence with explicit byte order and
/// address extension.
#[must_use]
pub fn download_with(byte_order: ByteOrder, addr: u32, addr_extension: u8, data: &[u8]) -> Vec<u8> {
    let mut seq = set_mta(byte_order, addr, addr_extension);
    seq.extend_from_slice(&download_frames(data));
    seq
}

/// Build the block-mode DOWNLOAD frame sequence (`DOWNLOAD` +
/// `DOWNLOAD_NEXT`s) without the `SET_MTA` prefix.
#[must_use]
pub fn download_frames(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 2 * (1 + data.len() / DOWNLOAD_MAX_CHUNK));
    let (first, rest) = data.split_at(data.len().min(DOWNLOAD_MAX_CHUNK));
    out.push(crate::XcpCommand::Download.code());
    #[allow(clippy::cast_possible_truncation)] // chunk ≤ 253 < 256
    out.push(first.len() as u8);
    out.extend_from_slice(first);
    for chunk in rest.chunks(DOWNLOAD_NEXT_MAX_CHUNK) {
        out.push(crate::XcpCommand::DownloadNext.code());
        #[allow(clippy::cast_possible_truncation)] // chunk ≤ 254 < 256
        out.push(chunk.len() as u8);
        out.extend_from_slice(chunk);
    }
    out
}

/// Build the SHORT_DOWNLOAD command frame:
/// `[0xED, 0x00, EXT, ADDR₃…₀, DATA…]` — write without changing the MTA.
/// The data must fit one CTO (at most 250 bytes after the 5-byte header
/// for a 255-byte `MAX_CTO`); longer writes belong in [`download`], which
/// chunks.
#[must_use]
pub fn short_download(
    byte_order: ByteOrder,
    addr: u32,
    addr_extension: u8,
    data: &[u8],
) -> Vec<u8> {
    let mut f = alloc::vec![
        crate::XcpCommand::ShortDownload.code(),
        0x00,
        addr_extension
    ];
    byte_order.put_u32(&mut f, addr);
    f.extend_from_slice(data);
    f
}

/// Build the SET_CAL_PAGE command frame: `[0xEB, MODE, SEG₂, SEG₁]`.
///
/// Mode bits: `0x01` switch CPU to page, `0x02` switch page on ECU, `0x80`
/// all segments. `segment` is the segment number (`0x0000` = all).
#[must_use]
pub fn set_cal_page(byte_order: ByteOrder, mode: u8, segment: u16) -> Vec<u8> {
    let mut f = alloc::vec![crate::XcpCommand::SetCalPage.code(), mode];
    byte_order.put_u16(&mut f, segment);
    f
}

/// Build the GET_CAL_PAGE command frame: `[0xEA, MODE, SEG₂, SEG₁]`.
#[must_use]
pub fn get_cal_page(byte_order: ByteOrder, mode: u8, segment: u16) -> Vec<u8> {
    let mut f = alloc::vec![crate::XcpCommand::GetCalPage.code(), mode];
    byte_order.put_u16(&mut f, segment);
    f
}
