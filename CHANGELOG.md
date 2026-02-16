# Changelog

All notable changes to the AIVory Monitor Rust Agent will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-02-16

### Added
- Automatic panic hook installation for panic capture
- Backtrace capture with full stack frame information
- Manual error capture via `aivory_monitor::capture_error()`
- Error capture with additional context
- User and custom context enrichment
- Async WebSocket transport via tokio and tokio-tungstenite
- Configurable sampling rate and capture depth
- Environment variable and programmatic configuration
- Thread-safe global agent with once_cell and parking_lot
- Graceful shutdown support
