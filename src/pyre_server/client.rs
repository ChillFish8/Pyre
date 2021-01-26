use mio::net::TcpStream;
use mio::Token;

use std::net::{SocketAddr, Shutdown};
use std::error::Error;

use crate::pyre_server::transport::EventLoopHandle;


/// A handler for a TcpStream.
///
/// This is in charge of managing both the socket and it's relevant event loop
/// handling e.g. adding and remove the socket from the event loop.
pub struct Client {
    /// The event loop token identifier.
    token: Token,

    /// The remote address of the stream.
    addr: SocketAddr,

    /// The TcpStream itself.
    pub stream: TcpStream,

    /// A cheaply cloneable handle for updating event loop calls.
    transport: EventLoopHandle,

    /// Is the socket being listened to by the event loop for reading.
    pub is_reading: bool,

    /// Is the socket being listened to by the event loop for writing.
    pub is_writing: bool,

    /// Whether or not the client is idle by not handling the stream
    /// anymore or is inactive.
    pub is_idle: bool,
}

impl Client {
    /// Builds a Client instance from the given token,
    /// stream and socket address.
    pub fn build_from(
        token: Token,
        stream: TcpStream,
        addr: SocketAddr,
        transport: EventLoopHandle,
    ) -> Self {
        Self {
            token,
            stream,
            addr,
            transport,

            is_reading: false,
            is_writing: false,
            is_idle: false,
        }
    }

    /// Allows the client to handle a new stream by essentially
    /// resetting it state.
    pub fn handle_new(
        &mut self,
        stream: TcpStream,
        addr: SocketAddr,
    ) {
        self.stream = stream;
        self.addr = addr;

        self.is_reading = false;
        self.is_writing = false;
        self.is_idle = false;
    }
}


/// Event loop event callbacks.
impl Client {
    /// Invoked when the socket is readable.
    ///
    /// This should be used to propel the state machine of the server
    /// for the most part e.g. parsing and invoking callbacks.
    pub fn read_ready(&mut self) -> Result<(), Box<dyn Error>> {

        Ok(())
    }

    /// Invoked when the socket is writeable.
    ///
    /// This can be used to propel the state machine of the server
    /// but that should mostly be done with the read event, this can
    /// be used to drain the writing buffer and wake up python tasks.
    pub fn write_ready(&mut self) -> Result<(), Box<dyn Error>> {

        Ok(())
    }

    /// Invoked when the socket has closed at least one half of its
    /// interface.
    ///
    /// If either the read or write side of the socket has been closed
    /// the whole stream should probably be closed down as the server is
    /// no longer able to continue with this stream.
    ///
    /// NOTE:
    /// This is not guaranteed to always be called when a socket shuts down.
    pub fn sock_shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        self.is_idle = true;
        let _ = self.stream.shutdown(Shutdown::Write);
        Ok(())
    }

    pub fn check_keep_alive(&mut self) -> Result<(), Box<dyn Error>> {

        Ok(())
    }
}