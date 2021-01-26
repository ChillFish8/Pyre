#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

mod pyre_server;

use crate::pyre_server::server;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;


/// Creates a client handler instance linked to a TcpListener and event loop.
///
/// Args:
///     host:
///         The given host string to bind to e.g. '127.0.0.1'.
///     port:
///         The given port to bind to e.g. 6060.
///     backlog:
///         The max amount of iterations to do when accepting clients
///         when the socket is ready and has been invoked.
///
/// Returns:
///     A un-initialised HandleClients instance linked to the main listener.
#[pyfunction]
fn create_server(
    host: &str,
    port: u16,
) -> PyResult<()> {
    println!("Running on http://{}:{}", host, port);
    let bind = format!("{}:{}", host, port);

    let mut server = server::LowLevelServer::from_addr(bind)?;
    if let Err(e) = server.start() {
        eprintln!("{:?}", e);
    };
    Ok(())
}


///
/// Wraps all our existing pyobjects together in the module
///
#[pymodule]
fn pyre_test(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_server, m)?)?;
    //m.add_class::<_Server>()?;
    //m.add_class::<DataSender>()?;
    //m.add_class::<DataReceiver>()?;
    Ok(())
}
