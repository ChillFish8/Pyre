use pyo3::PyResult;


/// The Protocol that is currently active.
pub enum SelectedProtocol {
    /// The HTTP/1.x protocol handler.
    H1,
}


/// Defines the two states a protocol's switch status can be either SwitchTo
/// type T, or dont switch at all.
pub enum SwitchStatus {
    SwitchTo(SelectedProtocol),
    NoSwitch,
}


/// Defines the required methods for making a protocol switchable.
pub trait Switchable {
    /// Invoked just after the socket has been read to give the
    /// chance for the protocol to be switched.
    fn switch_protocol(&mut self) -> PyResult<SwitchStatus>;
}