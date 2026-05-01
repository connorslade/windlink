use std::{
    io::{Read, Result, Write},
    net::TcpStream,
    sync::{Arc, Mutex, MutexGuard},
};

pub trait ForceLock<T> {
    fn force_lock(&self) -> MutexGuard<'_, T>;
}

impl<T> ForceLock<T> for Mutex<T> {
    fn force_lock(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(i) => i,
            Err(e) => e.into_inner(),
        }
    }
}

#[derive(Clone)]
pub struct SharedStream {
    stream: Arc<TcpStream>,
}

impl SharedStream {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: Arc::new(stream),
        }
    }
}

impl Read for SharedStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (&*self.stream).read(buf)
    }
}

impl Write for SharedStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (&*self.stream).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (&*self.stream).flush()
    }
}
