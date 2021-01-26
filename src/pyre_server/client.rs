use mio::net::TcpStream;
use mio::Token;

use std::net::SocketAddr;
use std::error::Error;


/// A handler for a TcpStream.
///
/// This is in charge of managing both the socket and it's relevant event loop
/// handling e.g. adding and remove the socket from the event loop.
pub struct Client {
    token: Token,
    addr: SocketAddr,
    stream: TcpStream,
}

impl Client {
    /// Builds a Client instance from the given token,
    /// stream and socket address.
    pub fn build_from(
        token: Token,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Self {
        Self {
            token,
            stream,
            addr,
        }
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

        Ok(())
    }
}