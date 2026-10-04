//! Typed, exhaustive error taxonomy for every operation in the crate.
//!
//! The enum covers two failure classes and is deliberately **exhaustive**
//! (no `#[non_exhaustive]`): callers are expected to match every variant,
//! and a new variant is a semver-minor event by design — the same policy as
//! `can-core`'s `CanError`.
//!
//! 1. **Transport-local structural errors** — the byte stream is not a
//!    well-formed XCP CTO response at all ([`XcpError::BufferTooShort`],
//!    [`XcpError::UnknownPid`]). These originate in this crate.
//! 2. **Slave error responses** — the slave answered `ERR` (PID `0xFE`) and
//!    the second byte is one of the ASAM XCP error codes
//!    ([`XcpError::CmdUnknown`] `0x20`, [`XcpError::Generic`] `0x31`, …).
//!    Unrecognised slave codes land in [`XcpError::SlaveOther`] so parsing
//!    stays total.
//!
//! Note [`XcpError::CmdSynch`] (`0x00`): as the response to the SYNCH
//! command it is the **success** indicator — synchronisation achieved — not
//! a failure. Callers driving a SYNCH handshake should treat it specially.

use core::fmt;

/// The single crate-level error for command framing, response parsing, and
/// DAQ list configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XcpError {
    // ---- transport-local structural errors ----------------------------
    /// The buffer is shorter than the XCP packet format requires
    /// (`need` bytes for the identified layout, `have` available).
    BufferTooShort {
        /// Number of bytes the packet layout requires.
        need: usize,
        /// Number of bytes actually available.
        have: usize,
    },
    /// The first byte is not a CTO response PID (`0xFF` RES, `0xFE` ERR,
    /// `0xFD` EV, `0xFC` SERV).
    UnknownPid(u8),

    // ---- slave ERR codes (ASAM XCP part 1, error-code table) ----------
    /// `ERR_CMD_SYNCH` (`0x00`) — command processor synchronisation.
    ///
    /// This is the **positive** completion indicator of the SYNCH command;
    /// after any ERR response the master must send SYNCH and synchronise
    /// on this code.
    CmdSynch,
    /// `ERR_CMD_BUSY` (`0x10`) — command was not executed; the command
    /// processor is busy.
    CmdBusy,
    /// `ERR_DAQ_ACTIVE` (`0x11`) — command rejected because DAQ is running.
    DaqActive,
    /// `ERR_PGM_ACTIVE` (`0x12`) — command rejected because the flash
    /// program sequencer is running.
    PgmActive,
    /// `ERR_CMD_UNKNOWN` (`0x20`) — unknown command or not-implemented
    /// optional command.
    CmdUnknown,
    /// `ERR_CMD_SYNTAX` (`0x21`) — command syntax invalid (structure,
    /// length).
    CmdSyntax,
    /// `ERR_OUT_OF_RANGE` (`0x22`) — syntax valid but parameter(s) out of
    /// range (page/segment/address range, element size, …).
    OutOfRange,
    /// `ERR_WRITE_PROTECTED` (`0x23`) — the memory location is write
    /// protected.
    WriteProtected,
    /// `ERR_ACCESS_DENIED` (`0x24`) — the memory location is not
    /// accessible.
    AccessDenied,
    /// `ERR_ACCESS_LOCKED` (`0x25`) — access denied; seed and key are
    /// required ([`XcpCommand::GetSeed`](crate::XcpCommand::GetSeed)).
    AccessLocked,
    /// `ERR_PAGE_NOT_VALID` (`0x26`) — the selected page is not available.
    PageNotValid,
    /// `ERR_MODE_NOT_VALID` (`0x27`) — the selected page mode is not
    /// available.
    ModeNotValid,
    /// `ERR_SEGMENT_NOT_VALID` (`0x28`) — the selected segment is not
    /// valid.
    SegmentNotValid,
    /// `ERR_SEQUENCE` (`0x29`) — sequence error (e.g. `DOWNLOAD_NEXT`
    /// without a preceding `DOWNLOAD`).
    Sequence,
    /// `ERR_DAQ_CONFIG` (`0x2A`) — the DAQ configuration is not valid.
    DaqConfig,
    /// `ERR_MEMORY_OVERFLOW` (`0x30`) — memory overflow error.
    MemoryOverflow,
    /// `ERR_GENERIC` (`0x31`) — generic error.
    Generic,
    /// `ERR_VERIFY` (`0x32`) — the slave's internal program verify routine
    /// detected an error.
    Verify,
    /// `ERR_RESOURCE_TEMPORARY_NOT_ACCESSIBLE` (`0x33`, XCP ≥ 1.1) — the
    /// requested resource is temporarily not accessible (e.g. the slave is
    /// busy with a non-XCP task).
    ResourceTemporaryNotAccessible,
    /// An ERR packet carrying a code outside the ASAM table (vendor- or
    /// future-reserved). Parsing stays total; the raw code is preserved.
    SlaveOther(u8),
}

impl XcpError {
    /// Map a slave ERR code byte to the typed error. Total: any byte
    /// outside the ASAM table yields [`XcpError::SlaveOther`].
    #[must_use]
    pub fn from_slave_code(code: u8) -> Self {
        match code {
            0x00 => Self::CmdSynch,
            0x10 => Self::CmdBusy,
            0x11 => Self::DaqActive,
            0x12 => Self::PgmActive,
            0x20 => Self::CmdUnknown,
            0x21 => Self::CmdSyntax,
            0x22 => Self::OutOfRange,
            0x23 => Self::WriteProtected,
            0x24 => Self::AccessDenied,
            0x25 => Self::AccessLocked,
            0x26 => Self::PageNotValid,
            0x27 => Self::ModeNotValid,
            0x28 => Self::SegmentNotValid,
            0x29 => Self::Sequence,
            0x2A => Self::DaqConfig,
            0x30 => Self::MemoryOverflow,
            0x31 => Self::Generic,
            0x32 => Self::Verify,
            0x33 => Self::ResourceTemporaryNotAccessible,
            other => Self::SlaveOther(other),
        }
    }

    /// The slave ERR code this error was raised from, or `None` for
    /// transport-local structural errors.
    #[must_use]
    pub fn slave_code(&self) -> Option<u8> {
        Some(match self {
            Self::CmdSynch => 0x00,
            Self::CmdBusy => 0x10,
            Self::DaqActive => 0x11,
            Self::PgmActive => 0x12,
            Self::CmdUnknown => 0x20,
            Self::CmdSyntax => 0x21,
            Self::OutOfRange => 0x22,
            Self::WriteProtected => 0x23,
            Self::AccessDenied => 0x24,
            Self::AccessLocked => 0x25,
            Self::PageNotValid => 0x26,
            Self::ModeNotValid => 0x27,
            Self::SegmentNotValid => 0x28,
            Self::Sequence => 0x29,
            Self::DaqConfig => 0x2A,
            Self::MemoryOverflow => 0x30,
            Self::Generic => 0x31,
            Self::Verify => 0x32,
            Self::ResourceTemporaryNotAccessible => 0x33,
            Self::SlaveOther(code) => *code,
            Self::BufferTooShort { .. } | Self::UnknownPid(_) => return None,
        })
    }

    /// `true` when the error is a slave ERR response rather than a
    /// transport-local structural failure.
    #[must_use]
    pub fn is_slave_error(&self) -> bool {
        self.slave_code().is_some()
    }
}

impl fmt::Display for XcpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooShort { need, have } => {
                write!(f, "buffer too short: need {need} bytes, have {have}")
            }
            Self::UnknownPid(pid) => write!(f, "not an XCP response PID: 0x{pid:02X}"),
            Self::CmdSynch => {
                f.write_str("ERR 0x00: command processor synchronisation (SYNCH completed)")
            }
            Self::CmdBusy => f.write_str("ERR 0x10: command processor busy, command not executed"),
            Self::DaqActive => f.write_str("ERR 0x11: rejected because DAQ is running"),
            Self::PgmActive => {
                f.write_str("ERR 0x12: rejected because flash programming is running")
            }
            Self::CmdUnknown => f.write_str("ERR 0x20: unknown or not-implemented command"),
            Self::CmdSyntax => f.write_str("ERR 0x21: command syntax invalid"),
            Self::OutOfRange => f.write_str("ERR 0x22: parameter out of range"),
            Self::WriteProtected => f.write_str("ERR 0x23: memory location is write protected"),
            Self::AccessDenied => f.write_str("ERR 0x24: memory location not accessible"),
            Self::AccessLocked => f.write_str("ERR 0x25: access locked, seed & key required"),
            Self::PageNotValid => f.write_str("ERR 0x26: selected page not available"),
            Self::ModeNotValid => f.write_str("ERR 0x27: selected page mode not available"),
            Self::SegmentNotValid => f.write_str("ERR 0x28: selected segment not valid"),
            Self::Sequence => f.write_str("ERR 0x29: sequence error"),
            Self::DaqConfig => f.write_str("ERR 0x2A: DAQ configuration not valid"),
            Self::MemoryOverflow => f.write_str("ERR 0x30: memory overflow"),
            Self::Generic => f.write_str("ERR 0x31: generic error"),
            Self::Verify => f.write_str("ERR 0x32: slave program verify failed"),
            Self::ResourceTemporaryNotAccessible => {
                f.write_str("ERR 0x33: resource temporarily not accessible")
            }
            Self::SlaveOther(code) => write!(f, "ERR 0x{code:02X}: slave-specific error code"),
        }
    }
}

impl core::error::Error for XcpError {}

/// A slave answered `ERR` (PID `0xFE`); the second byte carries the code.
///
/// [`crate::parse_response`] surfaces this as [`XcpError`]; the payload
/// bytes after the code (reserved on current slaves) are dropped.
pub(crate) fn slave_error(packet: &[u8]) -> Result<XcpError, XcpError> {
    let Some((&code, _)) = packet.split_first() else {
        return Err(XcpError::BufferTooShort {
            need: 2,
            have: packet.len() + 1,
        });
    };
    Ok(XcpError::from_slave_code(code))
}
