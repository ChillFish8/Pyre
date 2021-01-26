use mio::net::{TcpStream, TcpListener};
use mio::{Poll, Events, Token, Interest};
use mio::event::Event;

use std::net::SocketAddr;
use std::io;
use std::error::Error;
use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;


use crate::pyre_server::client::Client;


/// The standard server identifier token.
const SERVER: Token = Token(0);

/// The MAX events that can be enqueued at any one time.
const EVENTS_MAX: usize = 128;


/// The state that has updated on the socket showing its readiness.
pub enum SocketPollState {
    /// The socket can (maybe) be read from.
    Read,

    /// The socket can (maybe) be written to.
    Write,

    /// A end of the socket has been shutdown so communication
    /// cannot continue, note that this may or may not always
    /// happen so it's important to not implicitly rely on this.
    Shutdown,
}


struct TokenCounter {
    internal: usize,
}

impl TokenCounter {
    fn new() -> Self {
        Self { internal: 0 }
    }

    fn next(&mut self) -> Token {
        self.internal += 1;
        let id = self.internal;

        Token(id)
    }
}


/// The high-level handler for interacting with the server.
///
/// Interestingly this is actually controlled by the LowLevelServer
/// as the state machine is driven by the events emitted from the
/// LowLevelServer.
pub struct HighLevelServer {
    /// The mapping that stores all active clients, this is used to
    /// invoke the relevant callbacks when the event loop state changes.
    clients: FxHashMap<Token, Client>,

    /// A cheaply cloneable reference to the main poller of the event loop.
    poll: Rc<RefCell<Poll>>,

    /// The incremental counter that is used to generate new tokens.
    counter: TokenCounter,
}

impl HighLevelServer {
    /// Build a new HighLevelServer that takes the poll struct instance
    /// in order to share it with clients.
    pub fn new(poll: Rc<RefCell<Poll>>) -> Self {
        let clients = FxHashMap::default();
        let counter = TokenCounter::new();

        Self {
            clients,
            poll,
            counter,
        }
    }

    /// Invoked when ever a client is accepted from the listener.
    fn client_accepted(
        &mut self,
        mut stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn Error>> {

        let token = self.counter.next();
        let client = Client::build_from(
            token,
            stream,
            addr,
        );

        self.clients.insert(token, client);

        Ok(())
    }

    /// Invoked when ever a client state changes e.g. reading, writing or
    /// shutdown readiness.
    fn socket_state_update(
        &mut self,
        token: Token,
        state: SocketPollState
    ) -> Result<(), Box<dyn Error>> {
        let client = self.clients.get_mut(&token)
            .expect("No client at token.");

        match state {
            SocketPollState::Read => client.read_ready()?,
            SocketPollState::Write => client.write_ready()?,
            SocketPollState::Shutdown => client.sock_shutdown()?,
        };

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
    pub fn from_addr(addr: String) -> io::Result<Self> {
        let host = addr.parse()
            .expect("Failed to build SocketAddr from addr");

        let listener = TcpListener::bind(host)?;

        let poll = Rc::new(RefCell::new(Poll::new()?));

        let event_handler = HighLevelServer::new(poll.clone());

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
            self.event_handler.socket_state_update(
                event.token(),
                SocketPollState::Read
            )?;
        }

        if event.is_writable() {
            self.event_handler.socket_state_update(
                event.token(),
                SocketPollState::Write
            )?;
        }

        if event.is_write_closed() | event.is_read_closed() {
            self.event_handler.socket_state_update(
                event.token(),
                SocketPollState::Shutdown,
            )?;
        }


        Ok(())
    }
}
