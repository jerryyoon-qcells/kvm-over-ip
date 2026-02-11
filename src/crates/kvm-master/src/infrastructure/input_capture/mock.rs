//! Mock input source for unit testing.
//!
//! Allows tests to inject synthetic [`RawInputEvent`]s without requiring
//! a running Windows message loop or OS hooks.

use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};

use super::{CaptureError, InputSource, RawInputEvent};

/// A mock implementation of [`InputSource`] that allows tests to inject events.
pub struct MockInputSource {
    sender: Arc<Mutex<Option<Sender<RawInputEvent>>>>,
    suppress_count: Arc<Mutex<u32>>,
}

impl MockInputSource {
    /// Creates a new mock input source.
    pub fn new() -> Self {
        Self {
            sender: Arc::new(Mutex::new(None)),
            suppress_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Injects a synthetic event, as if captured from hardware.
    ///
    /// Panics if `start()` has not been called or if `stop()` has been called.
    pub fn inject_event(&self, event: RawInputEvent) {
        let guard = self.sender.lock().expect("lock poisoned");
        if let Some(ref sender) = *guard {
            sender
                .send(event)
                .expect("receiver has been dropped; call start() first");
        } else {
            panic!("MockInputSource::inject_event called before start()");
        }
    }

    /// Returns the number of times [`suppress_current_event`] was called.
    pub fn suppress_count(&self) -> u32 {
        *self.suppress_count.lock().expect("lock poisoned")
    }
}

impl Default for MockInputSource {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSource for MockInputSource {
    fn start(&self) -> Result<mpsc::Receiver<RawInputEvent>, CaptureError> {
        let (tx, rx) = mpsc::channel();
        *self.sender.lock().expect("lock poisoned") = Some(tx);
        Ok(rx)
    }

    fn stop(&self) {
        // Drop the sender to close the channel
        *self.sender.lock().expect("lock poisoned") = None;
    }

    fn suppress_current_event(&self) {
        let mut count = self.suppress_count.lock().expect("lock poisoned");
        *count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::input_capture::MouseButton;

    #[test]
    fn test_mock_input_source_starts_and_receives_events() {
        // Arrange
        let source = MockInputSource::new();
        let rx = source.start().expect("start should succeed");

        // Act
        source.inject_event(RawInputEvent::KeyDown {
            vk_code: 0x41,
            scan_code: 0x1E,
            time_ms: 0,
            is_extended: false,
        });

        // Assert
        let event = rx.recv().expect("should receive event");
        assert!(matches!(event, RawInputEvent::KeyDown { vk_code: 0x41, .. }));
    }

    #[test]
    fn test_mock_input_source_stop_closes_channel() {
        // Arrange
        let source = MockInputSource::new();
        let rx = source.start().expect("start should succeed");

        // Act
        source.stop();

        // Assert – channel should be disconnected
        let result = rx.recv();
        assert!(result.is_err(), "channel should be closed after stop()");
    }

    #[test]
    fn test_mock_input_source_tracks_suppress_count() {
        // Arrange
        let source = MockInputSource::new();
        source.start().expect("start should succeed");

        // Act
        source.suppress_current_event();
        source.suppress_current_event();
        source.suppress_current_event();

        // Assert
        assert_eq!(source.suppress_count(), 3);
    }

    #[test]
    fn test_mock_input_source_inject_multiple_event_types() {
        // Arrange
        let source = MockInputSource::new();
        let rx = source.start().expect("start should succeed");

        // Act
        source.inject_event(RawInputEvent::MouseMove { x: 100, y: 200, time_ms: 1 });
        source.inject_event(RawInputEvent::MouseButtonDown {
            button: MouseButton::Left,
            x: 100,
            y: 200,
            time_ms: 2,
        });
        source.inject_event(RawInputEvent::MouseWheel {
            delta: 120,
            x: 100,
            y: 200,
            time_ms: 3,
        });

        // Assert
        assert!(matches!(rx.recv().unwrap(), RawInputEvent::MouseMove { x: 100, .. }));
        assert!(matches!(
            rx.recv().unwrap(),
            RawInputEvent::MouseButtonDown { button: MouseButton::Left, .. }
        ));
        assert!(matches!(rx.recv().unwrap(), RawInputEvent::MouseWheel { delta: 120, .. }));
    }
}
