//! XCP-on-CAN transport bridge tests (can-core framing).
//!
//! Test file: strategic unwraps are by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use can_core::{CanError, CanFrame, CanId};
use xcp_core::{connect, transport::XcpOnCan, ByteOrder, ResourceMode, XcpCommand};

fn session() -> XcpOnCan {
    XcpOnCan::new(
        CanId::standard(0x620).unwrap(),
        CanId::standard(0x621).unwrap(),
    )
}

#[test]
fn command_frame_carries_packet() {
    let s = session();
    let packet = connect(ResourceMode::CONNECT_NORMAL);
    let frame = s.command_frame(&packet).unwrap();
    assert_eq!(frame.id(), CanId::standard(0x620).unwrap());
    assert_eq!(frame.data(), &[0xFF, 0x00]);
    assert_eq!(frame.dlc(), 2);
}

#[test]
fn command_bytes_produce_socketcan_layout() {
    let s = session();
    let bytes = s.command_bytes(&[0xFE]).unwrap();
    // struct can_frame: 4-byte LE id word, DLC byte, 3 reserved, then the
    // 8-byte inline payload.
    assert_eq!(bytes.len(), 16);
    assert_eq!(&bytes[..2], &[0x20, 0x06]); // 0x620 little-endian id word
    assert_eq!(bytes[4], 1);
    assert_eq!(bytes[8], 0xFE);
}

#[test]
fn extract_packet_accepts_response_and_daq_ids() {
    let s = session();
    let probe = [0xFF];
    let wire = s.command_bytes(&probe).unwrap();
    let frame = CanFrame::parse(&wire).unwrap();
    // Own command frames do not come back:
    assert!(s.extract_packet(&frame).is_none());

    // A frame from the slave's response ID yields the packet.
    let reply = CanFrame::new(CanId::standard(0x621).unwrap(), &[0xFF, 0x1C]).unwrap();
    assert_eq!(s.extract_packet(&reply), Some(&[0xFF, 0x1C][..]));

    // A distinct DAQ ID is honored once configured.
    let mut s2 = session();
    s2.daq = Some(CanId::standard(0x500).unwrap());
    let dto = CanFrame::new(CanId::standard(0x500).unwrap(), &[0x01, 2, 3]).unwrap();
    assert_eq!(s2.extract_packet(&dto), Some(&[0x01, 2, 3][..]));
    // ...but the unconfigured session rejects it.
    assert!(s.extract_packet(&dto).is_none());
}

#[test]
fn daq_frame_uses_override_or_response_id() {
    let s = session();
    let frame = s.daq_frame(&[0x00, 1, 2, 3]).unwrap();
    assert_eq!(frame.id(), CanId::standard(0x621).unwrap());

    let mut s2 = session();
    s2.daq = Some(CanId::extended(0x18FF_1122).unwrap());
    let frame = s2.daq_frame(&[0x00]).unwrap();
    assert_eq!(frame.id(), CanId::extended(0x18FF_1122).unwrap());
}

#[test]
fn oversized_packets_are_rejected_not_truncated() {
    let s = session();
    let packet = [0u8; 9];
    assert_eq!(
        s.command_frame(&packet),
        Err(CanError::DataTooLong { max: 8, got: 9 })
    );
}

#[test]
fn full_exchange_through_the_bridge() {
    // CONNECT out, CONNECT response in — over real can-core frames.
    let s = session();
    let connect_packet = connect(ResourceMode::CONNECT_NORMAL);
    let cmd = s.command_frame(&connect_packet).unwrap();
    assert_eq!(cmd.data(), &[0xFF, 0x00]);

    let wire = CanFrame::new(
        CanId::standard(0x621).unwrap(),
        &[0xFF, 0x1C, 0xC0, 0x08, 0x08, 0x00, 0x01, 0x01],
    )
    .unwrap();
    let packet = s.extract_packet(&wire).unwrap();
    let res =
        xcp_core::parse_response_with(ByteOrder::Intel, xcp_core::Expect::Connect, packet).unwrap();
    match res {
        xcp_core::XcpResponse::Connect(info) => assert_eq!(info.max_cto, 8),
        other => panic!("wrong variant: {other:?}"),
    }
    // The CONNECT PID decodes through the command table from the same path.
    assert_eq!(XcpCommand::from_code(packet[0]), Some(XcpCommand::Connect));
}
