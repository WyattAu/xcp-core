//! Response parsing and error-response handling tests, pinned to
//! known-good wire captures.
//!
//! Test file: strategic unwraps are by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use xcp_core::{
    parse_response, parse_response_with, AddressGranularity, ByteOrder, CommModeBasic, ConnectInfo,
    Expect, ResourceMode, SessionStatus, StatusInfo, VersionInfo, XcpError, XcpResponse,
};

/// Canonical CONNECT response captured from a reference slave:
/// DAQ+STIM+PGM resources, optional + slave-block mode, Intel byte order,
/// byte address granularity, `MAX_CTO` = `MAX_DTO` = 8, protocol/transport 1.0.
const CONNECT_REPLY: [u8; 8] = [0xFF, 0x1C, 0xC0, 0x08, 0x08, 0x00, 0x01, 0x01];

#[test]
fn connect_response_known_vector() {
    let res = parse_response_with(ByteOrder::Intel, Expect::Connect, &CONNECT_REPLY).unwrap();
    assert_eq!(
        res,
        XcpResponse::Connect(ConnectInfo {
            resource: ResourceMode::DAQ | ResourceMode::STIM | ResourceMode::PGM,
            comm_mode: CommModeBasic {
                byte_order: ByteOrder::Intel,
                address_granularity: AddressGranularity::Byte,
                slave_block_mode: true,
                optional_info: true,
            },
            max_cto: 8,
            max_dto: 8,
            protocol_version: (1, 0),
            transport_version: (1, 0),
        })
    );
    let info = match res {
        XcpResponse::Connect(info) => info,
        other => panic!("wrong variant: {other:?}"),
    };
    assert!(info.resource.daq());
    assert!(!info.resource.cal_page());
    assert!(info.comm_mode.slave_block_mode);
    assert_eq!(info.comm_mode.byte_order, ByteOrder::Intel);
}

#[test]
fn connect_response_motorola_byte_order() {
    // Same reply with Motorola (bit 0 of COMM_MODE_BASIC set) and a word
    // MAX_DTO; MAX_DTO must decode big-endian.
    let reply = [0xFF, 0x05, 0xC3, 0x08, 0x01, 0x00, 0x01, 0x01];
    let res = parse_response_with(ByteOrder::Motorola, Expect::Connect, &reply).unwrap();
    match res {
        XcpResponse::Connect(info) => {
            assert_eq!(info.comm_mode.byte_order, ByteOrder::Motorola);
            assert_eq!(info.max_dto, 0x0100);
            assert!(info.resource.cal_page());
            assert!(info.resource.daq());
            assert!(!info.resource.pgm());
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn connect_response_rejects_short_payload() {
    // Empty input hits the packet-level minimum first.
    assert_eq!(
        parse_response_with(ByteOrder::Intel, Expect::Connect, &[]),
        Err(XcpError::BufferTooShort { need: 1, have: 0 })
    );
    for len in 1..7 {
        let res = parse_response_with(ByteOrder::Intel, Expect::Connect, &CONNECT_REPLY[..len]);
        assert_eq!(
            res,
            Err(XcpError::BufferTooShort { need: 8, have: len }),
            "len {len}"
        );
    }
}

#[test]
fn upload_response_roundtrip() {
    // Positive UPLOAD response: PID + uploaded bytes.
    let res = parse_response_with(ByteOrder::Intel, Expect::Upload, &[0xFF, 1, 2, 3]).unwrap();
    assert_eq!(res, XcpResponse::Upload(vec![1, 2, 3]));
    // Zero-length upload is a bare ack.
    let res = parse_response_with(ByteOrder::Intel, Expect::Upload, &[0xFF]).unwrap();
    assert_eq!(res, XcpResponse::Upload(vec![]));
}

#[test]
fn get_status_response() {
    // session: STORE_CAL_REQUEST | DAQ_ACTIVE; protection: CAL/PAG only.
    let reply = [0xFF, 0x41, 0x01, 0x00, 0xAB, 0xCD];
    let res = parse_response_with(ByteOrder::Intel, Expect::GetStatus, &reply).unwrap();
    match res {
        XcpResponse::GetStatus(StatusInfo {
            session_status,
            protection_status,
            session_cfg_id,
        }) => {
            assert!(session_status.contains(SessionStatus::DAQ_ACTIVE));
            assert!(session_status.contains(SessionStatus::STORE_CAL_REQUEST));
            assert!(!session_status.contains(SessionStatus::RESUME));
            assert_eq!(protection_status, ResourceMode::CAL_PAGE);
            assert_eq!(session_cfg_id, 0xCDAB);
        }
        other => panic!("wrong variant: {other:?}"),
    }
    // Motorola order swaps the config id.
    let res = parse_response_with(ByteOrder::Motorola, Expect::GetStatus, &reply).unwrap();
    match res {
        XcpResponse::GetStatus(info) => assert_eq!(info.session_cfg_id, 0xABCD),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn get_version_response() {
    let reply = [0xFF, 0x00, 0x01, 0x05, 0x01, 0x03];
    let res = parse_response_with(ByteOrder::Intel, Expect::GetVersion, &reply).unwrap();
    assert_eq!(
        res,
        XcpResponse::GetVersion(VersionInfo {
            protocol_version: (1, 5),
            transport_version: (1, 3),
        })
    );
}

#[test]
fn get_id_response() {
    let reply = [0xFF, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00];
    let res = parse_response_with(ByteOrder::Intel, Expect::GetId, &reply).unwrap();
    match res {
        XcpResponse::GetId(info) => {
            assert_eq!(info.mode, 0);
            assert_eq!(info.length, 10);
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn start_stop_and_daq_clock_responses() {
    let res =
        parse_response_with(ByteOrder::Intel, Expect::StartStopDaqList, &[0xFF, 0x20]).unwrap();
    assert_eq!(res, XcpResponse::StartStopDaqList { first_pid: 0x20 });

    let res = parse_response_with(
        ByteOrder::Motorola,
        Expect::GetDaqClock,
        &[0xFF, 0, 0, 0, 0x11, 0x22, 0x33, 0x44],
    )
    .unwrap();
    match res {
        XcpResponse::GetDaqClock { timestamp } => assert_eq!(timestamp, 0x1122_3344),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn generic_positive_responses() {
    // DISCONNECT ack: bare RES.
    let res = parse_response(&[0xFF]).unwrap();
    assert_eq!(res, XcpResponse::Generic { data: vec![] });

    // SET_MTA / DOWNLOAD acks may carry trailing pad bytes.
    let res = parse_response(&[0xFF, 0x00, 0x00, 0x00]).unwrap();
    assert_eq!(
        res,
        XcpResponse::Generic {
            data: vec![0, 0, 0]
        }
    );

    // Small uploads fall through to Generic under the heuristic (documented
    // collision) — the Expect-typed path is the precise one.
    let res = parse_response(&[0xFF, 9, 9]).unwrap();
    assert_eq!(res, XcpResponse::Generic { data: vec![9, 9] });
}

#[test]
fn inference_heuristic() {
    // 8 total bytes → CONNECT.
    assert!(matches!(
        parse_response(&CONNECT_REPLY),
        Ok(XcpResponse::Connect(_))
    ));
    // 6 total, reserved 0x00 → GET_VERSION.
    assert!(matches!(
        parse_response(&[0xFF, 0x00, 0x01, 0x05, 0x01, 0x00]),
        Ok(XcpResponse::GetVersion(_))
    ));
    // 6 total, non-zero first payload byte → GET_STATUS.
    assert!(matches!(
        parse_response(&[0xFF, 0x41, 0x01, 0x00, 0xAB, 0xCD]),
        Ok(XcpResponse::GetStatus(_))
    ));
}

#[test]
fn event_and_service_packets() {
    // EV: DAQ overload (0x06) with no payload.
    let res = parse_response(&[0xFD, 0x06]).unwrap();
    assert_eq!(
        res,
        XcpResponse::Event {
            code: 0x06,
            data: vec![]
        }
    );
    // SERV: reset request (0x01).
    let res = parse_response(&[0xFC, 0x01]).unwrap();
    assert_eq!(
        res,
        XcpResponse::Service {
            code: 0x01,
            data: vec![]
        }
    );
    // EV/SERV without the code byte is too short.
    assert_eq!(
        parse_response(&[0xFD]),
        Err(XcpError::BufferTooShort { need: 2, have: 1 })
    );
}

#[test]
fn error_response_handling_full_taxonomy() {
    // One wire packet per ASAM ERR code, and the typed error it must
    // produce.
    let cases: [(u8, XcpError); 20] = [
        (0x00, XcpError::CmdSynch),
        (0x10, XcpError::CmdBusy),
        (0x11, XcpError::DaqActive),
        (0x12, XcpError::PgmActive),
        (0x20, XcpError::CmdUnknown),
        (0x21, XcpError::CmdSyntax),
        (0x22, XcpError::OutOfRange),
        (0x23, XcpError::WriteProtected),
        (0x24, XcpError::AccessDenied),
        (0x25, XcpError::AccessLocked),
        (0x26, XcpError::PageNotValid),
        (0x27, XcpError::ModeNotValid),
        (0x28, XcpError::SegmentNotValid),
        (0x29, XcpError::Sequence),
        (0x2A, XcpError::DaqConfig),
        (0x30, XcpError::MemoryOverflow),
        (0x31, XcpError::Generic),
        (0x32, XcpError::Verify),
        (0x33, XcpError::ResourceTemporaryNotAccessible),
        (0x7F, XcpError::SlaveOther(0x7F)),
    ];
    for (code, expected) in cases {
        let res = parse_response(&[0xFE, code]);
        assert_eq!(res, Err(expected), "code {code:#04x}");
        assert!(expected.is_slave_error());
        assert_eq!(expected.slave_code(), Some(code));
    }
    // ERR without the code byte:
    assert_eq!(
        parse_response(&[0xFE]),
        Err(XcpError::BufferTooShort { need: 2, have: 1 })
    );
    // Structural errors report no slave code:
    assert_eq!(XcpError::UnknownPid(0x55).slave_code(), None);
    assert!(!XcpError::UnknownPid(0x55).is_slave_error());
}

#[test]
fn error_display_and_codes_are_complete() {
    // Every slave-code variant: Display carries the code, slave_code()
    // round-trips, and structural errors carry no code.
    let all: [XcpError; 22] = [
        XcpError::BufferTooShort { need: 2, have: 1 },
        XcpError::UnknownPid(0x42),
        XcpError::CmdSynch,
        XcpError::CmdBusy,
        XcpError::DaqActive,
        XcpError::PgmActive,
        XcpError::CmdUnknown,
        XcpError::CmdSyntax,
        XcpError::OutOfRange,
        XcpError::WriteProtected,
        XcpError::AccessDenied,
        XcpError::AccessLocked,
        XcpError::PageNotValid,
        XcpError::ModeNotValid,
        XcpError::SegmentNotValid,
        XcpError::Sequence,
        XcpError::DaqConfig,
        XcpError::MemoryOverflow,
        XcpError::Generic,
        XcpError::Verify,
        XcpError::ResourceTemporaryNotAccessible,
        XcpError::SlaveOther(0xEE),
    ];
    for err in all {
        let msg = err.to_string();
        assert_ne!(msg, "");
        // Errors are usable as std errors through the core::error impl.
        let _: &dyn core::error::Error = &err;
        match err {
            XcpError::BufferTooShort { .. } | XcpError::UnknownPid(_) => {
                assert!(msg.contains("buffer") || msg.contains("PID"));
            }
            _ => assert!(msg.contains("ERR"), "{msg}"),
        }
    }
    assert_eq!(
        XcpError::BufferTooShort { need: 4, have: 1 }.to_string(),
        "buffer too short: need 4 bytes, have 1"
    );
    assert_eq!(
        XcpError::UnknownPid(0x42).to_string(),
        "not an XCP response PID: 0x42"
    );
    assert_eq!(
        XcpError::SlaveOther(0xEE).to_string(),
        "ERR 0xEE: slave-specific error code"
    );
    assert_eq!(
        XcpError::ResourceTemporaryNotAccessible.slave_code(),
        Some(0x33)
    );
}

#[test]
fn resource_and_session_display() {
    // Empty set:
    assert_eq!(ResourceMode::from_raw(0).to_string(), "none");
    // Named flags join with |:
    assert_eq!(
        (ResourceMode::CAL_PAGE | ResourceMode::DAQ).to_string(),
        "cal_pag|daq"
    );
    // PGM + STIM:
    assert_eq!(
        (ResourceMode::PGM | ResourceMode::STIM).to_string(),
        "stim|pgm"
    );
    // Reserved bits render as hex after the named flags:
    assert_eq!(
        (ResourceMode::CAL_PAGE | ResourceMode::from_raw(0x40)).to_string(),
        "cal_pag|0x40"
    );
    // Complement:
    assert_eq!((!ResourceMode::from_raw(0xFF)).bits(), 0x00);
    assert_eq!(
        (!ResourceMode::CAL_PAGE).bits(),
        !ResourceMode::CAL_PAGE.bits()
    );
    // Accessors:
    let full =
        ResourceMode::CAL_PAGE_OR_DAQ | ResourceMode::STIM | ResourceMode::PGM | ResourceMode::DBG;
    assert!(full.stim());
    assert!(full.pgm());
    assert!(full.cal_page());
    assert!(full.daq());
    assert!(!ResourceMode::DAQ.stim());
    // Session status Display is the raw hex byte:
    assert_eq!(
        (SessionStatus::DAQ_ACTIVE | SessionStatus::RESUME).to_string(),
        "0xC0"
    );
    assert_eq!(SessionStatus::from_raw(0).to_string(), "0x00");
    assert!(SessionStatus::from_raw(0x41).daq_active());
    assert!(SessionStatus::from_raw(0x01).contains(SessionStatus::STORE_CAL_REQUEST));
    assert!(SessionStatus::from_raw(0).is_empty());
}

#[test]
fn synch_completes_with_cmd_synch() {
    // The SYNCH handshake's success indicator is ERR 0x00 — modelled as
    // XcpError::CmdSynch, which session code matches explicitly.
    assert_eq!(parse_response(&[0xFE, 0x00]), Err(XcpError::CmdSynch));
}

#[test]
fn unknown_pid_is_typed() {
    // Bytes below the response-PID range are STIM PIDs, not responses.
    for pid in [0x00u8, 0x01, 0x3F, 0x80, 0xE0] {
        assert_eq!(parse_response(&[pid]), Err(XcpError::UnknownPid(pid)));
    }
}

#[test]
fn empty_buffer_is_typed() {
    assert_eq!(
        parse_response(&[]),
        Err(XcpError::BufferTooShort { need: 1, have: 0 })
    );
}

#[test]
fn trailing_padding_is_tolerated() {
    // A GET_STATUS response padded to a 8-byte classic CAN frame.
    let padded = [0xFF, 0x41, 0x01, 0x00, 0xAB, 0xCD, 0xCC, 0xCC];
    let res = parse_response_with(ByteOrder::Intel, Expect::GetStatus, &padded).unwrap();
    assert!(matches!(res, XcpResponse::GetStatus(_)));
}
