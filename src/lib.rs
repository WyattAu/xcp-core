//! XCP on CAN — Universal Measurement and Calibration Protocol: commands,
//! responses, DAQ lists.
//!
//! `xcp-core` is the estate's **L1 substrate** for XCP calibration and
//! measurement work: the complete CTO command table (ASAM XCP part 1,
//! protocol layer, versions 1.0–1.5), a total response parser with the
//! full slave error taxonomy, DAQ list configuration framing, and the CAN
//! transport bridge ([`transport`], XCP part 3) over [`can-core`].
//!
//! # Total parsing
//!
//! [`parse_response`] never panics: every buffer access is bounds-checked,
//! and any input — hostile replay captures included — resolves to a typed
//! [`XcpResponse`] or a typed [`XcpError`]. Slave `ERR` packets map onto
//! the exhaustive [`XcpError`] taxonomy (`0x20` command unknown, `0x31`
//! generic, `0x33` resource temporarily not accessible, …), so session
//! logic can `match` on protocol semantics, not raw bytes.
//!
//! # Byte order
//!
//! XCP slaves declare their byte order — Intel (little-endian) or Motorola
//! (big-endian) — in the CONNECT response. [`ByteOrder`] parameterizes
//! every multi-byte field the crate sends or decodes
//! ([`frame::set_mta`], [`frame::download_with`], [`daq::DaqList::configure_frames`],
//! [`parse_response_with`]), with Intel as the crate default.
//!
//! # Example
//!
//! A full CONNECT exchange, wire bytes to typed session parameters:
//!
//! ```
//! use xcp_core::{connect, parse_response, ByteOrder, Expect, ResourceMode, XcpCommand};
//!
//! // Master → slave: CONNECT, normal mode.
//! assert_eq!(connect(ResourceMode::CONNECT_NORMAL), [0xFF, 0x00]);
//!
//! // Slave → master (a real-world response):
//! let reply = [0xFF, 0x1C, 0xC0, 0x08, 0x08, 0x00, 0x01, 0x01];
//! let res = xcp_core::parse_response_with(ByteOrder::Intel, Expect::Connect, &reply)?;
//!
//! assert_eq!(res,
//!     xcp_core::XcpResponse::Connect(xcp_core::ConnectInfo {
//!         resource: ResourceMode::DAQ | ResourceMode::STIM | ResourceMode::PGM,
//!         comm_mode: xcp_core::CommModeBasic {
//!             byte_order: ByteOrder::Intel,
//!             address_granularity: xcp_core::AddressGranularity::Byte,
//!             slave_block_mode: true,
//!             optional_info: true,
//!         },
//!         max_cto: 8,
//!         max_dto: 8,
//!         protocol_version: (1, 0),
//!         transport_version: (1, 0),
//!     }));
//! assert!(matches!(XcpCommand::from_code(0xF6), Some(XcpCommand::SetMta)));
//! # Ok::<(), xcp_core::XcpError>(())
//! ```
//!
//! Calibration data goes through the MTA — [`frame::download`] builds the
//! whole SET_MTA + DOWNLOAD (block-mode chunked) sequence, and DAQ
//! measurement is configured with [`daq::DaqList`]:
//!
//! ```
//! use xcp_core::{DaqEntry, DaqList};
//!
//! let mut list = DaqList::new(0, 0x0010);
//! list.push_entry(DaqEntry { bit_offset: 0, size: 2, address: 0x3200_0100, address_extension: 0 });
//! let frames = list.configure_frames(xcp_core::ByteOrder::Intel);
//! // SET_DAQ_LIST_MODE, SET_DAQ_PTR, WRITE_DAQ — 8 bytes each:
//! assert_eq!(frames.len(), 24);
//! assert_eq!(list.start_frame(), [0xDE, 0x01, 0x00, 0x00]);
//! ```
//!
//! # `no_std`
//!
//! The crate is `no_std` + `alloc` and depends only on `can-core` (the L0
//! frame layer) and `thiserror`.

#![no_std]
extern crate alloc;

mod byteorder;
mod command;
mod daq;
mod error;
mod frame;
mod resource;
mod response;
pub mod transport;

pub use byteorder::{AddressGranularity, ByteOrder};
pub use command::XcpCommand;
pub use daq::{DaqEntry, DaqList, DaqListMode, Direction, StartStopMode};
pub use error::XcpError;
pub use frame::{
    connect, disconnect, download, download_frames, download_with, get_cal_page,
    get_comm_mode_info, get_id, get_status, get_version, set_cal_page, set_mta, short_download,
    short_upload, synch, upload, upload_frame, upload_with, DOWNLOAD_MAX_CHUNK,
    DOWNLOAD_NEXT_MAX_CHUNK,
};
pub use resource::{ResourceMode, SessionStatus};
pub use response::{
    parse_response, parse_response_with, CommModeBasic, ConnectInfo, Expect, IdInfo, Pid,
    StatusInfo, VersionInfo, XcpResponse,
};
