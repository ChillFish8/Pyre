use mio::net::{TcpStream, TcpListener};
use mio::{Poll, Events, Token, Interest};
use mio::event::Event;

use std::net::SocketAddr;
use std::io;
use std::error::Error;
use std::cell::RefCell;
use std::rc::Rc;


/// The standard server identifier token.
const SERVER: Token = Token(0);

/// The MAX events that can be enqueued at any one time.
const EVENTS_MAX: usize = 128;


pub enum SocketPollState {
    Read,
    Write,
}

/// The high-level handler for interacting with the server.
///
/// Interestingly this is actually controlled by the LowLevelServer
/// as the state machine is driven by the events emitted from the
/// LowLevelServer.
pub struct HighLevelServer {

}

impl HighLevelServer {
    pub fn new() -> Self {
        Self {}
    }

    fn client_accepted(
        &mut self,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn socket_state_update(
        &mut self,
        token: Token,
        state: SocketPollState
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

}


/// The low-level polling side of the server, this is built from a
/// `mio::net::TcpListener` and handles running the event loop itself.
pub struct LowLevelServer {
    /// This server's SocketAddr containing hostname, address and port.
    host: SocketAddr,

    /// The TcpListener itself that the event loop polls off to begin with.
    listener: TcpListener,

    /// A cheaply cloneable reference to the main poller of the event loop.
    poll: Rc<RefCell<Poll>>,

    /// The high-level server that handles everything other than the OS
    /// interactions.
    event_handler: HighLevelServer,
}

impl LowLevelServer {
    /// Builds a server instance from a given addr string e.g.
    /// `127.0.0.1:8080`, this has the potential to raise an io Error
    /// as it binds to the socket in the process of building this server.
    pub fn from_addr(
        addr: String,
        event_handler: HighLevelServer
    ) -> io::Result<Self> {
        let host = addr.parse()
            .expect("Failed to build SocketAddr from addr");

        let listener = TcpListener::bind(host)?;

        let poll = Rc::new(RefCell::new(Poll::new()?));

        Ok(Self {
            host,
            listener,
            poll,
            event_handler,
        })
    }

    /// Starts the event loop on the given thread, this is blocking and
    /// will never exit unless ther`e is an error that causes and abruptly
    /// stops the loop.
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Starting");
        let mut events = Events::with_capacity(EVENTS_MAX);

        self.poll.borrow()
            .registry().register(
            &mut self.listener,
            SERVER,
            Interest::READABLE
        )?;

        loop {
            self.poll.borrow_mut()
                .poll(&mut events, None)?;

            self.process_events(&events)?;
        }

        Ok(())
    }

    /// Manages any events received.
    ///
    /// This is always invoked after poll() has completed and the events
    /// list has been filled.
    fn process_events(&mut self, events: &Events) -> Result<(), Box<dyn Error>> {
        for event in events.iter() {
            match event.token() {
                SERVER => self.on_client_incoming()?,
                _ => self.on_socket_state_change(event)?,
            }
        }

        Ok(())
    }

    /// Handles a client waiting to be accepted from the listener.
    fn on_client_incoming(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let (client, addr) = match self.listener.accept() {
                Ok(pair) => pair,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(())
                },
                Err(e) => {
                    eprintln!("Failed accepting from listener: {:?}", e);
                    return Ok(())
                },
            };
            self.event_handler.client_accepted(client, addr)?;
        }

        Ok(())
    }

    /// Handles any other event other than the server listener being
    /// readable.
    fn on_socket_state_change(
        &mut self,
        event: &Event
    )  -> Result<(), Box<dyn Error>> {

        if event.is_readable() {
            self.event_handler
        }

        if event.is_writable() {
            self.event_handler
        }



        Ok(())
    }
}
