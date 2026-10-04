//! Resource and session-status bit sets.
//!
//! [`ResourceMode`] models the resource bitmask carried by the CONNECT
//! response (`RESOURCE` byte), the GET_STATUS protection status, and the
//! CONNECT mode byte. [`SessionStatus`] models the GET_STATUS session
//! status byte.

use core::fmt;
use core::ops::{BitAnd, BitOr, Not};

/// Bit set of XCP resources: which feature groups a slave offers (CONNECT
/// response / GET_STATUS protection status) and which access is requested
/// in the CONNECT mode byte.
///
/// Bit assignment per XCP part 1 (`RESOURCE` byte):
/// bit 0 CAL/PAG, bit 2 DAQ, bit 3 STIM, bit 4 PGM, bit 5 DBG (XCP ≥ 1.5);
/// bits 1, 6, 7 are reserved.
///
/// The CONNECT command's mode byte is `0x00` (normal) or `0x01`
/// (user-defined) per the spec; `connect(mode)` passes the bit set through
/// so user-defined modes can be modeled. Use
/// [`ResourceMode::CONNECT_NORMAL`] / [`ResourceMode::CONNECT_USER_DEFINED`]
/// for the standard modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ResourceMode(u8);

impl ResourceMode {
    /// Calibration and page switching (`CAL/PAG`) — bit 0.
    pub const CAL_PAGE: Self = Self(0x01);
    /// Data acquisition (`DAQ`) — bit 2.
    pub const DAQ: Self = Self(0x04);
    /// Data stimulation (`STIM`) — bit 3.
    pub const STIM: Self = Self(0x08);
    /// Flash programming (`PGM`) — bit 4.
    pub const PGM: Self = Self(0x10);
    /// Debugging (`DBG`, XCP ≥ 1.5) — bit 5.
    pub const DBG: Self = Self(0x20);
    /// Composite: calibration *or* DAQ access (`CAL/PAG | DAQ`) — the
    /// common "measure or calibrate" session.
    pub const CAL_PAGE_OR_DAQ: Self = Self(0x05);
    /// CONNECT mode `0x00` — normal mode.
    pub const CONNECT_NORMAL: Self = Self(0x00);
    /// CONNECT mode `0x01` — user-defined mode.
    pub const CONNECT_USER_DEFINED: Self = Self(0x01);

    /// Build from a raw resource byte. Total: unknown/reserved bits are
    /// preserved (slaves may set vendor bits).
    #[must_use]
    pub const fn from_raw(bits: u8) -> Self {
        Self(bits)
    }

    /// The raw resource byte.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// `true` when no bit is set (e.g. the slave reports no protected
    /// resources).
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// `true` when every bit in `other` is also set in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// `true` when the CAL/PAG resource is present.
    #[must_use]
    pub const fn cal_page(self) -> bool {
        self.contains(Self::CAL_PAGE)
    }

    /// `true` when the DAQ resource is present.
    #[must_use]
    pub const fn daq(self) -> bool {
        self.contains(Self::DAQ)
    }

    /// `true` when the STIM resource is present.
    #[must_use]
    pub const fn stim(self) -> bool {
        self.contains(Self::STIM)
    }

    /// `true` when the PGM resource is present.
    #[must_use]
    pub const fn pgm(self) -> bool {
        self.contains(Self::PGM)
    }
}

impl BitOr for ResourceMode {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for ResourceMode {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Not for ResourceMode {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

impl core::ops::BitOr for SessionStatus {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl fmt::Display for ResourceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("none");
        }
        let mut first = true;
        let flags = [
            (Self::CAL_PAGE.0, "cal_pag"),
            (Self::DAQ.0, "daq"),
            (Self::STIM.0, "stim"),
            (Self::PGM.0, "pgm"),
            (Self::DBG.0, "dbg"),
        ];
        for (bit, name) in flags {
            if self.0 & bit == bit {
                if !first {
                    f.write_str("|")?;
                }
                f.write_str(name)?;
                first = false;
            }
        }
        let unknown = self.0 & !0x3F;
        if unknown != 0 {
            if !first {
                f.write_str("|")?;
            }
            write!(f, "0x{unknown:02X}")?;
        }
        Ok(())
    }
}

/// Bit set carried by the GET_STATUS response's `SESSION_STATUS` byte.
///
/// The byte is bit-reversed relative to the resource byte: bit 0
/// `STORE_CAL_REQUEST`, bit 1 `CAL_PAGE_CFG_LOST`, bit 2
/// `STORE_DAQ_REQUEST`, bit 3 `CLEAR_DAQ_REQUEST`, bit 4 `DAQ_CFG_LOST`,
/// bit 5 reserved, bit 6 `DAQ_ACTIVE`, bit 7 `RESUME`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SessionStatus(u8);

impl SessionStatus {
    /// Non-volatile calibration data has to be stored — bit 0.
    pub const STORE_CAL_REQUEST: Self = Self(0x01);
    /// Calibration page configuration lost — bit 1.
    pub const CAL_PAGE_CFG_LOST: Self = Self(0x02);
    /// Non-volatile DAQ configuration has to be stored — bit 2.
    pub const STORE_DAQ_REQUEST: Self = Self(0x04);
    /// A request to clear the DAQ configuration is pending — bit 3.
    pub const CLEAR_DAQ_REQUEST: Self = Self(0x08);
    /// DAQ configuration lost — bit 4.
    pub const DAQ_CFG_LOST: Self = Self(0x10);
    /// DAQ list(s) currently running — bit 6.
    pub const DAQ_ACTIVE: Self = Self(0x40);
    /// Session is in resume mode — bit 7.
    pub const RESUME: Self = Self(0x80);

    /// Build from the raw `SESSION_STATUS` byte. Total: reserved bits are
    /// preserved.
    #[must_use]
    pub const fn from_raw(bits: u8) -> Self {
        Self(bits)
    }

    /// The raw `SESSION_STATUS` byte.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// `true` when no flag is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// `true` when every flag in `other` is also set in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// `true` when DAQ list(s) are currently running.
    #[must_use]
    pub const fn daq_active(self) -> bool {
        self.contains(Self::DAQ_ACTIVE)
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02X}", self.0)
    }
}
