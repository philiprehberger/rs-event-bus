# rs-event-bus

Thread-safe event bus with typed listeners for Rust. Supports persistent and one-shot listeners, listener removal by ID, and safe concurrent access from multiple threads.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
philiprehberger-event-bus = "0.1"
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
| `bus.remove_all_listeners(event)` | Remove listeners for one event (`Some`) or all (`None`) |
| `bus.max_listeners()` | Get the max listeners setting |
| `bus.set_max_listeners(max)` | Set the max listeners limit |
| `ListenerId` | Opaque ID returned by `on`/`once`, used with `off` |

## License

MIT
