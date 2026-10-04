//! proptest: address/data framing round-trips, byte-order consistency, and
//! parser totality. 500 cases per property.
//!
//! Test file: strategic unwraps are by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use proptest::prelude::*;
use xcp_core::{
    download_with, parse_response, parse_response_with, short_upload, upload_with, ByteOrder,
    CommModeBasic, ConnectInfo, Expect, ResourceMode, XcpCommand, XcpError, XcpResponse,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// The SET_MTA + DOWNLOAD sequence round-trips: the MTA address is
    /// recoverable from the first 8 bytes in either byte order, and every
    /// chunk's payload is byte-exact.
    #[test]
    fn download_roundtrip(
        addr in any::<u32>(),
        ext in any::<u8>(),
        bo in prop::sample::select(vec![ByteOrder::Intel, ByteOrder::Motorola]),
        data in proptest::collection::vec(any::<u8>(), 0..600),
    ) {
        let frames = download_with(bo, addr, ext, &data);

        // SET_MTA prefix: PID F6, two reserved, extension, address.
        prop_assert_eq!(&frames[..3], &[0xF6, 0x00, 0x00]);
        prop_assert_eq!(frames[3], ext);
        let mtu_addr = match bo {
            ByteOrder::Intel => u32::from_le_bytes(frames[4..8].try_into().unwrap()),
            ByteOrder::Motorola => u32::from_be_bytes(frames[4..8].try_into().unwrap()),
        };
        prop_assert_eq!(mtu_addr, addr);

        // Payload reassembly across the chunk boundaries is exact.
        let mut rebuilt = Vec::with_capacity(data.len());
        let mut i = 8;
        while i < frames.len() {
            let pid = frames[i];
            let n = frames[i + 1] as usize;
            let expected_pid = if rebuilt.is_empty() {
                XcpCommand::Download.code()
            } else {
                XcpCommand::DownloadNext.code()
            };
            prop_assert_eq!(pid, expected_pid, "chunk pid at byte {}", i);
            prop_assert!(n <= 254);
            rebuilt.extend_from_slice(&frames[i + 2..i + 2 + n]);
            i += 2 + n;
        }
        prop_assert_eq!(rebuilt, data);
    }

    /// SHORT_UPLOAD frames: the address encodes per byte order and the
    /// element count is the second byte.
    #[test]
    fn short_upload_address_roundtrip(
        addr in any::<u32>(),
        ext in any::<u8>(),
        len in any::<u8>(),
        bo in prop::sample::select(vec![ByteOrder::Intel, ByteOrder::Motorola]),
    ) {
        let frame = short_upload(bo, addr, ext, len);
        prop_assert_eq!(frame.len(), 8);
        prop_assert_eq!(&frame[..2], &[0xF4, len]);
        prop_assert_eq!(frame[2], 0x00);
        prop_assert_eq!(frame[3], ext);
        let decoded = match bo {
            ByteOrder::Intel => u32::from_le_bytes(frame[4..8].try_into().unwrap()),
            ByteOrder::Motorola => u32::from_be_bytes(frame[4..8].try_into().unwrap()),
        };
        prop_assert_eq!(decoded, addr);
    }

    /// UPLOAD sequences: SET_MTA + UPLOAD frame, address recoverable in
    /// both byte orders.
    #[test]
    fn upload_sequence_roundtrip(
        addr in any::<u32>(),
        len in 0u8..=254,
        bo in prop::sample::select(vec![ByteOrder::Intel, ByteOrder::Motorola]),
    ) {
        let seq = upload_with(bo, addr, 0, len);
        prop_assert_eq!(seq.len(), 10);
        prop_assert_eq!(seq[0], XcpCommand::SetMta.code());
        prop_assert_eq!(&seq[8..], &[XcpCommand::Upload.code(), len]);
        let decoded = match bo {
            ByteOrder::Intel => u32::from_le_bytes(seq[4..8].try_into().unwrap()),
            ByteOrder::Motorola => u32::from_be_bytes(seq[4..8].try_into().unwrap()),
        };
        prop_assert_eq!(decoded, addr);
    }

    /// CONNECT response encode→decode: any resource/comm-mode combination
    /// and sizes survive a synthetic wire round-trip.
    #[test]
    fn connect_response_roundtrip(
        resource in any::<u8>(),
        cmb in any::<u8>(),
        max_cto in any::<u8>(),
        max_dto in any::<u16>(),
        pver in 0u8..=16,
        tver in 0u8..=16,
        bo in prop::sample::select(vec![ByteOrder::Intel, ByteOrder::Motorola]),
    ) {
        let mut payload = vec![0xFF, resource, cmb, max_cto];
        bo.put_u16(&mut payload, max_dto);
        payload.extend_from_slice(&[pver, tver]);

        let res = parse_response_with(bo, Expect::Connect, &payload).unwrap();
        match res {
            XcpResponse::Connect(ConnectInfo {
                resource: r,
                comm_mode,
                max_cto: cto,
                max_dto: dto,
                protocol_version: (pm, _),
                transport_version: (tm, _),
            }) => {
                prop_assert_eq!(r.bits(), resource);
                prop_assert_eq!(cto, max_cto);
                prop_assert_eq!(dto, max_dto);
                prop_assert_eq!(pm, pver);
                prop_assert_eq!(tm, tver);
                prop_assert_eq!(comm_mode, CommModeBasic::from_raw(cmb));
                // to_raw reproduces the modeled bits (7, 6, 2–1, 0);
                // reserved bits 3–5 are dropped by design.
                prop_assert_eq!(comm_mode.to_raw(), cmb & 0xC7);
            }
            other => prop_assert!(false, "wrong variant {other:?}"),
        }
    }

    /// parse_response is total: arbitrary bytes never panic, and every ERR
    /// packet maps to the typed slave error with the same code byte.
    #[test]
    fn parse_is_total(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        // Must not panic:
        let _ = parse_response(&bytes);
        let _ = parse_response_with(ByteOrder::Motorola, Expect::GetStatus, &bytes);
        // Any ERR packet yields exactly the typed slave code:
        if let [0xFE, code, rest @ ..] = &bytes[..] {
            let err = parse_response(&bytes).unwrap_err();
            prop_assert_eq!(err.slave_code(), Some(*code));
            prop_assert!(rest.len() < 64); // keep `rest` used without indexing games
        }
    }

    /// Slave error code ↔ typed error is a bijection for the ASAM table
    /// and SlaveOther for everything else.
    #[test]
    fn error_code_bijection(code in any::<u8>()) {
        let err = XcpError::from_slave_code(code);
        match code {
            0x00 | 0x10..=0x12 | 0x20..=0x2A | 0x30..=0x33 => {
                prop_assert_eq!(err.slave_code(), Some(code));
            }
            other => {
                prop_assert_eq!(err, XcpError::SlaveOther(other));
                prop_assert_eq!(err.slave_code(), Some(other));
            }
        }
    }

    /// ResourceMode flag algebra: union/intersection/contains agree with
    /// the underlying bits for arbitrary bytes.
    #[test]
    fn resource_mode_algebra(a in any::<u8>(), b in any::<u8>()) {
        let (ra, rb) = (ResourceMode::from_raw(a), ResourceMode::from_raw(b));
        prop_assert_eq!((ra | rb).bits(), a | b);
        prop_assert_eq!((ra & rb).bits(), a & b);
        prop_assert_eq!(ra.contains(rb), a & b == b);
        prop_assert_eq!(ra.is_empty(), a == 0);
        // CAL_PAGE_OR_DAQ composite behaves as its name.
        let composite = ResourceMode::CAL_PAGE_OR_DAQ;
        prop_assert_eq!(composite.bits(), 0x05);
        prop_assert!(composite.contains(ResourceMode::CAL_PAGE));
        prop_assert!(composite.contains(ResourceMode::DAQ));
    }

    /// DAQ list configuration round-trip: any number/event/entry set
    /// re-encodes to a parseable command sequence with matching fields.
    #[test]
    fn daq_configuration_roundtrip(
        number in any::<u16>(),
        event in any::<u16>(),
        prescaler in 1u8..=255,
        bo in prop::sample::select(vec![ByteOrder::Intel, ByteOrder::Motorola]),
        addrs in proptest::collection::vec(any::<u32>(), 0..8),
    ) {
        let mut list = xcp_core::DaqList::new(number, event);
        list.prescaler = prescaler;
        for addr in &addrs {
            list.push_entry(xcp_core::DaqEntry {
                bit_offset: 0,
                size: 1,
                address: *addr,
                address_extension: 0,
            });
        }
        let frames = list.configure_frames(bo);

        // Mode block:
        prop_assert_eq!(frames[0], XcpCommand::SetDaqListMode.code());
        let decoded_number = bo.get_u16(&frames[2..]).unwrap();
        let decoded_event = bo.get_u16(&frames[4..]).unwrap();
        prop_assert_eq!(decoded_number, number);
        prop_assert_eq!(decoded_event, event);
        prop_assert_eq!(frames[6], prescaler);

        // Entry blocks:
        for (i, addr) in addrs.iter().enumerate() {
            let base = 8 + i * 16;
            prop_assert_eq!(frames[base], XcpCommand::SetDaqPtr.code());
            prop_assert_eq!(bo.get_u16(&frames[base + 4..]).unwrap(), number);
            let wd = base + 8;
            prop_assert_eq!(frames[wd], XcpCommand::WriteDaq.code());
            prop_assert_eq!(bo.get_u32(&frames[wd + 4..]).unwrap(), *addr);
        }
    }
}
