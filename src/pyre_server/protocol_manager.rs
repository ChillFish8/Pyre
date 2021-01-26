
// internal imports
use crate::pyre_server::switch::{Switchable, SwitchStatus, SelectedProtocol};
use crate::pyre_server::py_callback::CallbackHandler;
use crate::pyre_server::transport::EventLoopHandle;
use crate::pyre_server::abc::{ProtocolBuffers, SocketCommunicator};

// protocols
use crate::pyre_server::protocols::h1;

use bytes::BytesMut;
use mio::Token;
use std::error::Error;
use pyo3::PyResult;


const MAX_BUFFER_LIMIT: usize = 256 * 1024;


/// A changeable protocol which does not modify the external API.
pub struct AutoProtocol {
    /// The client's identification token.
    token: Token,

    /// The selector that determines which protocol is called and when.
    selected: SelectedProtocol,

    /// The event loop handle, this handles all event loop interactions.
    event_loop: EventLoopHandle,

    /// The http/1 protocol handler.
    h1: h1::H1Protocol,

    /// The writer buffer that covers all protocols, this saves memory as
    /// we have to create each protocol instance per client so we dont want
    /// to be creating 3 * 256KB every time.
    writer_buffer: BytesMut,

    /// The reader buffer that covers all protocols, this saves memory as
    /// we have to create each protocol instance per client so we dont want
    /// to be creating 3 * 256KB every time.
    reader_buffer: BytesMut,
}

impl AutoProtocol {
    /// Creates a new auto protocol with the protocol set the specified
    /// `SelectedProtocol` enum.
    pub fn new(
        token: Token,
        selected: SelectedProtocol,
        event_loop: EventLoopHandle,
        callback: CallbackHandler,
    ) -> Self {

        let mut h1 = h1::H1Protocol::new(
            token,
            callback,
             event_loop.clone(),
        );

        let buff1 = BytesMut::with_capacity(MAX_BUFFER_LIMIT);
        let buff2 = BytesMut::with_capacity(MAX_BUFFER_LIMIT);

        Self {
            token,
            selected,
            event_loop,
            h1,
            writer_buffer: buff1,
            reader_buffer: buff2,
        }
    }
}

impl AutoProtocol {
    /// Called when the protocol is in charge of a new socket / handle.
    pub fn new_connection(&mut self) -> PyResult<()> {
        match self.selected {
            SelectedProtocol::H1 => self.h1.new_connection()?,
        };

        Ok(())
    }

    /// Called when the connection is lost from the protocol in order to
    /// properly reset state.
    pub fn lost_connection(&mut self) -> PyResult<()> {
        self.reader_buffer.clear();
        self.writer_buffer.clear();

        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.lost_connection()
            },
        }
    }
}

impl AutoProtocol {
    /// Allows the chance to switch protocol just after reading has
    /// finished.
    pub fn maybe_switch(&mut self) -> PyResult<SwitchStatus> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.switch_protocol()
            },
        }
    }

    /// Pauses reading from the event loop and notifies the protocol of
    /// the pause to allow the protocol to re-wake the state later on.
    fn pause_writing(&mut self) -> PyResult<()> {
        self.event_loop.pause_writing(self.token);
        Ok(())
    }

    /// The EOF has been sent by the socket.
    pub fn eof_received(&mut self) -> PyResult<()> {
        match self.selected {
            SelectedProtocol::H1 => {
                self.h1.eof_received()
            },
        }
    }
}

impl SocketCommunicator for AutoProtocol {
    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn read_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        Ok(&mut self.reader_buffer)
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn read_buffer_filled(&mut self, _amount: usize) -> PyResult<()> {
        return match self.selected {
            SelectedProtocol::H1 => {
                self.h1.data_received(&mut self.reader_buffer)
            },
        }
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn write_buffer_acquire(&mut self) -> PyResult<&mut BytesMut> {
        match self.selected {
            SelectedProtocol::H1 => {
                self.h1.fill_write_buffer(&mut self.writer_buffer)?;
            },
        };

        Ok(&mut self.writer_buffer)
    }

    /// Called when data is able to be read from the socket, the returned
    /// buffer is filled and then the read_buffer_filled callback is invoked.
    fn write_buffer_drained(&mut self, amount: usize) -> PyResult<()> {
        if (amount == 0) | (self.writer_buffer.len() == 0) {
            self.pause_writing()?;
        }

        Ok(())
    }
}