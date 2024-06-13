use std::any::Any;

use crate::event_emitter::Event;

#[derive(Debug, Clone)]
pub struct ClockSubscriberEvent {
    current_ts: i64,
}

impl ClockSubscriberEvent {
    pub fn new(current_ts: i64) -> Self {
        Self { current_ts }
    }

    pub fn get_current_ts(&self) -> i64 {
        self.current_ts
    }
}

impl Event for ClockSubscriberEvent {
    fn box_clone(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
