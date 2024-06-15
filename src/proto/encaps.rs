use std::{future::Future, mem};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};

/// Writes encapsulated messages
pub trait AsyncMsgSend {
    /// Sends a message
    fn send(&mut self, msg: &[u8]) -> impl Future<Output = std::io::Result<()>>;
}

/// Receives encapsulated messages
pub trait AsyncMsgRecv {
    /// Sends a message
    fn recv(&mut self) -> impl Future<Output = io::Result<Vec<u8>>>;
}

/// Wrapper for AsyncWriteExt object that provides length-and-message encapsulation
pub struct LenU64EncapsMsgSender<W> {
    writer: W,
}

impl<W> LenU64EncapsMsgSender<W>
where
    W: AsyncWriteExt + Unpin,
{
    /// Creates a new EncapsulatedWriter
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W> AsyncMsgSend for LenU64EncapsMsgSender<W>
where
    W: AsyncWriteExt + Unpin,
{
    /// Sends a length-and-message encapulated message
    async fn send(&mut self, msg: &[u8]) -> io::Result<()> {
        // Convert length of message to u64 type that is going to be sent

        let len = u64::try_from(msg.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "message too big for encapsulation")
        })?;

        // Send length and message
        self.writer.write_all(&len.to_be_bytes()).await?;
        self.writer.write_all(msg).await?;

        Ok(())
    }
}

/// Wrapper for AsyncReadExt object that provides length-and-message encapsulation
pub struct LenU64EncapsMsgReceiver<R> {
    reader: BufReader<R>,
}

impl<R> LenU64EncapsMsgReceiver<R>
where
    R: AsyncReadExt + Unpin,
{
    /// Creates a new EncapsulatedReader
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }
}

impl<R> AsyncMsgRecv for LenU64EncapsMsgReceiver<R>
where
    R: AsyncReadExt + Unpin,
{
    /// Receives a length-and-message encapsulated message
    async fn recv(&mut self) -> io::Result<Vec<u8>> {
        // Read length
        let mut len = [0u8; mem::size_of::<u64>()];
        self.reader.read_exact(&mut len).await?;
        let len = u64::from_be_bytes(len);

        // Convert length to system size
        let len = usize::try_from(len).map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "message too big for encapsulation")
        })?;

        // Read message of length
        let mut msg = vec![0u8; len];
        self.reader.read_exact(&mut msg).await?;

        Ok(msg)
    }
}
