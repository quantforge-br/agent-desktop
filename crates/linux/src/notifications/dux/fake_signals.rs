//! The notification service's signals as the fake dux-shell raises them.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use crate::notifications::signal::{Held, Signal, SignalSource};

/// The notification service's signals, as the fake shell raised them.
pub(super) struct DuxSignals {
    pub(super) heard: Rc<RefCell<Vec<Signal>>>,
    pub(super) fails: bool,
}

struct Heard {
    heard: Rc<RefCell<Vec<Signal>>>,
    from: usize,
}

impl Held for Heard {
    fn collect(self: Box<Self>, _: Duration) -> Vec<Signal> {
        self.heard.borrow()[self.from..].to_vec()
    }
}

impl SignalSource for DuxSignals {
    fn subscribe(&self, _: Deadline) -> Result<Box<dyn Held>, AdapterError> {
        if self.fails {
            return Err(AdapterError::new(ErrorCode::Internal, "no session bus"));
        }
        Ok(Box::new(Heard {
            heard: self.heard.clone(),
            from: self.heard.borrow().len(),
        }))
    }
}
