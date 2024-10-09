# Changelog

## 0.2.0

- Add `clear_event(event_name)` to remove all listeners for a specific event
- Add `set_error_handler()` for catching listener panics during emission
- Panics in listeners are caught via `std::panic::catch_unwind` when an error handler is set

## 0.1.5

- Add readme, rust-version, documentation to Cargo.toml
- Add Development section to README
## 0.1.4 (2026-03-16)

- Update install snippet to use full version

## 0.1.3 (2026-03-16)

- Add README badges
- Synchronize version across Cargo.toml, README, and CHANGELOG

## 0.1.0 (2026-03-13)

- Initial release
- Thread-safe event bus with Arc-based cloning
- Support for persistent and one-shot listeners
- Listener removal by ID
- Max listeners configuration
