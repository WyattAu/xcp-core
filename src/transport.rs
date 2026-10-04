//! XCP on CAN transport framing (ASAM XCP part 3).
//!
//! The CAN transport layer carries one XCP packet per CAN frame: the CTO/
//! DTO bytes become the frame payload, and the slave's command and
//! response CAN identifiers are fixed configuration (discoverable at
//! runtime with the `GET_SLAVE_ID` / `GET_DAQ_ID` transport commands).
//! Classic CAN caps a packet at 8 bytes — the `MAX_CTO = 8` most slaves
//! report — and the DLC equals the packet length.
//!
//! [`XcpOnCan`] bridges this crate's packets to `can-core` frames: command
//! packets are framed for transmission to the slave, and received frames
//! are validated and stripped back to raw packets.

use alloc::vec::Vec;
use can_core::{CanError, CanFrame, CanId};

/// The classic-CAN XCP packet limit: one CTO/DTO per 8-byte frame.
pub const CLASSIC_CAN_MAX_CTO: usize = 8;

/// Master-side CAN identifiers for an XCP session.
///
/// From the master's seat: `command` frames go **to** the slave (the
/// slave's "receive" ID, `CTO`/`DTO` rx in part 3 terms), `response`
/// frames arrive **from** the slave. DAQ/STIM DTOs typically use the same
/// response ID; `daq_id` overrides when GET_DAQ_ID reports a distinct one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XcpOnCan {
    /// CAN ID the slave listens on for commands (master → slave).
    pub command: CanId,
    /// CAN ID the slave transmits responses and DAQ DTOs on
    /// (slave → master).
    pub response: CanId,
    /// Optional distinct CAN ID for DAQ/STIM DTOs (`None` = same as
    /// `response`).
    pub daq: Option<CanId>,
}

impl XcpOnCan {
    /// A session with one command ID and one shared response/DAQ ID.
    #[must_use]
    pub const fn new(command: CanId, response: CanId) -> Self {
        Self {
            command,
            response,
            daq: None,
        }
    }

    /// Frame a command packet for transmission to the slave.
    ///
    /// # Errors
    ///
    /// [`CanError::DataTooLong`] when the packet exceeds the classic CAN
    /// payload limit (8 bytes — a CTO must fit one frame; block-mode
    /// transfers chunk at the protocol layer, see
    /// [`frame`](crate::frame::download_frames)).
    pub fn command_frame<'a>(&self, packet: &'a [u8]) -> Result<CanFrame<'a>, CanError> {
        CanFrame::new(self.command, packet)
    }

    /// Frame a DAQ/STIM DTO for transmission (STIM direction).
    ///
    /// # Errors
    ///
    /// See [`XcpOnCan::command_frame`].
    pub fn daq_frame<'a>(&self, packet: &'a [u8]) -> Result<CanFrame<'a>, CanError> {
        CanFrame::new(self.daq.unwrap_or(self.response), packet)
    }

    /// Validate that `frame` belongs to this session (its ID matches the
    /// response or DAQ ID) and return the XCP packet payload.
    #[must_use]
    pub fn extract_packet<'a>(&self, frame: &'a CanFrame<'a>) -> Option<&'a [u8]> {
        let id = frame.id();
        if id == self.response || self.daq.is_some_and(|d| d == id) {
            Some(frame.data())
        } else {
            None
        }
    }

    /// Convenience: frame a command packet and return the raw SocketCAN
    /// bytes ([`CanFrame::to_bytes`]).
    ///
    /// # Errors
    ///
    /// See [`XcpOnCan::command_frame`].
    pub fn command_bytes(&self, packet: &[u8]) -> Result<Vec<u8>, CanError> {
        Ok(self.command_frame(packet)?.to_bytes())
    }
}
