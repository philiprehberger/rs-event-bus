//! Thread-safe event bus with typed listeners.
//!
//! # Example
//!
//! ```rust
//! use philiprehberger_event_bus::EventBus;
//!
//! let bus = EventBus::new();
//! bus.on("greet", || {
//!     println!("Hello!");
//! });
//! bus.emit("greet");
//! ```

use std::collections::HashMap;
use std::fmt;
use std::panic;
use std::sync::{Arc, RwLock};

/// Opaque identifier for a registered listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(u64);

struct Listener {
    id: ListenerId,
    callback: Arc<dyn Fn() + Send + Sync>,
    once: bool,
}

type ErrorHandler = Arc<dyn Fn(&str, String) + Send + Sync>;

struct Inner {
    listeners: HashMap<String, Vec<Listener>>,
    max_listeners: usize,
    next_id: u64,
    error_handler: Option<ErrorHandler>,
}

impl Inner {
    fn allocate_id(&mut self) -> ListenerId {
        let id = ListenerId(self.next_id);
        self.next_id += 1;
        id
    }
}

/// A thread-safe event bus that supports persistent and one-shot listeners.
///
/// Cloning an `EventBus` produces a handle that shares the same underlying state,
/// so listeners registered on one handle are visible to all clones.
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<RwLock<Inner>>,
}

impl EventBus {
    /// Create a new event bus with a default max-listeners limit of 10.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                listeners: HashMap::new(),
                max_listeners: 10,
                next_id: 0,
                error_handler: None,
            })),
        }
    }

    /// Register a persistent listener for `event`. Returns a [`ListenerId`] that
    /// can be passed to [`off`](Self::off) to remove it later.
    pub fn on(
        &self,
        event: impl Into<String>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> ListenerId {
        self.add_listener(event.into(), callback, false)
    }

    /// Register a one-shot listener for `event`. The listener is automatically
    /// removed after it fires once.
    pub fn once(
        &self,
        event: impl Into<String>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> ListenerId {
        self.add_listener(event.into(), callback, true)
    }

    fn add_listener(
        &self,
        event: String,
        callback: impl Fn() + Send + Sync + 'static,
        once: bool,
    ) -> ListenerId {
        let mut inner = self.inner.write().unwrap();
        let id = inner.allocate_id();
        let listener = Listener {
            id,
            callback: Arc::new(callback),
            once,
        };
        inner.listeners.entry(event).or_default().push(listener);
        id
    }

    /// Remove the listener identified by `id` from any event. Returns `true` if
    /// the listener was found and removed.
    pub fn off(&self, id: ListenerId) -> bool {
        let mut inner = self.inner.write().unwrap();
        for listeners in inner.listeners.values_mut() {
            if let Some(pos) = listeners.iter().position(|l| l.id == id) {
                listeners.remove(pos);
                return true;
            }
        }
        false
    }

    /// Emit an event, calling every listener registered for it. One-shot
    /// listeners are removed after being called. Returns the number of
    /// listeners that were invoked.
    ///
    /// Callbacks are invoked outside the lock so they may safely call back
    /// into the bus without deadlocking. If a listener panics and an error
    /// handler has been set via [`set_error_handler`](Self::set_error_handler),
    /// the panic is caught and reported to the handler instead of propagating.
    pub fn emit(&self, event: &str) -> usize {
        // Collect callbacks and error handler under the write lock.
        let callbacks: Vec<Arc<dyn Fn() + Send + Sync>>;
        let error_handler: Option<ErrorHandler>;
        {
            let mut inner = self.inner.write().unwrap();
            error_handler = inner.error_handler.clone();

            let Some(listeners) = inner.listeners.get_mut(event) else {
                return 0;
            };

            callbacks = listeners.iter().map(|l| Arc::clone(&l.callback)).collect();

            // Remove one-shot listeners.
            listeners.retain(|l| !l.once);

            // Clean up empty entries.
            if listeners.is_empty() {
                inner.listeners.remove(event);
            }
        }

        let count = callbacks.len();
        for cb in &callbacks {
            if error_handler.is_some() {
                let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    cb();
                }));
                if let Err(panic_value) = result {
                    let message = if let Some(s) = panic_value.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else if let Some(s) = panic_value.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    if let Some(ref handler) = error_handler {
                        handler(event, message);
                    }
                }
            } else {
                cb();
            }
        }
        count
    }

    /// Return the number of listeners registered for `event`.
    pub fn listener_count(&self, event: &str) -> usize {
        let inner = self.inner.read().unwrap();
        inner
            .listeners
            .get(event)
            .map_or(0, |listeners| listeners.len())
    }

    /// Return a sorted list of event names that have at least one listener.
    pub fn event_names(&self) -> Vec<String> {
        let inner = self.inner.read().unwrap();
        let mut names: Vec<String> = inner
            .listeners
            .keys()
            .filter(|k| {
                inner
                    .listeners
                    .get(k.as_str())
                    .is_some_and(|v| !v.is_empty())
            })
            .cloned()
            .collect();
        names.sort();
        names
    }

    /// Removes all listeners for a specific event.
    pub fn clear_event(&self, event_name: &str) {
        let mut inner = self.inner.write().unwrap();
        inner.listeners.remove(event_name);
    }

    /// Set a handler called when a listener panics during emission.
    ///
    /// When set, panics inside listener callbacks are caught via
    /// [`std::panic::catch_unwind`] and reported to this handler with the
    /// event name and a string description of the panic. Without an error
    /// handler, panics propagate normally.
    pub fn set_error_handler<F>(&self, handler: F)
    where
        F: Fn(&str, String) + Send + Sync + 'static,
    {
        let mut inner = self.inner.write().unwrap();
        inner.error_handler = Some(Arc::new(handler));
    }

    /// Remove listeners. If `event` is `Some`, only listeners for that event are
    /// removed. If `None`, all listeners for every event are removed.
    pub fn remove_all_listeners(&self, event: Option<&str>) {
        let mut inner = self.inner.write().unwrap();
        match event {
            Some(name) => {
                inner.listeners.remove(name);
            }
            None => {
                inner.listeners.clear();
            }
        }
    }

    /// Return the current max-listeners setting.
    pub fn max_listeners(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.max_listeners
    }

    /// Set the max-listeners limit.
    pub fn set_max_listeners(&self, max: usize) {
        let mut inner = self.inner.write().unwrap();
        inner.max_listeners = max;
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.read().unwrap();
        let mut map = f.debug_map();
        let mut names: Vec<&String> = inner.listeners.keys().collect();
        names.sort();
        for name in names {
            if let Some(listeners) = inner.listeners.get(name) {
                map.entry(&name, &listeners.len());
            }
        }
        map.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn on_and_emit() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        bus.on("test", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        bus.emit("test");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn once_fires_once() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        bus.once("test", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        bus.emit("test");
        bus.emit("test");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn off_removes() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        let id = bus.on("test", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        assert!(bus.off(id));
        bus.emit("test");
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn emit_returns_count() {
        let bus = EventBus::new();
        bus.on("test", || {});
        bus.on("test", || {});
        assert_eq!(bus.emit("test"), 2);
    }

    #[test]
    fn remove_all_for_event() {
        let bus = EventBus::new();
        let counter_a = Arc::new(AtomicUsize::new(0));
        let counter_b = Arc::new(AtomicUsize::new(0));

        let ca = counter_a.clone();
        bus.on("a", move || {
            ca.fetch_add(1, Ordering::SeqCst);
        });
        let cb = counter_b.clone();
        bus.on("b", move || {
            cb.fetch_add(1, Ordering::SeqCst);
        });

        bus.remove_all_listeners(Some("a"));
        bus.emit("a");
        bus.emit("b");

        assert_eq!(counter_a.load(Ordering::SeqCst), 0);
        assert_eq!(counter_b.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn remove_all_global() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = counter.clone();
        bus.on("a", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let c = counter.clone();
        bus.on("b", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        bus.remove_all_listeners(None);
        bus.emit("a");
        bus.emit("b");

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn listener_count() {
        let bus = EventBus::new();
        let id1 = bus.on("test", || {});
        bus.on("test", || {});
        bus.on("test", || {});
        assert_eq!(bus.listener_count("test"), 3);

        bus.off(id1);
        assert_eq!(bus.listener_count("test"), 2);
    }

    #[test]
    fn event_names() {
        let bus = EventBus::new();
        bus.on("b", || {});
        bus.on("a", || {});
        assert_eq!(bus.event_names(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn max_listeners() {
        let bus = EventBus::new();
        assert_eq!(bus.max_listeners(), 10);
        bus.set_max_listeners(50);
        assert_eq!(bus.max_listeners(), 50);
    }

    #[test]
    fn thread_safety() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        bus.on("ping", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        let mut handles = vec![];
        for _ in 0..10 {
            let b = bus.clone();
            handles.push(std::thread::spawn(move || {
                b.emit("ping");
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn clone_shares_state() {
        let bus = EventBus::new();
        let clone = bus.clone();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        bus.on("shared", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        clone.emit("shared");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn emit_no_listeners() {
        let bus = EventBus::new();
        assert_eq!(bus.emit("nonexistent"), 0);
    }

    #[test]
    fn clear_event_removes_all_for_event() {
        let bus = EventBus::new();
        let counter_a = Arc::new(AtomicUsize::new(0));
        let counter_b = Arc::new(AtomicUsize::new(0));

        let ca = counter_a.clone();
        bus.on("a", move || {
            ca.fetch_add(1, Ordering::SeqCst);
        });
        bus.on("a", move || {});
        let cb = counter_b.clone();
        bus.on("b", move || {
            cb.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(bus.listener_count("a"), 2);
        bus.clear_event("a");
        assert_eq!(bus.listener_count("a"), 0);
        bus.emit("a");
        bus.emit("b");

        assert_eq!(counter_a.load(Ordering::SeqCst), 0);
        assert_eq!(counter_b.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn clear_event_nonexistent_is_noop() {
        let bus = EventBus::new();
        bus.clear_event("nope"); // should not panic
    }

    #[test]
    fn error_handler_catches_panic() {
        let bus = EventBus::new();
        let caught = Arc::new(RwLock::new(Vec::<(String, String)>::new()));
        let c = caught.clone();
        bus.set_error_handler(move |event, msg| {
            c.write().unwrap().push((event.to_string(), msg));
        });

        bus.on("boom", || {
            panic!("listener exploded");
        });
        let count = bus.emit("boom");
        assert_eq!(count, 1);

        let errors = caught.read().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, "boom");
        assert!(errors[0].1.contains("listener exploded"));
    }

    #[test]
    fn error_handler_does_not_stop_other_listeners() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let caught = Arc::new(AtomicUsize::new(0));

        let cc = caught.clone();
        bus.set_error_handler(move |_event, _msg| {
            cc.fetch_add(1, Ordering::SeqCst);
        });

        let c = counter.clone();
        bus.on("mixed", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        bus.on("mixed", || {
            panic!("bad listener");
        });
        let c = counter.clone();
        bus.on("mixed", move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        let count = bus.emit("mixed");
        assert_eq!(count, 3);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(caught.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn without_error_handler_panic_propagates() {
        let bus = EventBus::new();
        bus.on("boom", || {
            panic!("no handler");
        });
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            bus.emit("boom");
        }));
        assert!(result.is_err());
    }
}
