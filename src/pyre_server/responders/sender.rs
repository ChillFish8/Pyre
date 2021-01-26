use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use crossbeam::channel::{Sender, Receiver, bounded, TryRecvError};

use bytes::Bytes;

use crate::pyre_server::responders::Payload;
use crate::pyre_server::transport::EventLoopHandle;
use mio::Token;


/// The callable class that handling communication back to the server protocol.
#[pyclass]
pub struct DataSender {
    token: Token,

    event_loop: EventLoopHandle,

    tx: Sender<Payload>,
}

impl DataSender {
    /// Create a new handler with the given sender.
    pub fn new(
        token: Token,
        event_loop: EventLoopHandle,
        tx: Sender<Payload>,
    ) -> Self {
        Self { tx, event_loop, token }
    }
}

#[pymethods]
impl DataSender {
    /// Invoked by python passing more_body which represents if there
    /// is any more body to expect or not, and the body itself.
    #[call]
    fn __call__(&self, more_body: bool, body: Vec<u8>) -> PyResult<()> {
        self.event_loop.resume_writing(self.token);

        if let Err(e) = self.tx.send((more_body, body)) {
            return Err(PyRuntimeError::new_err(format!("{:?}", e)))
        }

        Ok(())
    }
}


pub struct SenderHandler {
    /// The sender half for sending body chunks.
    sender_tx: Sender<Payload>,

    /// The receiver half for sending body chunks.
    sender_rx: Receiver<Payload>,

    token: Token,

    event_loop: EventLoopHandle,
}

impl SenderHandler {
    pub fn new(token: Token, event_loop: EventLoopHandle) -> Self {
        let (tx, rx) = bounded(10);
        Self {
            sender_tx: tx,
            sender_rx: rx,
            token,
            event_loop,
        }
    }

    pub fn make_handle(&self) -> DataSender {
        DataSender::new(
            self.token,
            self.event_loop.clone(),
            self.sender_tx.clone()
        )
    }

    pub fn recv(&self) -> Result<Payload, TryRecvError> {
        self.sender_rx.try_recv()
    }
}