//! DAQ list configuration tests.
//!
//! Test file: strategic unwraps are by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use xcp_core::{ByteOrder, DaqEntry, DaqList, Direction, StartStopMode, XcpCommand, XcpError};

fn entry(address: u32, size: u8) -> DaqEntry {
    DaqEntry {
        bit_offset: 0,
        size,
        address,
        address_extension: 0,
    }
}

#[test]
fn static_configuration_sequence_intel() {
    let mut list = DaqList::new(2, 0x0100);
    list.push_entry(entry(0x3200_1000, 2));
    list.push_entry(entry(0x3200_2000, 4));

    let frames = list.configure_frames(ByteOrder::Intel);

    // SET_DAQ_LIST_MODE (8 bytes) + (SET_DAQ_PTR + WRITE_DAQ) per entry
    // (8 + 8 bytes each).
    assert_eq!(frames.len(), 8 + 2 * 16);

    // SET_DAQ_LIST_MODE: E0, MODE=00 (DAQ, no timestamp), DAQ=02 00,
    // EVENT=00 01, PRESCALER=01, PRIORITY=00.
    assert_eq!(
        &frames[..8],
        &[0xE0, 0x00, 0x02, 0x00, 0x00, 0x01, 0x01, 0x00]
    );

    // First SET_DAQ_PTR: E2, 0,0,0, DAQ=02 00, ODT=00, ENTRY=00.
    assert_eq!(
        &frames[8..16],
        &[0xE2, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00]
    );
    // First WRITE_DAQ: E1, BIT_OFFSET=00, SIZE=02, EXT=00, ADDR LE.
    assert_eq!(
        &frames[16..24],
        &[0xE1, 0x00, 0x02, 0x00, 0x00, 0x10, 0x00, 0x32]
    );

    // Second pointer advances the ODT number.
    assert_eq!(
        &frames[24..32],
        &[0xE2, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00]
    );
    // Second WRITE_DAQ.
    assert_eq!(
        &frames[32..40],
        &[0xE1, 0x00, 0x04, 0x00, 0x00, 0x20, 0x00, 0x32]
    );
}

#[test]
fn static_configuration_sequence_motorola() {
    let mut list = DaqList::new(1, 5);
    list.timestamp = true;
    list.push_entry(entry(0x0000_1234, 1));

    let frames = list.configure_frames(ByteOrder::Motorola);

    // MODE byte with timestamp: 0x10. DAQ/event/addr big-endian.
    assert_eq!(
        &frames[..8],
        &[0xE0, 0x10, 0x00, 0x01, 0x00, 0x05, 0x01, 0x00]
    );
    assert_eq!(
        &frames[8..16],
        &[0xE2, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00]
    );
    assert_eq!(
        &frames[16..24],
        &[0xE1, 0x00, 0x01, 0x00, 0x00, 0x00, 0x12, 0x34]
    );
}

#[test]
fn mode_bits_encode_direction_and_options() {
    let mut list = DaqList::new(0, 0);
    assert_eq!(list.mode_bits(), 0x00);
    list.timestamp = true;
    assert_eq!(list.mode_bits(), 0x10);
    list.pid_off = true;
    assert_eq!(list.mode_bits(), 0x30);
    list.direction = Direction::Stim;
    assert_eq!(list.mode_bits(), 0x32);
}

#[test]
fn run_control_frames() {
    let list = DaqList::new(3, 7);
    assert_eq!(list.start_frame(), [0xDE, 0x01, 0x03, 0x00]);
    assert_eq!(list.stop_frame(), [0xDE, 0x00, 0x03, 0x00]);
    assert_eq!(
        list.start_stop_frame_with(ByteOrder::Motorola, StartStopMode::Select),
        [0xDE, 0x02, 0x00, 0x03]
    );
    assert_eq!(DaqList::start_synch_frame(), [0xDD, 0x00]);
    assert_eq!(DaqList::stop_synch_frame(), [0xDD, 0x01]);
}

#[test]
fn configuration_decodes_as_commands() {
    // Every 8-byte block of the sequence is a valid command frame whose
    // PID decodes through the command table.
    let mut list = DaqList::new(1, 1);
    list.push_entry(entry(0x42, 1));
    let frames = list.configure_frames(ByteOrder::Intel);
    for chunk in frames.chunks_exact(8) {
        let cmd = XcpCommand::from_code(chunk[0]).unwrap();
        assert!(
            matches!(
                cmd,
                XcpCommand::SetDaqListMode | XcpCommand::SetDaqPtr | XcpCommand::WriteDaq
            ),
            "{cmd}"
        );
    }
}

#[test]
fn empty_list_configures_mode_only() {
    let list = DaqList::new(9, 2);
    let frames = list.configure_frames(ByteOrder::Intel);
    assert_eq!(frames.len(), 8);
    assert_eq!(frames[0], 0xE0);
}

#[test]
fn daq_entry_fields_survive_construction() {
    let e = DaqEntry {
        bit_offset: 3,
        size: 2,
        address: 0xF000_0000,
        address_extension: 1,
    };
    let mut list = DaqList::new(0, 0);
    list.push_entry(e);
    assert_eq!(list.entries, vec![e]);
    assert_eq!(list.entries[0].bit_offset, 3);
}

#[test]
fn oversized_lists_saturate_not_panic() {
    // 300 entries: ODT numbering saturates at u8::MAX rather than
    // panicking — real slaves cap MAX_ODT far earlier.
    let mut list = DaqList::new(0, 0);
    for i in 0..300u32 {
        list.push_entry(entry(i, 1));
    }
    let frames = list.configure_frames(ByteOrder::Intel);
    assert_eq!(frames.len(), 8 + 300 * 16);
    // Last entry's ODT byte is the saturation value.
    let last_ptr = &frames[8 + 299 * 16..8 + 299 * 16 + 8];
    assert_eq!(last_ptr[6], u8::MAX);
}

#[test]
fn error_taxonomy_documents_daq_codes() {
    // The DAQ-relevant slave errors keep their spec codes.
    assert_eq!(XcpError::DaqActive.slave_code(), Some(0x11));
    assert_eq!(XcpError::DaqConfig.slave_code(), Some(0x2A));
    assert_eq!(XcpError::PgmActive.slave_code(), Some(0x12));
}
