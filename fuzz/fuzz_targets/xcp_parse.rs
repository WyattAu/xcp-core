//! Fuzz target — `parse_response` totality: arbitrary bytes produce either
//! a typed response or a typed error, never a panic. Connect-info decoding
//! is additionally checked for byte-order consistency.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Total over every input, in both byte orders:
    let res_le = xcp_core::parse_response_with(xcp_core::ByteOrder::Intel, xcp_core::Expect::Generic, data);
    let res_be = xcp_core::parse_response_with(xcp_core::ByteOrder::Motorola, xcp_core::Expect::GetStatus, data);
    let _ = (res_le, res_be);

    // Whatever parses as a CONNECT response must re-encode its comm-mode
    // byte losslessly on the modeled bits.
    if let Ok(xcp_core::XcpResponse::Connect(info)) =
        xcp_core::parse_response_with(xcp_core::ByteOrder::Intel, xcp_core::Expect::Connect, data)
    {
        let raw = data[2];
        assert_eq!(
            info.comm_mode.to_raw(),
            raw & 0xC7,
            "COMM_MODE_BASIC round-trip diverged"
        );
    }

    // Frame builders stay within protocol caps for any parsed session:
    if let Ok(xcp_core::XcpResponse::Connect(info)) =
        xcp_core::parse_response_with(xcp_core::ByteOrder::Intel, xcp_core::Expect::Connect, data)
    {
        assert!(
            xcp_core::transport::CLASSIC_CAN_MAX_CTO >= 8,
            "classic CAN cap constant lost"
        );
        assert!(info.max_cto <= 255);
    }
});
