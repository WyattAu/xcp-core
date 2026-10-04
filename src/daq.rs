//! DAQ list configuration for periodic measurement.
//!
//! A DAQ (Data ACquisition) list groups ODTs (Object Descriptor Tables),
//! each holding ODT entries — (bit offset, size, address) slices of ECU
//! memory sampled on an event channel. This module frames the static
//! configuration sequence ([`DaqList::configure_frames`]: SET_DAQ_LIST_MODE,
//! then per-entry SET_DAQ_PTR/WRITE_DAQ) and the run-control frames
//! (START_STOP_DAQ_LIST / START_STOP_SYNCH).
//!
//! # Example
//!
//! ```
//! use xcp_core::{ByteOrder, DaqEntry, DaqList};
//!
//! let mut list = DaqList::new(1, 0x0010);
//! list.timestamp = true;
//! list.push_entry(DaqEntry { bit_offset: 0, size: 4, address: 0x3200_1000, address_extension: 0 });
//!
//! // SET_DAQ_LIST_MODE + (SET_DAQ_PTR + WRITE_DAQ) × entries:
//! let frames = list.configure_frames(ByteOrder::Intel);
//! assert_eq!(frames[0], 0xE0);          // SET_DAQ_LIST_MODE
//! assert_eq!(frames[8], 0xE2);          // SET_DAQ_PTR
//! assert_eq!(frames[16], 0xE1);         // WRITE_DAQ
//!
//! // Start the list (mode 0x01), then all selected lists:
//! assert_eq!(list.start_frame(), [0xDE, 0x01, 0x01, 0x00]);
//! assert_eq!(DaqList::start_synch_frame(), [0xDD, 0x00]);
//! ```

use crate::byteorder::ByteOrder;
use alloc::vec::Vec;

/// One ODT entry: a slice of ECU memory sampled into a DAQ packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DaqEntry {
    /// Bit offset within the addressed element (0–7 for byte granularity).
    pub bit_offset: u8,
    /// Element size in address-granularity units (bytes at byte
    /// granularity).
    pub size: u8,
    /// ECU address of the element.
    pub address: u32,
    /// Address extension (memory selector; `0` on most ECUs).
    pub address_extension: u8,
}

/// DAQ list direction for [`DaqListMode::STIM`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(clippy::exhaustive_enums)]
pub enum Direction {
    /// Data acquisition (slave → master).
    #[default]
    Daq,
    /// Data stimulation (master → slave).
    Stim,
}

/// Bits of the SET_DAQ_LIST_MODE `MODE` byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DaqListMode {
    /// Direction: DAQ (default) or STIM.
    pub direction: Direction,
    /// Attach a timestamp to the list's DTOs.
    pub timestamp: bool,
    /// Send DTOs without the identification field (PID off).
    pub pid_off: bool,
}

impl DaqListMode {
    /// Encode to the SET_DAQ_LIST_MODE `MODE` byte.
    #[must_use]
    pub const fn to_bits(self) -> u8 {
        let stim = match self.direction {
            Direction::Daq => 0x00,
            Direction::Stim => 0x02,
        };
        stim | if self.timestamp { 0x10 } else { 0 } | if self.pid_off { 0x20 } else { 0 }
    }
}

/// START_STOP_DAQ_LIST modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::exhaustive_enums)]
pub enum StartStopMode {
    /// `0x00` — stop the DAQ list.
    Stop = 0x00,
    /// `0x01` — start the DAQ list.
    Start = 0x01,
    /// `0x02` — select the DAQ list for a later START_STOP_SYNCH.
    Select = 0x02,
}

impl StartStopMode {
    /// The wire mode byte.
    #[must_use]
    pub const fn to_bits(self) -> u8 {
        match self {
            Self::Stop => 0x00,
            Self::Start => 0x01,
            Self::Select => 0x02,
        }
    }
}

/// A DAQ list configuration for periodic measurement: which event channel
/// triggers it, at what prescaler/priority, and which ODT entries it
/// samples.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DaqList {
    /// DAQ list number (`0x0000`–`MAX_DAQ − 1`).
    pub number: u16,
    /// Event channel the list is triggered by.
    pub event_channel: u16,
    /// Event-channel prescaler: sample every `prescaler`-th event
    /// (`1` = every event). Applied at start time.
    pub prescaler: u8,
    /// DAQ list priority (`0` = standard).
    pub priority: u8,
    /// Attach timestamps to this list's DTOs.
    pub timestamp: bool,
    /// Send DTOs without the identification field.
    pub pid_off: bool,
    /// DAQ (measure) or STIM (stimulate).
    pub direction: Direction,
    /// The list's ODT entries, in sampling order.
    pub entries: Vec<DaqEntry>,
}

impl DaqList {
    /// A fresh DAQ list on `event_channel`, prescaler 1, priority 0, no
    /// entries.
    #[must_use]
    pub fn new(number: u16, event_channel: u16) -> Self {
        Self {
            number,
            event_channel,
            prescaler: 1,
            priority: 0,
            timestamp: false,
            pid_off: false,
            direction: Direction::Daq,
            entries: Vec::new(),
        }
    }

    /// Append an ODT entry.
    pub fn push_entry(&mut self, entry: DaqEntry) {
        self.entries.push(entry);
    }

    /// The SET_DAQ_LIST_MODE `MODE` byte for this list.
    #[must_use]
    pub fn mode_bits(&self) -> u8 {
        DaqListMode {
            direction: self.direction,
            timestamp: self.timestamp,
            pid_off: self.pid_off,
        }
        .to_bits()
    }

    /// Build the full static-configuration frame sequence:
    /// SET_DAQ_LIST_MODE, then SET_DAQ_PTR + WRITE_DAQ for every entry.
    ///
    /// All multi-byte fields use `byte_order` — the order the slave
    /// advertised in CONNECT. For a *dynamic* list, precede this sequence
    /// with FREE_DAQ/ALLOC_DAQ/ALLOC_ODT/ALLOC_ODT_ENTRY (not framed here).
    #[must_use]
    pub fn configure_frames(&self, byte_order: ByteOrder) -> Vec<u8> {
        // SET_DAQ_LIST_MODE: E0, MODE, DAQ(word), EVENT(word), PRESCALER,
        // PRIORITY.
        let mut out = Vec::with_capacity(8 + self.entries.len() * 16);
        out.push(crate::XcpCommand::SetDaqListMode.code());
        out.push(self.mode_bits());
        byte_order.put_u16(&mut out, self.number);
        byte_order.put_u16(&mut out, self.event_channel);
        out.push(self.prescaler);
        out.push(self.priority);

        // SET_DAQ_PTR: E2, 0, 0, 0, DAQ(word), ODT, ODT_ENTRY.
        // WRITE_DAQ:   E1, BIT_OFFSET, SIZE, ADDR_EXT, ADDR(dword).
        for (i, entry) in self.entries.iter().enumerate() {
            // One ODT per entry (the default optimisation type); entry 0
            // within each ODT. Lists beyond 256 entries saturate — a
            // slave's MAX_ODT limit applies long before.
            let odt = u8::try_from(i).unwrap_or(u8::MAX);
            out.push(crate::XcpCommand::SetDaqPtr.code());
            out.extend_from_slice(&[0x00, 0x00, 0x00]);
            byte_order.put_u16(&mut out, self.number);
            out.push(odt);
            out.push(0x00); // entry 0 within the ODT

            out.push(crate::XcpCommand::WriteDaq.code());
            out.push(entry.bit_offset);
            out.push(entry.size);
            out.push(entry.address_extension);
            byte_order.put_u32(&mut out, entry.address);
        }
        out
    }

    /// Build a START_STOP_DAQ_LIST frame for this list in `mode`:
    /// `[0xDE, MODE, DAQ₂, DAQ₁]`.
    #[must_use]
    pub fn start_stop_frame(&self, mode: StartStopMode) -> Vec<u8> {
        self.start_stop_frame_with(ByteOrder::Intel, mode)
    }

    /// [`DaqList::start_stop_frame`] with explicit byte order.
    #[must_use]
    pub fn start_stop_frame_with(&self, byte_order: ByteOrder, mode: StartStopMode) -> Vec<u8> {
        let mut f = alloc::vec![crate::XcpCommand::StartStopDaqList.code(), mode.to_bits()];
        byte_order.put_u16(&mut f, self.number);
        f
    }

    /// `[0xDE, 0x01, DAQ₂, DAQ₁]` — start this list (Intel byte order).
    #[must_use]
    pub fn start_frame(&self) -> Vec<u8> {
        self.start_stop_frame(StartStopMode::Start)
    }

    /// `[0xDE, 0x00, DAQ₂, DAQ₁]` — stop this list (Intel byte order).
    #[must_use]
    pub fn stop_frame(&self) -> Vec<u8> {
        self.start_stop_frame(StartStopMode::Stop)
    }

    /// `[0xDD, 0x00]` — START_STOP_SYNCH: start all selected DAQ lists.
    #[must_use]
    pub fn start_synch_frame() -> Vec<u8> {
        alloc::vec![crate::XcpCommand::StartStopSynch.code(), 0x00]
    }

    /// `[0xDD, 0x01]` — START_STOP_SYNCH: stop all selected DAQ lists.
    #[must_use]
    pub fn stop_synch_frame() -> Vec<u8> {
        alloc::vec![crate::XcpCommand::StartStopSynch.code(), 0x01]
    }
}
