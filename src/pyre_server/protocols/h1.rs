// `mem::uninitialized` replaced with `mem::MaybeUninit`,
// can't upgrade yet
#![allow(deprecated)]

use crate::pyre_server::abc::{ProtocolBuffers, BaseTransport};
use crate::pyre_server::switch::{Switchable, SwitchStatus};
use crate::pyre_server::transport::EventLoopHandle;
use crate::pyre_server::py_callback::CallbackHandler;
use crate::pyre_server::responders::sender::SenderHandler;
use crate::pyre_server::responders::receiver::ReceiverHandler;

use pyo3::{PyResult, Python, Py};
use pyo3::types::PyBytes;
use pyo3::exceptions::PyRuntimeError;

use std::mem;
use std::sync::Arc;
use std::str;

use bytes::{BytesMut, Bytes};
use mio::Token;
use crossbeam::channel::{Sender, Receiver, unbounded};

use httparse::{Status, parse_chunk_size, Header, Request};
use http::version::Version;
use http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use std::error::Error;


/// The max headers allowed in a single request.
const MAX_HEADERS: usize = 100;



/// The protocol to add handling for the HTTP/1.x protocol.
pub struct H1Protocol {
    /// A possible Transport struct, this can be None if the protocol
    /// is not initialised before it starts handling interactions but this
    /// should never happen.
    event_loop: EventLoopHandle,

    /// The client token that identifies itself.
    token: Token,

    /// The python callback handler.
    callback: CallbackHandler,

    /// The sender half handler for ASGI callbacks.
    sender: SenderHandler,

    /// The receiver half handler for ASGI callbacks.
    receiver: ReceiverHandler,

    expected_content_length: usize,

    chunked_encoding: bool,
}

impl H1Protocol {
    /// Create a new H1Protocol instance.
    pub fn new(
        token: Token,
        callback: CallbackHandler,
        event_loop: EventLoopHandle,
    ) -> Self {
        let sender = SenderHandler::new(
            token,
            event_loop.clone()
        );
        let receiver = ReceiverHandler::new(
            token,
            event_loop.clone()
        );

        Self {
            token,
            event_loop,
            callback,
            sender,
            receiver,

            expected_content_length: 0,
            chunked_encoding: false,
        }
    }
}

impl H1Protocol {
    /// Called when the protocol is in charge of a new socket / handle.
    pub fn new_connection(&mut self) -> PyResult<()> {
        Ok(())
    }

    /// Called when the connection is lost from the protocol in order to
    /// properly reset state.
    pub fn lost_connection(&mut self) -> PyResult<()> {
        Ok(())
    }
}

impl ProtocolBuffers for H1Protocol {
    fn data_received(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        // This should be fine as it is guaranteed to be initialised
        // before we use it, just waiting for the ability to use
        // MaybeUninit, till then here we are.
        let mut headers: [Header<'_>; MAX_HEADERS] = unsafe {
            mem::uninitialized()
        };

        let body = buffer.clone();

        let mut request = Request::new(&mut headers);
        let status = match request.parse(&body) {
            Ok(status) => status,
            Err(e) => {
                eprintln!("{:?}", e);
                return Ok(())
            }
        };

        let len= if status.is_partial() {
            return Ok(())
        } else {
            status.unwrap()
        };

        let _ = buffer.split_to(len);

        self.on_request_parse(&mut request)?;

        Ok(())
    }

    fn fill_write_buffer(&mut self, buffer: &mut BytesMut) -> PyResult<()> {
        while let Ok((_more_body, buff)) = self.sender.recv() {
            buffer.extend(buff);
        }

        self.event_loop.pause_writing(self.token);

        Ok(())
    }

    fn eof_received(&mut self) -> PyResult<()> {
        self.event_loop.pause_reading(self.token);
        self.event_loop.pause_writing(self.token);
        Ok(())
    }
}

impl Switchable for H1Protocol {
    /// Determines what the protocol should be switched to if it is
    /// necessary called just after reading has completed to allow
    /// for upgrading.
    fn switch_protocol(&mut self) -> PyResult<SwitchStatus> {
        // ignore for now
        Ok(SwitchStatus::NoSwitch)
    }
}

impl H1Protocol {
    fn on_request_parse(&mut self, request: &mut Request) -> PyResult<()> {
        let method = request.method
            .expect("Value was None at complete parse");
        let path = request.path
            .expect("Value was None at complete parse");
        let version = request.version
            .expect("Value was None at complete parse");


        let headers_new = Python::with_gil(|py| {
            let mut parsed_vec = Vec::with_capacity(request.headers.len());
            for header in request.headers.iter() {
                self.check_header(&header);

                let converted: Py<PyBytes> = Py::from(PyBytes::new(py, header.value));
                parsed_vec.push((header.name, converted))
            }

            parsed_vec
        });


        let sender = self.sender.make_handle();
        let receiver = self.receiver.make_handle();
        self.callback.invoke((
            sender,
            receiver,
            headers_new,
            method,
            path,
            version,
        ))?;

        Ok(())
    }

    fn check_header(&mut self, header: &Header) {
        if header.name == CONTENT_LENGTH {
            self.expected_content_length = str::from_utf8(header.value)
                .map(|v| v.parse::<usize>().unwrap_or(0))
                .unwrap_or(0)
        } else if header.name == TRANSFER_ENCODING {
            let lowered = header.value.to_ascii_lowercase();
            self.chunked_encoding = str::from_utf8(lowered.as_ref())
                .map(|v| v.contains("chunked"))
                .unwrap_or(false)
        }
    }
}