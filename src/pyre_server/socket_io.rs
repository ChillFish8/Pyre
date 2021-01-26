use mio::net::TcpStream;
use bytes::{BytesMut, BufMut};
use std::io::{self, Read, Write};


/// The buffer IO trait allows the TcpStream to read and write form
/// `bytes::BytesMut` directly rather than creating a intermediate
/// buffer like a `&[u8]`.
pub trait BufferIO {
    /// Reads data from self to the buffer.
    fn read_buf(&mut self, buffer: &mut BytesMut) -> io::Result<usize>;

    /// Writes data from the buffer to self.
    fn write_buf(&mut self, buffer: &mut BytesMut) -> io::Result<usize>;
}

impl BufferIO for TcpStream {
    /// Reads data from the socket to the given buffer.
    fn read_buf(&mut self, buffer: &mut BytesMut) -> io::Result<usize> {
        let data = buffer.chunk_mut();
        let mut slice = unsafe {
            std::slice::from_raw_parts_mut(data.as_mut_ptr(),data.len())
        };

        let len = self.read(&mut slice)?;

        unsafe { buffer.advance_mut(len); }

        Ok(len)
    }

    /// Writes data from the given buffer to the socket.
    fn write_buf(&mut self, buffer: &mut BytesMut) -> io::Result<usize> {
        let len = self.write(buffer)?;
        let _ = buffer.split_to(len);
        Ok(len)
    }
}