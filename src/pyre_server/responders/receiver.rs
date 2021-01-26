use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use crossbeam::channel::{Sender, Receiver, bounded, TrySendError};
use bytes::Bytes;
use mio::Token;

use crate::pyre_server::responders::Payload;
use crate::pyre_server::transport::EventLoopHandle;


/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataReceiver {
    token: Token,
    event_loop: EventLoopHandle,
    rx: Receiver<Payload>,
}

impl DataReceiver {
    /// Create a new handler with the given sender.
    pub fn new(
        token: Token,
        event_loop: EventLoopHandle,
        rx: Receiver<Payload>,
    ) -> Self {
        Self { rx, event_loop, token }
    }
}

#[pymethods]
impl DataReceiver {
    /// Invoked by python passing more_body which represents if there
    /// is any more body to expect or not, and the body itself.
    #[call]
    fn __call__(&self) -> PyResult<()> {
        self.event_loop.resume_reading(self.token);

        Ok(())
    }
}


pub struct ReceiverHandler {
    /// The sender half for sending body chunks.
    receiver_tx: Sender<Payload>,

    /// The receiver half for sending body chunks.
    receiver_rx: Receiver<Payload>,

    token: Token,

    event_loop: EventLoopHandle,
}

impl ReceiverHandler {
    pub fn new(
        token: Token,
        event_loop: EventLoopHandle
    ) -> Self {
        let (tx, rx) = bounded(10);
        Self {
            receiver_tx: tx,
            receiver_rx: rx,
            token,
            event_loop,
        }
    }

    pub fn make_handle(&self) -> DataReceiver{
        DataReceiver::new(
            self.token,
            self.event_loop.clone(),
            self.receiver_rx.clone()
        )
    }

    pub fn send(&self, data: Payload) -> Result<(), TrySendError<Payload>> {
        self.receiver_tx.try_send(data)
    }
}