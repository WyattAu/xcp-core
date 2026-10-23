# xcp-core
[![Estate integration](https://github.com/WyattAu/estate-integration/actions/workflows/ci.yml/badge.svg)](https://github.com/WyattAu/estate-integration/actions/workflows/ci.yml)

XCP on CAN: Universal Measurement and Calibration Protocol: commands,
responses, DAQ lists.

**Layer:** L1: substrate · **Estate deps:** `can-core` (L0) · **Runtime
deps:** `can-core`, `thiserror` · `no_std` + `alloc` (CI exercises
`thumbv7em-none-eabihf`).

## What's here

| Module | Contents |
|---|---|
| `command` | [`XcpCommand`](https://docs.rs/xcp-core/latest/xcp_core/enum.XcpCommand.html): the complete ASAM XCP part 1 CTO command table (CONNECT `0xFF` through `TIME_CORRELATION_PROPERTIES` `0xC6`, versions 1.0–1.5), with `code()` / total `from_code()` / spec `name()` |
| `response` | `parse_response` / `parse_response_with`: total CTO response parser: RES (`0xFF`) decoded per the outstanding command (`Expect`), ERR (`0xFE`) mapped onto the typed error taxonomy, EV (`0xFD`) and SERV (`0xFC`) preserved |
| `error` | `XcpError`: the exhaustive ASAM error-code table (`0x20` command unknown, `0x31` generic, `0x33` resource temporarily not accessible, …) plus transport-local structural errors |
| `frame` | Master-side builders: `connect`, `set_mta`, `upload` / `download` (SET_MTA + block-mode-chunked DOWNLOAD sequences), `short_upload` / `short_download`, `set_cal_page` / `get_cal_page` |
| `daq` | `DaqList`: periodic-measurement configuration: event channel, prescaler, priority, timestamp/PID-off/stim mode bits, ODT entries; frames the full SET_DAQ_LIST_MODE + SET_DAQ_PTR/WRITE_DAQ sequence and the START_STOP run-control frames |
| `byteorder` | `ByteOrder`: Intel (little-endian) and Motorola (big-endian) addressing per the CONNECT `COMM_MODE_BASIC` bit, `AddressGranularity` |
| `resource` | `ResourceMode` (CAL/PAG, DAQ, STIM, PGM, DBG bit set) and `SessionStatus` (GET_STATUS session byte) |
| `transport` | `XcpOnCan`: the XCP part 3 bridge: packets ↔ `can-core` frames, command/response/DAQ CAN-ID model, classic-CAN 8-byte cap |

## Guarantees

- **Total parser**: `parse_response` never panics: bounds are checked at
  every access (`first_chunk`/`split_first`, deny-level lints on the lib
  target), and every input resolves to a typed response or typed error.
  Covered by the `xcp_parse` cargo-fuzz target and 30 s CI fuzz smoke.
- **Exhaustive protocol tables**: every command and error code ASAM XCP
  1.0–1.5 defines is a typed variant; `from_code` is total; the reserved
  holes (`0xC3`, `0xC4`, …) are `None` by construction.
- **Byte-order fidelity**: addresses, `MAX_DTO`, DAQ list numbers, event
  channels, and timestamps encode/decode in the slave's advertised order;
  verified by 500-case property tests in both orders.
- **Block-mode chunking**: `download()` splits arbitrary-length writes
  into `DOWNLOAD` (≤ 253 B) + `DOWNLOAD_NEXT` (≤ 254 B) frames exactly as
  the MTA advances.

## Example

```rust
use xcp_core::{connect, parse_response_with, ByteOrder, Expect, ResourceMode};

// CONNECT, normal mode: first frame of every session:
assert_eq!(connect(ResourceMode::CONNECT_NORMAL), [0xFF, 0x00]);

// A real-world CONNECT response: DAQ+STIM+PGM, Intel byte order, 8-byte CTOs.
let reply = [0xFF, 0x1C, 0xC0, 0x08, 0x08, 0x00, 0x01, 0x01];
let res = parse_response_with(ByteOrder::Intel, Expect::Connect, &reply).unwrap();
match res {
    xcp_core::XcpResponse::Connect(info) => {
        assert!(info.resource.daq() && info.resource.pgm());
        assert_eq!(info.max_cto, 8);
        assert_eq!(info.comm_mode.byte_order, ByteOrder::Intel);
    }
    _ => unreachable!(),
}
```

Calibration writes ride the MTA; measurement rides DAQ lists:

```rust
use xcp_core::{download, DaqEntry, DaqList};

// SET_MTA + DOWNLOAD, Intel addressing:
let frames = download(0x4000_0000, &[0xDE, 0xAD, 0xBE, 0xEF]);
assert_eq!(frames.len(), 14);

let mut list = DaqList::new(0, 0x0010);
list.push_entry(DaqEntry { bit_offset: 0, size: 2, address: 0x3200_0100, address_extension: 0 });
let cfg = list.configure_frames(xcp_core::ByteOrder::Intel); // 3 × 8-byte frames
assert_eq!(list.start_frame(), [0xDE, 0x01, 0x00, 0x00]);
```

## Tests

`cargo test` runs unit + integration suites, doctests, and an 8-property
500-case proptest battery (address/data framing round-trips in both byte
orders, CONNECT encode/decode, parser totality, error-code bijection,
DAQ configuration round-trips). `cargo fuzz run xcp_parse` covers the
response parser; miri runs the lib suite; `thumbv7em-none-eabihf` is
checked in CI.

## License

MIT OR Apache-2.0: see LICENSE-MIT and LICENSE-APACHE.
