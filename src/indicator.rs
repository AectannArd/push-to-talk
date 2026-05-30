//! Console recording indicator: terminal title + visible marker.
//! Used alongside the tray icon for foreground sessions.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct IndicatorHandle {
    visible: Arc<AtomicBool>,
}

impl IndicatorHandle {
    pub fn set_visible(&self, v: bool) {
        self.visible.store(v, Ordering::Relaxed);
        if v {
            eprintln!("\n🔴 ● RECORDING ● 🔴");
            eprint!("\x1b]0;🔴 REC ● push-to-talk\x07");
        } else {
            eprint!("\x1b]0;push-to-talk\x07");
        }
    }
}

impl Drop for IndicatorHandle {
    fn drop(&mut self) {
        self.set_visible(false);
    }
}

pub fn spawn() -> IndicatorHandle {
    IndicatorHandle {
        visible: Arc::new(AtomicBool::new(false)),
    }
}
