//! Command-code table and command-frame builder tests.
//!
//! Test file: fixed assertions and strategic unwraps are by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use xcp_core::{
    connect, disconnect, download, download_frames, download_with, get_cal_page,
    get_comm_mode_info, get_id, get_status, get_version, set_cal_page, set_mta, short_download,
    short_upload, synch, upload, upload_frame, upload_with, ByteOrder, ResourceMode, XcpCommand,
};

#[test]
fn command_table_is_complete_and_bijective() {
    // Every defined code maps to a command…
    let commands = [
        (0xFF, XcpCommand::Connect),
        (0xFE, XcpCommand::Disconnect),
        (0xFD, XcpCommand::GetStatus),
        (0xFC, XcpCommand::Synch),
        (0xFB, XcpCommand::GetCommModeInfo),
        (0xFA, XcpCommand::GetId),
        (0xF9, XcpCommand::SetRequestMode),
        (0xF8, XcpCommand::GetSeed),
        (0xF7, XcpCommand::Unlock),
        (0xF6, XcpCommand::SetMta),
        (0xF5, XcpCommand::Upload),
        (0xF4, XcpCommand::ShortUpload),
        (0xF3, XcpCommand::BuildChecksum),
        (0xF2, XcpCommand::TransportLayerCommand),
        (0xF1, XcpCommand::UserCommand),
        (0xF0, XcpCommand::Download),
        (0xEF, XcpCommand::DownloadNext),
        (0xEE, XcpCommand::DownloadMax),
        (0xED, XcpCommand::ShortDownload),
        (0xEC, XcpCommand::ModifyBits),
        (0xEB, XcpCommand::SetCalPage),
        (0xEA, XcpCommand::GetCalPage),
        (0xE9, XcpCommand::GetPagProcessorInfo),
        (0xE8, XcpCommand::GetSegmentInfo),
        (0xE7, XcpCommand::GetPageInfo),
        (0xE6, XcpCommand::SetSegmentMode),
        (0xE5, XcpCommand::GetSegmentMode),
        (0xE4, XcpCommand::CopyCalPage),
        (0xE3, XcpCommand::ClearDaqList),
        (0xE2, XcpCommand::SetDaqPtr),
        (0xE1, XcpCommand::WriteDaq),
        (0xE0, XcpCommand::SetDaqListMode),
        (0xDF, XcpCommand::GetDaqListMode),
        (0xDE, XcpCommand::StartStopDaqList),
        (0xDD, XcpCommand::StartStopSynch),
        (0xDC, XcpCommand::GetDaqClock),
        (0xDB, XcpCommand::ReadDaq),
        (0xDA, XcpCommand::GetDaqProcessorInfo),
        (0xD9, XcpCommand::GetDaqResolutionInfo),
        (0xD8, XcpCommand::GetDaqListInfo),
        (0xD7, XcpCommand::GetDaqEventInfo),
        (0xD6, XcpCommand::FreeDaq),
        (0xD5, XcpCommand::AllocDaq),
        (0xD4, XcpCommand::AllocOdt),
        (0xD3, XcpCommand::AllocOdtEntry),
        (0xD2, XcpCommand::ProgramStart),
        (0xD1, XcpCommand::ProgramClear),
        (0xD0, XcpCommand::Program),
        (0xCF, XcpCommand::ProgramReset),
        (0xCE, XcpCommand::GetPgmProcessorInfo),
        (0xCD, XcpCommand::GetSectorInfo),
        (0xCC, XcpCommand::ProgramPrepare),
        (0xCB, XcpCommand::ProgramFormat),
        (0xCA, XcpCommand::ProgramNext),
        (0xC9, XcpCommand::ProgramMax),
        (0xC8, XcpCommand::ProgramVerify),
        (0xC0, XcpCommand::GetVersion),
        (0xC1, XcpCommand::SetDaqPackedMode),
        (0xC2, XcpCommand::GetDaqPackedMode),
        (0xC5, XcpCommand::DtoCtrProperties),
        (0xC6, XcpCommand::TimeCorrelationProperties),
    ];
    for (code, command) in commands {
        assert_eq!(XcpCommand::from_code(code), Some(command), "code {code:#x}");
        assert_eq!(command.code(), code);
        assert!(command.name().is_ascii());
    }
    // WRITE_DAQ_MULTIPLE sits in the reserved hole set at 0xC7.
    assert_eq!(
        XcpCommand::from_code(0xC7),
        Some(XcpCommand::WriteDaqMultiple)
    );
}

#[test]
fn reserved_codes_are_none() {
    for code in [0x00u8, 0x3F, 0xBE, 0xC3, 0xC4] {
        assert!(XcpCommand::from_code(code).is_none(), "0x{code:02X}");
    }
}

#[test]
fn display_is_spec_name_and_code() {
    assert_eq!(XcpCommand::Connect.to_string(), "CONNECT (0xFF)");
    assert_eq!(XcpCommand::SetMta.to_string(), "SET_MTA (0xF6)");
    assert_eq!(XcpCommand::GetVersion.to_string(), "GET_VERSION (0xC0)");
}

#[test]
fn basic_command_frames() {
    assert_eq!(connect(ResourceMode::CONNECT_NORMAL), [0xFF, 0x00]);
    assert_eq!(connect(ResourceMode::CONNECT_USER_DEFINED), [0xFF, 0x01]);
    assert_eq!(disconnect(), [0xFE]);
    assert_eq!(get_status(), [0xFD]);
    assert_eq!(synch(), [0xFC]);
    assert_eq!(get_comm_mode_info(), [0xFB]);
    assert_eq!(get_version(), [0xC0, 0x00]);
    assert_eq!(get_id(0x00), [0xFA, 0x00, 0x00, 0x00]);
    assert_eq!(upload_frame(4), [0xF5, 0x04]);
}

#[test]
fn set_mta_byte_orders() {
    // Intel (little-endian): 0x8000_0000 → 00 00 00 80.
    assert_eq!(
        set_mta(ByteOrder::Intel, 0x8000_0000, 0x00),
        [0xF6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80]
    );
    // Motorola (big-endian): 0x8000_0000 → 80 00 00 00.
    assert_eq!(
        set_mta(ByteOrder::Motorola, 0x8000_0000, 0x05),
        [0xF6, 0x00, 0x00, 0x05, 0x80, 0x00, 0x00, 0x00]
    );
}

#[test]
fn upload_sequence_shapes() {
    // SET_MTA + UPLOAD in one sequence.
    let seq = upload_with(ByteOrder::Motorola, 0x0040_0000, 0x01, 6);
    assert_eq!(seq.len(), 10);
    assert_eq!(&seq[..8], &[0xF6, 0x00, 0x00, 0x01, 0x00, 0x40, 0x00, 0x00]);
    assert_eq!(&seq[8..], &[0xF5, 0x06]);

    // Contract form: Intel, extension 0.
    assert_eq!(
        upload(0x1000, 2),
        upload_with(ByteOrder::Intel, 0x1000, 0, 2)
    );

    // SHORT_UPLOAD: single frame, no MTA change. Canonical capture vector:
    // read 4 bytes from 0x80000000 with Intel byte order.
    assert_eq!(
        short_upload(ByteOrder::Intel, 0x8000_0000, 0x00, 0x04),
        [0xF4, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80]
    );
}

#[test]
fn download_sequence_shapes_and_block_mode() {
    // Small write: SET_MTA + one DOWNLOAD frame.
    let seq = download(0x4000_0000, &[0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(
        seq,
        [
            0xF6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, // SET_MTA
            0xF0, 0x04, 0xDE, 0xAD, 0xBE, 0xEF, // DOWNLOAD
        ]
    );

    // Empty write is a legal probe: DOWNLOAD with element count 0.
    assert_eq!(download(0x1234, &[]).len(), 8 + 2);

    // Block mode: 253 bytes fill one DOWNLOAD; the rest rides DOWNLOAD_NEXT.
    let data: Vec<u8> = (0..=254u8).collect(); // 255 bytes
    let frames = download_frames(&data);
    assert_eq!(frames[0], 0xF0);
    assert_eq!(frames[1], 253);
    assert_eq!(frames[2 + 253], 0xEF);
    assert_eq!(frames[3 + 253], 2);
    assert_eq!(&frames[4 + 253..], &[253, 254]);
    assert_eq!(frames.len(), 2 + 253 + 2 + 2);

    // Byte-order passthrough on the MTA prefix (Motorola):
    let mut expected = set_mta(ByteOrder::Motorola, 0x0102_0304, 0);
    expected.extend_from_slice(&download_frames(&[1]));
    assert_eq!(
        download_with(ByteOrder::Motorola, 0x0102_0304, 0, &[1]),
        expected
    );
}

#[test]
fn short_download_shape() {
    assert_eq!(
        short_download(ByteOrder::Intel, 0x0000_1000, 0x00, &[0xAA, 0xBB]),
        [0xED, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0xAA, 0xBB]
    );
}

#[test]
fn cal_page_frames() {
    // Switch CPU to page 1 of segment 0.
    assert_eq!(
        set_cal_page(ByteOrder::Intel, 0x01, 0x0000),
        [0xEB, 0x01, 0x00, 0x00]
    );
    assert_eq!(
        get_cal_page(ByteOrder::Motorola, 0x02, 0x0001),
        [0xEA, 0x02, 0x00, 0x01]
    );
}
