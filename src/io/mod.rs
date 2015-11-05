pub mod term;

/// This is basically a braindead reimplementation of the standard
/// library's `Read` trait. Most of the methods available on the
/// standard lib's `Read` are not yet implemented.
pub trait Read {
    type Error;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn read_all(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

pub trait Write {
    type Error;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
}
