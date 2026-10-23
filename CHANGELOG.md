# Changelog

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning:
[SemVer](https://semver.org/).

## [0.1.0]: 2026-10-04

Initial release: L1 substrate over `can-core` L0, `no_std` + `alloc`.

### Added

- **Command table**: `XcpCommand`: every ASAM XCP part 1 CTO command code
  (1.0–1.5): standard (CONNECT `0xFF` … USER_CMD `0xF1`), calibration
  (DOWNLOAD `0xF0` … MODIFY_BITS `0xEC`), page switching (SET_CAL_PAGE
  `0xEB` … COPY_CAL_PAGE `0xE4`), DAQ (CLEAR_DAQ_LIST `0xE3` …
  ALLOC_ODT_ENTRY `0xD3`), flash programming (PROGRAM_START `0xD2` …
  PROGRAM_VERIFY `0xC8`), and the 1.1–1.5 extensions (GET_VERSION `0xC0`,
  WRITE_DAQ_MULTIPLE `0xC7`, DTO_CTR_PROPERTIES `0xC5`,
  TIME_CORRELATION_PROPERTIES `0xC6`, packed-mode `0xC1`/`0xC2`). Total
  `from_code`, spec `name()`.
- **Response parsing**: `parse_response` / `parse_response_with`: total
  CTO parser over RES/ERR/EV/SERV; typed CONNECT (resource,
  `COMM_MODE_BASIC` incl. byte order + address granularity, MAX_CTO/MAX_DTO,
  versions), GET_STATUS (session status, protection status, session config
  ID), GET_VERSION, GET_ID, UPLOAD, START_STOP_DAQ_LIST, GET_DAQ_CLOCK, EV
  and SERV packets; documented length-based inference heuristic when no
  outstanding command is given (`Expect` disambiguates).
- **Error taxonomy**: `XcpError`: the exhaustive ASAM slave ERR table
  (`0x00`–`0x33` incl. `ERR_RESOURCE_TEMPORARY_NOT_ACCESSIBLE`) plus
  `BufferTooShort`/`UnknownPid` structural errors; `slave_code()`
  round-trip; SYNCH's ERR `0x00` success semantics documented.
- **Frame builders**: `connect`, `disconnect`, `get_status`, `synch`,
  `get_version`, `get_id`, `get_comm_mode_info`, `set_mta`,
  `upload`/`upload_with`, `upload_frame`, `short_upload`,
  `download`/`download_with` (block-mode chunking via DOWNLOAD +
  DOWNLOAD_NEXT), `download_frames`, `short_download`,
  `set_cal_page`/`get_cal_page`.
- **DAQ lists**: `DaqList`/`DaqEntry`/`DaqListMode`/`StartStopMode`:
  static-configuration frame sequence and start/stop/select/synch
  run-control frames.
- **Byte order**: `ByteOrder` (Intel/Motorola per XCP part 1
  `COMM_MODE_BASIC` bit 0) with typed `put_u16`/`put_u32`/`get_u16`/
  `get_u32`; `AddressGranularity`.
- **Resources**: `ResourceMode` (CAL/PAG, DAQ, STIM, PGM, DBG +
  CAL_PAGE_OR_DAQ composite, CONNECT_NORMAL/CONNECT_USER_DEFINED) and
  `SessionStatus`.
- **Transport**: `transport::XcpOnCan`: XCP part 3 on CAN: command/
  response/DAQ CAN-ID model, packet ↔ `can_core::CanFrame` framing,
  classic-CAN 8-byte cap enforcement.
- **Gates**: 8-property 500-case proptest battery, cargo-fuzz target
  `xcp_parse` with committed corpus, miri, embedded
  (`thumbv7em-none-eabihf`), tier-a shared CI pinned to
  engineering-standards `main`, 96.7 % line coverage.

[0.1.0]: https://github.com/WyattAu/xcp-core/releases/tag/v0.1.0
