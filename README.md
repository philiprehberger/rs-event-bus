# rs-event-bus

[![CI](https://github.com/philiprehberger/rs-event-bus/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-event-bus/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-event-bus.svg)](https://crates.io/crates/philiprehberger-event-bus)
[![License](https://img.shields.io/github/license/philiprehberger/rs-event-bus)](LICENSE)
[![Sponsor](https://img.shields.io/badge/sponsor-GitHub%20Sponsors-ec6cb9)](https://github.com/sponsors/philiprehberger)

Thread-safe event bus with typed listeners for Rust

## Installation

```toml
[dependencies]
philiprehberger-event-bus = "0.2.3"
```

## Usage

```rust
use philiprehberger_event_bus::EventBus;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

let bus = EventBus::new();

let counter = Arc::new(AtomicUsize::new(0));
let c = counter.clone();
bus.on("greet", move || {
    c.fetch_add(1, Ordering::SeqCst);
});

bus.emit("greet"); // counter is now 1
bus.emit("greet"); // counter is now 2

// One-shot listener fires only once
let once_counter = Arc::new(AtomicUsize::new(0));
let c = once_counter.clone();
bus.once("init", move || {
    c.fetch_add(1, Ordering::SeqCst);
});

bus.emit("init"); // once_counter is 1
bus.emit("init"); // once_counter is still 1
```

### Introspection

Query the bus for registered events and listener counts:

```rust
use philiprehberger_event_bus::EventBus;

let bus = EventBus::new();
bus.on("click", || {});
bus.on("click", || {});
bus.on("hover", || {});

assert_eq!(bus.listener_count("click"), 2);
assert_eq!(bus.event_names(), vec!["click", "hover"]);

bus.clear_event("click");
assert_eq!(bus.listener_count("click"), 0);
```

### Error Handling

By default, a panic inside a listener propagates normally. You can install an
error handler to catch and log panics without crashing the emitter:

```rust
use philiprehberger_event_bus::EventBus;

let bus = EventBus::new();
bus.set_error_handler(|event, message| {
    eprintln!("Listener panic on '{event}': {message}");
});

bus.on("task", || {
    panic!("something went wrong");
});

// The panic is caught; emit still returns the total listener count.
let count = bus.emit("task"); // prints warning, returns 1
```

When an error handler is set, `emit` uses `std::panic::catch_unwind` around
each callback. Panics are reported to the handler and execution continues with
the remaining listeners.

## API

| Item | Description |
|------|-------------|
| `EventBus::new()` | Create a new event bus with default max listeners (10) |
| `bus.on(event, callback)` | Register a persistent listener; returns `ListenerId` |
| `bus.once(event, callback)` | Register a one-shot listener; returns `ListenerId` |
| `bus.off(id)` | Remove a listener by ID; returns `true` if found |
| `bus.emit(event)` | Emit an event, calling all listeners; returns count called |
| `bus.listener_count(event)` | Return number of listeners for an event |
| `bus.event_names()` | Return sorted list of event names with listeners |
| `bus.clear_event(event)` | Remove all listeners for a specific event |
| `bus.remove_all_listeners(event)` | Remove listeners for one event (`Some`) or all (`None`) |
| `bus.max_listeners()` | Get the max listeners setting |
| `bus.set_max_listeners(max)` | Set the max listeners limit |
| `bus.set_error_handler(handler)` | Set a handler called when a listener panics during emission |
| `ListenerId` | Opaque ID returned by `on`/`once`, used with `off` |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## License

MIT
