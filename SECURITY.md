# Security Policy — xcp-core

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅        |

## Reporting a vulnerability

Report privately via [GitHub security advisories] for this repository, or
email **wyatt_au@protonmail.com**. Do **not** open a public issue for
security reports.

You will receive an acknowledgement within **72 hours**. Coordinated
disclosure: we ask for up to 90 days before public disclosure while a
patch ships.

[GitHub security advisories]: https://github.com/WyattAu/xcp-core/security/advisories/new

## Threat model

**I/O surface: untrusted input.** `xcp-core` parses CTO packets that
originate from CAN buses, SocketCAN sockets, log captures, and other
untrusted sources. The trust boundary is the *contents* of every `&[u8]`
handed to `parse_response` (and to the `transport` bridge). Callers
control where the bytes come from; xcp-core guarantees safety of the
*decode* regardless.

| ID | Threat | Mitigation |
|----|--------|------------|
| T1 | Malformed/truncated packets drive the parser out of bounds or into a panic (remote crash on hostile bus capture) | Totality by construction: bounds checked at every access (`first_chunk`/`split_first`-only; `unwrap`/`expect`/`panic`/`indexing_slicing` denied at the lib target), exhaustive `XcpError`, cargo-fuzz target `xcp_parse`, 30 s CI fuzz smoke, miri over the lib suite |
| T2 | Forged ERR/unknown PIDs mislead session state machines | Every ERR code maps to a typed variant with an exact `slave_code()` round-trip; unrecognised codes surface as `SlaveOther(raw)` instead of being coerced into a standard variant; unknown PIDs are rejected, never reinterpreted |
| T3 | Byte-order confusion silently corrupts calibration addresses/values | One `ByteOrder` parameter governs every multi-byte encode/decode; both orders are property-tested for round-trip fidelity at 500 cases per property |
| T4 | Oversized downloads overflow a slave's MAX_CTO | Block-mode chunking caps `DOWNLOAD` at 253 B and `DOWNLOAD_NEXT` at 254 B (protocol absolute max); the transport layer rejects > 8-byte packets for classic CAN instead of truncating |
| T5 | Dependency-supply compromise of the codec | Two runtime deps (`can-core`, `thiserror`), both `no_std`; `cargo-deny` advisories/bans gate in CI; `cargo-vet` supply-chain directory committed |
