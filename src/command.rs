//! The complete CTO command-code table (ASAM XCP part 1, protocol layer).
//!
//! Command PIDs live in `0xC0..=0xFF`, assigned downward from `0xFF`
//! (CONNECT) by feature group: standard, calibration, page switching, DAQ,
//! flash programming, then the XCP 1.1–1.5 extensions at the bottom of the
//! range.

/// An XCP CTO command code — every command defined by the ASAM XCP
/// protocol layer (part 1) through version 1.5.
///
/// The codes are the wire PIDs; [XcpCommand::from_code] is total over the
/// defined range and returns `None` for reserved codes. `0x00..=0xBF` are
/// STIM PIDs (DTO direction, master → slave) and are not commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::exhaustive_enums)]
pub enum XcpCommand {
    // ---- standard commands --------------------------------------------
    /// `0xFF` — establish the XCP connection. First command of every
    /// session; returns resource, basic comm mode, `MAX_CTO`/`MAX_DTO`,
    /// protocol and transport version.
    Connect,
    /// `0xFE` — terminate the XCP connection.
    Disconnect,
    /// `0xFD` — read session status, resource protection status, session
    /// configuration ID.
    GetStatus,
    /// `0xFC` — synchronise the command stream after any error response.
    /// Completes with ERR `0x00` ([`XcpError::CmdSynch`] semantics).
    Synch,
    /// `0xFB` — read optional communication-mode information (block mode,
    /// interleaved mode, queue size, driver version).
    GetCommModeInfo,
    /// `0xFA` — read a text/filename/URL/EPK identification string.
    GetId,
    /// `0xF9` — set request mode (store/clear non-volatile DAQ/calibration
    /// data requests).
    SetRequestMode,
    /// `0xF8` — request the seed for unlock.
    GetSeed,
    /// `0xF7` — return the key computed from the seed to unlock a resource.
    Unlock,
    /// `0xF6` — set the memory transfer address (MTA) for subsequent
    /// UPLOAD/DOWNLOAD/DOWNLOAD_NEXT/BUILD_CHECKSUM.
    SetMta,
    /// `0xF5` — upload (read) from the MTA; advances the MTA.
    Upload,
    /// `0xF4` — upload from an explicit address without setting the MTA.
    ShortUpload,
    /// `0xF3` — build a checksum over a memory block.
    BuildChecksum,
    /// `0xF2` — transport-layer-specific command (XCP on CAN: GET_SLAVE_ID
    /// `0xFF`, GET_DAQ_ID `0xFE`, SET_DAQ_ID `0xFD`).
    TransportLayerCommand,
    /// `0xF1` — user-defined command.
    UserCommand,

    // ---- calibration commands -----------------------------------------
    /// `0xF0` — download (write) to the MTA; advances the MTA.
    Download,
    /// `0xEF` — continue a block-mode download.
    DownloadNext,
    /// `0xEE` — download a full block-mode frame of data.
    DownloadMax,
    /// `0xED` — download to an explicit address without setting the MTA.
    ShortDownload,
    /// `0xEC` — modify bits at the MTA (read-modify-write with mask).
    ModifyBits,

    // ---- page-switching commands --------------------------------------
    /// `0xEB` — switch the calibration page of a segment.
    SetCalPage,
    /// `0xEA` — read the active calibration page of a segment.
    GetCalPage,
    /// `0xE9` — read page-switching processor properties.
    GetPagProcessorInfo,
    /// `0xE8` — read segment information (mode 0/1/2).
    GetSegmentInfo,
    /// `0xE7` — read page information/properties.
    GetPageInfo,
    /// `0xE6` — set the segment mode.
    SetSegmentMode,
    /// `0xE5` — read the segment mode.
    GetSegmentMode,
    /// `0xE4` — copy one calibration page to another.
    CopyCalPage,

    // ---- DAQ commands (periodic data exchange) ------------------------
    /// `0xE3` — clear (empty) a DAQ list's ODT entries.
    ClearDaqList,
    /// `0xE2` — set the DAQ list write pointer (list, ODT, ODT entry).
    SetDaqPtr,
    /// `0xE1` — write the ODT entry at the DAQ pointer.
    WriteDaq,
    /// `0xC7` — write several consecutive ODT entries in one command.
    WriteDaqMultiple,
    /// `0xE0` — set DAQ list properties (direction, event channel,
    /// prescaler, priority, timestamp, PID-off).
    SetDaqListMode,
    /// `0xDF` — read DAQ list properties.
    GetDaqListMode,
    /// `0xDE` — start / stop / select a single DAQ list.
    StartStopDaqList,
    /// `0xDD` — start or stop all selected DAQ lists.
    StartStopSynch,
    /// `0xDC` — read the DAQ clock (free-running slave timestamp).
    GetDaqClock,
    /// `0xDB` — read the ODT entry at the DAQ pointer back from the slave.
    ReadDaq,
    /// `0xDA` — read DAQ processor properties (`MAX_DAQ`, `MAX_EVENT_CHANNEL`).
    GetDaqProcessorInfo,
    /// `0xD9` — read DAQ granularity/ODT entry size and timestamp
    /// resolution.
    GetDaqResolutionInfo,
    /// `0xD8` — read a DAQ list's properties and limits.
    GetDaqListInfo,
    /// `0xD7` — read an event channel's properties and cycle time.
    GetDaqEventInfo,
    /// `0xD6` — free all dynamically allocated DAQ lists.
    FreeDaq,
    /// `0xD5` — allocate DAQ lists (dynamic configuration).
    AllocDaq,
    /// `0xD4` — allocate ODTs in a DAQ list (dynamic configuration).
    AllocOdt,
    /// `0xD3` — allocate ODT entries in an ODT (dynamic configuration).
    AllocOdtEntry,

    // ---- flash-programming commands ------------------------------------
    /// `0xD2` — enter programming mode.
    ProgramStart,
    /// `0xD1` — clear flash memory sectors.
    ProgramClear,
    /// `0xD0` — program (flash write) at the MTA.
    Program,
    /// `0xCF` — leave programming mode, reset the slave.
    ProgramReset,
    /// `0xCE` — read programming-processor properties.
    GetPgmProcessorInfo,
    /// `0xCD` — read flash sector information.
    GetSectorInfo,
    /// `0xCC` — set the program write pointer (programming MTA).
    ProgramPrepare,
    /// `0xCB` — set compression/encryption format for programming data.
    ProgramFormat,
    /// `0xCA` — continue a block-mode program write.
    ProgramNext,
    /// `0xC9` — program a full block-mode frame.
    ProgramMax,
    /// `0xC8` — verify programmed data.
    ProgramVerify,

    // ---- XCP 1.1–1.5 extensions ---------------------------------------
    /// `0xC0` — read protocol-layer and transport-layer version numbers
    /// (XCP ≥ 1.1).
    GetVersion,
    /// `0xC1` — set the DAQ packed mode of an event channel (XCP ≥ 1.5).
    SetDaqPackedMode,
    /// `0xC2` — read the DAQ packed mode of an event channel (XCP ≥ 1.5).
    GetDaqPackedMode,
    /// `0xC5` — read DTO counter properties (XCP ≥ 1.5).
    DtoCtrProperties,
    /// `0xC6` — read time-correlation properties (XCP ≥ 1.5).
    TimeCorrelationProperties,
}

impl XcpCommand {
    /// The wire command code (CTO PID).
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Connect => 0xFF,
            Self::Disconnect => 0xFE,
            Self::GetStatus => 0xFD,
            Self::Synch => 0xFC,
            Self::GetCommModeInfo => 0xFB,
            Self::GetId => 0xFA,
            Self::SetRequestMode => 0xF9,
            Self::GetSeed => 0xF8,
            Self::Unlock => 0xF7,
            Self::SetMta => 0xF6,
            Self::Upload => 0xF5,
            Self::ShortUpload => 0xF4,
            Self::BuildChecksum => 0xF3,
            Self::TransportLayerCommand => 0xF2,
            Self::UserCommand => 0xF1,
            Self::Download => 0xF0,
            Self::DownloadNext => 0xEF,
            Self::DownloadMax => 0xEE,
            Self::ShortDownload => 0xED,
            Self::ModifyBits => 0xEC,
            Self::SetCalPage => 0xEB,
            Self::GetCalPage => 0xEA,
            Self::GetPagProcessorInfo => 0xE9,
            Self::GetSegmentInfo => 0xE8,
            Self::GetPageInfo => 0xE7,
            Self::SetSegmentMode => 0xE6,
            Self::GetSegmentMode => 0xE5,
            Self::CopyCalPage => 0xE4,
            Self::ClearDaqList => 0xE3,
            Self::SetDaqPtr => 0xE2,
            Self::WriteDaq => 0xE1,
            Self::WriteDaqMultiple => 0xC7,
            Self::SetDaqListMode => 0xE0,
            Self::GetDaqListMode => 0xDF,
            Self::StartStopDaqList => 0xDE,
            Self::StartStopSynch => 0xDD,
            Self::GetDaqClock => 0xDC,
            Self::ReadDaq => 0xDB,
            Self::GetDaqProcessorInfo => 0xDA,
            Self::GetDaqResolutionInfo => 0xD9,
            Self::GetDaqListInfo => 0xD8,
            Self::GetDaqEventInfo => 0xD7,
            Self::FreeDaq => 0xD6,
            Self::AllocDaq => 0xD5,
            Self::AllocOdt => 0xD4,
            Self::AllocOdtEntry => 0xD3,
            Self::ProgramStart => 0xD2,
            Self::ProgramClear => 0xD1,
            Self::Program => 0xD0,
            Self::ProgramReset => 0xCF,
            Self::GetPgmProcessorInfo => 0xCE,
            Self::GetSectorInfo => 0xCD,
            Self::ProgramPrepare => 0xCC,
            Self::ProgramFormat => 0xCB,
            Self::ProgramNext => 0xCA,
            Self::ProgramMax => 0xC9,
            Self::ProgramVerify => 0xC8,
            Self::GetVersion => 0xC0,
            Self::SetDaqPackedMode => 0xC1,
            Self::GetDaqPackedMode => 0xC2,
            Self::DtoCtrProperties => 0xC5,
            Self::TimeCorrelationProperties => 0xC6,
        }
    }

    /// Total inverse of [`XcpCommand::code`] over the defined range;
    /// `None` for reserved codes.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0xFF => Self::Connect,
            0xFE => Self::Disconnect,
            0xFD => Self::GetStatus,
            0xFC => Self::Synch,
            0xFB => Self::GetCommModeInfo,
            0xFA => Self::GetId,
            0xF9 => Self::SetRequestMode,
            0xF8 => Self::GetSeed,
            0xF7 => Self::Unlock,
            0xF6 => Self::SetMta,
            0xF5 => Self::Upload,
            0xF4 => Self::ShortUpload,
            0xF3 => Self::BuildChecksum,
            0xF2 => Self::TransportLayerCommand,
            0xF1 => Self::UserCommand,
            0xF0 => Self::Download,
            0xEF => Self::DownloadNext,
            0xEE => Self::DownloadMax,
            0xED => Self::ShortDownload,
            0xEC => Self::ModifyBits,
            0xEB => Self::SetCalPage,
            0xEA => Self::GetCalPage,
            0xE9 => Self::GetPagProcessorInfo,
            0xE8 => Self::GetSegmentInfo,
            0xE7 => Self::GetPageInfo,
            0xE6 => Self::SetSegmentMode,
            0xE5 => Self::GetSegmentMode,
            0xE4 => Self::CopyCalPage,
            0xE3 => Self::ClearDaqList,
            0xE2 => Self::SetDaqPtr,
            0xE1 => Self::WriteDaq,
            0xC7 => Self::WriteDaqMultiple,
            0xE0 => Self::SetDaqListMode,
            0xDF => Self::GetDaqListMode,
            0xDE => Self::StartStopDaqList,
            0xDD => Self::StartStopSynch,
            0xDC => Self::GetDaqClock,
            0xDB => Self::ReadDaq,
            0xDA => Self::GetDaqProcessorInfo,
            0xD9 => Self::GetDaqResolutionInfo,
            0xD8 => Self::GetDaqListInfo,
            0xD7 => Self::GetDaqEventInfo,
            0xD6 => Self::FreeDaq,
            0xD5 => Self::AllocDaq,
            0xD4 => Self::AllocOdt,
            0xD3 => Self::AllocOdtEntry,
            0xD2 => Self::ProgramStart,
            0xD1 => Self::ProgramClear,
            0xD0 => Self::Program,
            0xCF => Self::ProgramReset,
            0xCE => Self::GetPgmProcessorInfo,
            0xCD => Self::GetSectorInfo,
            0xCC => Self::ProgramPrepare,
            0xCB => Self::ProgramFormat,
            0xCA => Self::ProgramNext,
            0xC9 => Self::ProgramMax,
            0xC8 => Self::ProgramVerify,
            0xC0 => Self::GetVersion,
            0xC1 => Self::SetDaqPackedMode,
            0xC2 => Self::GetDaqPackedMode,
            0xC5 => Self::DtoCtrProperties,
            0xC6 => Self::TimeCorrelationProperties,
            _ => return None,
        })
    }

    /// The ASAM-assigned command name, e.g. `"CONNECT"`, `"SET_MTA"`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Connect => "CONNECT",
            Self::Disconnect => "DISCONNECT",
            Self::GetStatus => "GET_STATUS",
            Self::Synch => "SYNCH",
            Self::GetCommModeInfo => "GET_COMM_MODE_INFO",
            Self::GetId => "GET_ID",
            Self::SetRequestMode => "SET_REQUEST_MODE",
            Self::GetSeed => "GET_SEED",
            Self::Unlock => "UNLOCK",
            Self::SetMta => "SET_MTA",
            Self::Upload => "UPLOAD",
            Self::ShortUpload => "SHORT_UPLOAD",
            Self::BuildChecksum => "BUILD_CHECKSUM",
            Self::TransportLayerCommand => "TRANSPORT_LAYER_CMD",
            Self::UserCommand => "USER_CMD",
            Self::Download => "DOWNLOAD",
            Self::DownloadNext => "DOWNLOAD_NEXT",
            Self::DownloadMax => "DOWNLOAD_MAX",
            Self::ShortDownload => "SHORT_DOWNLOAD",
            Self::ModifyBits => "MODIFY_BITS",
            Self::SetCalPage => "SET_CAL_PAGE",
            Self::GetCalPage => "GET_CAL_PAGE",
            Self::GetPagProcessorInfo => "GET_PAG_PROCESSOR_INFO",
            Self::GetSegmentInfo => "GET_SEGMENT_INFO",
            Self::GetPageInfo => "GET_PAGE_INFO",
            Self::SetSegmentMode => "SET_SEGMENT_MODE",
            Self::GetSegmentMode => "GET_SEGMENT_MODE",
            Self::CopyCalPage => "COPY_CAL_PAGE",
            Self::ClearDaqList => "CLEAR_DAQ_LIST",
            Self::SetDaqPtr => "SET_DAQ_PTR",
            Self::WriteDaq => "WRITE_DAQ",
            Self::WriteDaqMultiple => "WRITE_DAQ_MULTIPLE",
            Self::SetDaqListMode => "SET_DAQ_LIST_MODE",
            Self::GetDaqListMode => "GET_DAQ_LIST_MODE",
            Self::StartStopDaqList => "START_STOP_DAQ_LIST",
            Self::StartStopSynch => "START_STOP_SYNCH",
            Self::GetDaqClock => "GET_DAQ_CLOCK",
            Self::ReadDaq => "READ_DAQ",
            Self::GetDaqProcessorInfo => "GET_DAQ_PROCESSOR_INFO",
            Self::GetDaqResolutionInfo => "GET_DAQ_RESOLUTION_INFO",
            Self::GetDaqListInfo => "GET_DAQ_LIST_INFO",
            Self::GetDaqEventInfo => "GET_DAQ_EVENT_INFO",
            Self::FreeDaq => "FREE_DAQ",
            Self::AllocDaq => "ALLOC_DAQ",
            Self::AllocOdt => "ALLOC_ODT",
            Self::AllocOdtEntry => "ALLOC_ODT_ENTRY",
            Self::ProgramStart => "PROGRAM_START",
            Self::ProgramClear => "PROGRAM_CLEAR",
            Self::Program => "PROGRAM",
            Self::ProgramReset => "PROGRAM_RESET",
            Self::GetPgmProcessorInfo => "GET_PGM_PROCESSOR_INFO",
            Self::GetSectorInfo => "GET_SECTOR_INFO",
            Self::ProgramPrepare => "PROGRAM_PREPARE",
            Self::ProgramFormat => "PROGRAM_FORMAT",
            Self::ProgramNext => "PROGRAM_NEXT",
            Self::ProgramMax => "PROGRAM_MAX",
            Self::ProgramVerify => "PROGRAM_VERIFY",
            Self::GetVersion => "GET_VERSION",
            Self::SetDaqPackedMode => "SET_DAQ_PACKED_MODE",
            Self::GetDaqPackedMode => "GET_DAQ_PACKED_MODE",
            Self::DtoCtrProperties => "DTO_CTR_PROPERTIES",
            Self::TimeCorrelationProperties => "TIME_CORRELATION_PROPERTIES",
        }
    }
}

impl core::fmt::Display for XcpCommand {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} (0x{:02X})", self.name(), self.code())
    }
}
