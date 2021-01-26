use mio::{Waker, Token};
use std::sync::Arc;
use crossbeam::queue::SegQueue;


/// The update queue that signifies the changes that should
/// be made to the event loop.
pub type UpdatesQueue = Arc<SegQueue<EventUpdate>>;

/// The update that should be applied to the event loop.
pub enum EventUpdate {
    PauseReading(Token),
    PauseWriting(Token),

    ResumeReading(Token),
    ResumeWriting(Token),
}

#[derive(Clone)]
pub struct Transport {
    internal: UpdatesQueue,
    waker: Arc<Waker>
}

impl Transport {
    pub fn from_queue_and_waker(
        internal: UpdatesQueue,
        waker: Arc<Waker>
    ) -> Self {
        Self { internal, waker }
    }
}