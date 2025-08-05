# Changelog

## 0.2.3 (2026-03-22)

- Fix README section ordering

## 0.2.2 (2026-03-22)

- Fix CHANGELOG compliance

## 0.2.1 (2026-03-17)

- Add crate-level documentation with usage examples

## 0.2.0 (2026-03-17)

- Add `clear_event(event_name)` to remove all listeners for a specific event
- Add `set_error_handler()` for catching listener panics during emission
- Panics in listeners are caught via `std::panic::catch_unwind` when an error handler is set

## 0.1.5 (2026-03-17)

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
