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
pub struct EventLoopHandle {
    internal: UpdatesQueue,
    waker: Arc<Waker>
}

impl EventLoopHandle {
    pub fn from_queue_and_waker(
        internal: UpdatesQueue,
        waker: Arc<Waker>
    ) -> Self {
        Self { internal, waker }
    }

    pub fn pause_reading(&self, token: Token) {
        let update = EventUpdate::PauseReading(token);
        self.internal.push(update);

        self.waker.wake()
            .expect("Failed to wake event loop");
    }

    pub fn pause_writing(&self, token: Token) {
        let update = EventUpdate::PauseWriting(token);
        self.internal.push(update);

        self.waker.wake()
            .expect("Failed to wake event loop");
    }

    pub fn resume_reading(&self, token: Token) {
        let update = EventUpdate::ResumeReading(token);
        self.internal.push(update);

        self.waker.wake()
            .expect("Failed to wake event loop");
    }

    pub fn resume_writing(&self, token: Token) {
        let update = EventUpdate::ResumeWriting(token);
        self.internal.push(update);

        self.waker.wake()
            .expect("Failed to wake event loop");
    }
}