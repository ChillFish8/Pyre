use mio::net::{TcpStream, TcpListener};
use mio::{Poll, Events, Token, Interest, Waker};
use mio::event::Event;

use std::net::SocketAddr;
use std::io;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use rustc_hash::FxHashMap;

use crate::pyre_server::client::Client;
use crate::pyre_server::transport::{UpdatesQueue, EventUpdate, EventLoopHandle};



/// The standard server identifier token.
const SERVER: Token = Token(0);

/// The wakeup event that checks updates.
const CHECK_UPDATE: Token = Token(1);

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


/// A simple incremental counter that produces new tokens with its
/// given internal counter.
struct TokenCounter {
    internal: usize,
}

impl TokenCounter {
    /// Make a new counter that starts at `0`
    fn new() -> Self {
        Self { internal: 2 }
    }

    /// Get a new token from the counter.
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

    /// The incremental counter that is used to generate new tokens.
    counter: TokenCounter,

    /// A queue of updates that stack up for the event loop.
    transport: EventLoopHandle,
}

impl HighLevelServer {
    /// Build a new HighLevelServer that takes the poll struct instance
    /// in order to share it with clients.
    pub fn new(transport: EventLoopHandle) -> Self {
        let clients = FxHashMap::default();
        let counter = TokenCounter::new();

        Self {
            clients,
            counter,
            transport,
        }
    }

    /// Finds the first client that is classed as 'idle' which is
    /// then selected to be used as the handler of the new connection.
    fn get_idle_client(&self) -> Option<Token> {
        for (key, client) in self.clients.iter() {
            if client.is_idle {
                return Some(*key)
            }
        }

        None
    }

    /// Selects a index using either an existing idle protocol instance
    /// or making a new protocol by returning a index that does not exist in
    /// the clients hashmap.
    fn select_token(&mut self) -> Token {
        match self.get_idle_client() {
            Some(token) => token,
            None => {
                let token = self.counter.next();

                token
            }
        }
    }

    /// Invoked when ever a client is accepted from the listener.
    fn client_accepted(
        &mut self,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn Error>> {

        let token = self.select_token();

        if let Some(client) = self.clients.get_mut(&token) {
            client.handle_new(
                stream,
                addr
            );
        } else {
            let client = Client::build_from(
                token,
                stream,
                addr,
                self.transport.clone(),
            );

            self.clients.insert(token, client);
        }

        self.transport.resume_reading(token);

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

    /// Invoked every n seconds checking for keep alive on sockets.
    fn keep_alive_tick(&mut self) {
        for (_, client) in self.clients.iter_mut() {
            if let Err(e) = client.check_keep_alive() {
                eprintln!("Exception handling client: {:?}", e);
            };
        }
    }

    fn get_client(&mut self, token: &Token) -> &mut Client {
        self.clients.get_mut(token)
            .expect("Failed to get client from token.")
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
    poll: Poll,

    /// A queue of updates that stack up for the event loop.
    updates: UpdatesQueue,

    /// The high-level server that handles everything other than the OS
    /// interactions.
    high_level: HighLevelServer,

    /// The max time between data handling.
    keep_alive_timeout: Duration,
}

impl LowLevelServer {
    /// Builds a server instance from a given addr string e.g.
    /// `127.0.0.1:8080`, this has the potential to raise an io Error
    /// as it binds to the socket in the process of building this server.
    pub fn from_addr(
        addr: String,
        keep_alive_timeout: Duration,
    ) -> io::Result<Self> {
        let host = addr.parse()
            .expect("Failed to build SocketAddr from addr");

        let listener = TcpListener::bind(host)?;

        let poll = Poll::new()?;

        let updates = UpdatesQueue::default();

        let waker = Waker::new(
            poll.registry(),
            CHECK_UPDATE
        )?;

        let transport = EventLoopHandle::from_queue_and_waker(
            updates.clone(),
            Arc::new(waker),
        );

        let high_level = HighLevelServer::new(transport);

        Ok(Self {
            host,
            listener,
            poll,
            updates,
            high_level,
            keep_alive_timeout,
        })
    }

    /// Starts the event loop on the given thread, this is blocking and
    /// will never exit unless ther`e is an error that causes and abruptly
    /// stops the loop.
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let mut events = Events::with_capacity(EVENTS_MAX);

        self.poll.registry()
            .register(
            &mut self.listener,
            SERVER,
            Interest::READABLE
            )?;

        loop {
            let status = self.poll.poll(
                &mut events,
                Some(self.keep_alive_timeout)
            );

            if let Err(e) = status {
                if e.kind() == io::ErrorKind::WouldBlock {
                    self.high_level.keep_alive_tick();
                } else {
                    eprintln!("{:?}", e);
                }
            }

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
                CHECK_UPDATE => self.on_update_wakeup()?,
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
                    break;
                },
                Err(e) => {
                    eprintln!("Failed accepting from listener: {:?}", e);
                    return Ok(());
                },
            };

            self.high_level.client_accepted(client, addr)?;
        };

        Ok(())
    }

    /// Handles any update events received e.g. adding reading and writers.
    fn on_update_wakeup(&mut self) -> Result<(), Box<dyn Error>> {
        while let Some(update) = self.updates.pop() {
            self.handle_update(update)?;
        }

        Ok(())
    }

    /// Handles any other event other than the server listener being
    /// readable.
    fn on_socket_state_change(
        &mut self,
        event: &Event
    )  -> Result<(), Box<dyn Error>> {

        let token = event.token();
        if event.is_readable() {
            self.high_level.socket_state_update(
                token,
                SocketPollState::Read
            )?;
        } else if event.is_writable() {
            self.high_level.socket_state_update(
                token,
                SocketPollState::Write
            )?;
        } else if event.is_write_closed() | event.is_read_closed() {
            self.high_level.socket_state_update(
                token,
                SocketPollState::Shutdown,
            )?;

            // Lets remove everything and shut this stream down.
            self.pause_writing(token)?;
            self.pause_reading(token)?;
        }


        Ok(())
    }
}

impl LowLevelServer {
    fn handle_update(&mut self, update: EventUpdate) -> io::Result<()> {
        match update {
            EventUpdate::PauseReading(token) => self.pause_reading(
                token,
            ),
            EventUpdate::PauseWriting(token) => self.pause_writing(
                token
            ),
            EventUpdate::ResumeReading(token) => self.resume_reading(
                token
            ),
            EventUpdate::ResumeWriting(token) => self.resume_writing(
                token
            ),
        }
    }

    fn pause_reading(&mut self, token: Token) -> io::Result<()> {
        let client = self.high_level.get_client(&token);

        // Only need to change something if its actually doing it.
        if client.is_reading {
            if client.is_writing {
                self.poll.registry().reregister(
                    &mut client.stream,
                    token,
                    Interest::WRITABLE,
                )?;
            } else {
                self.poll.registry().deregister(&mut client.stream)?;
            }

            client.is_reading = false;
        }

        Ok(())
    }

    fn pause_writing(&mut self, token: Token) -> io::Result<()> {
        let client = self.high_level.get_client(&token);

        // Only need to change something if its actually doing it.
        if client.is_writing {
            if client.is_reading {
                self.poll.registry().reregister(
                    &mut client.stream,
                    token,
                    Interest::READABLE,
                )?;
            } else {
                self.poll.registry().deregister(&mut client.stream)?;
            }

            client.is_writing = false;
        }

        Ok(())
    }

    fn resume_reading(&mut self, token: Token) -> io::Result<()> {
        let client = self.high_level.get_client(&token);

        // Only need to change something if its actually doing it.
        if !client.is_reading {
            if client.is_writing {
                self.poll.registry().reregister(
                    &mut client.stream,
                    token,
                    Interest::READABLE | Interest::WRITABLE,
                )?;
            } else {
                self.poll.registry().register(
                    &mut client.stream,
                    token,
                    Interest::READABLE,
                )?;
            }

            client.is_reading = true;
        }

        Ok(())
    }

    fn resume_writing(&mut self, token: Token) -> io::Result<()> {
        let client = self.high_level.get_client(&token);

        // Only need to change something if its actually doing it.
        if !client.is_writing {
            if client.is_reading {
                self.poll.registry().reregister(
                    &mut client.stream,
                    token,
                    Interest::READABLE | Interest::WRITABLE,
                )?;
            } else {
                self.poll.registry().register(
                    &mut client.stream,
                    token,
                    Interest::WRITABLE,
                )?;
            }

            client.is_writing = true;
        }

        Ok(())
    }
}