// Complete async buffer reader implementation using Pin and pin_project
// This is a production-ready example of safe self-referential structure

use pin_project::pin_project;
use std::io;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

/// A buffered asynchronous reader with internal self-reference
///
/// This struct maintains a buffer and a pointer to the filled portion.
/// The pointer is safe because:
/// 1. The struct is marked !Unpin with PhantomPinned
/// 2. It must be used through Pin<&mut Self>
/// 3. pin_project ensures safe field access
#[pin_project]
pub struct AsyncBufReader<R> {
    #[pin]
    inner: R,

    // The buffer is pinned to prevent reallocation
    buffer: Box<[u8]>,

    // Raw pointer to filled portion of buffer
    // SAFETY: Valid as long as the struct is pinned
    filled_ptr: *const u8,
    filled_len: usize,

    // Current read position
    pos: usize,

    // Mark as !Unpin to prevent moving
    _pin: PhantomPinned,
}

impl<R> AsyncBufReader<R> {
    /// Create a new AsyncBufReader with specified buffer size
    ///
    /// Returns Pin<Box<Self>> to ensure the struct is immediately pinned
    pub fn new(inner: R, capacity: usize) -> Pin<Box<Self>> {
        let buffer = vec![0u8; capacity].into_boxed_slice();
        let filled_ptr = buffer.as_ptr();

        let reader = Self {
            inner,
            buffer,
            filled_ptr,
            filled_len: 0,
            pos: 0,
            _pin: PhantomPinned,
        };

        Box::pin(reader)
    }

    /// Get a reference to the filled buffer
    ///
    /// SAFETY: This is safe because:
    /// - The struct is pinned (cannot move)
    /// - filled_ptr points to buffer which is also pinned
    /// - We never reallocate buffer (it's a Box<[u8]>, not Vec)
    pub fn filled(self: Pin<&Self>) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.filled_ptr, self.filled_len)
        }
    }

    /// Get the available (unread) portion of the buffer
    pub fn available(self: Pin<&Self>) -> &[u8] {
        let filled = self.filled();
        &filled[self.pos..]
    }

    /// Consume bytes from the buffer
    pub fn consume(self: Pin<&mut Self>, amt: usize) {
        let this = self.project();
        *this.pos = (*this.pos + amt).min(*this.filled_len);
    }
}

impl<R: AsyncRead> AsyncBufReader<R> {
    /// Fill the buffer by reading from the inner reader
    ///
    /// This is the core async operation that demonstrates Pin usage
    pub fn poll_fill_buf(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<&[u8]>> {
        // Use pin_project to safely access fields
        let this = self.project();

        // If we have unread data, return it
        if *this.pos < *this.filled_len {
            let filled = unsafe {
                std::slice::from_raw_parts(*this.filled_ptr, *this.filled_len)
            };
            return Poll::Ready(Ok(&filled[*this.pos..]));
        }

        // Need to read more data
        *this.pos = 0;

        // Create ReadBuf from our buffer
        let mut read_buf = ReadBuf::new(this.buffer);

        // Poll the inner reader
        match this.inner.poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let filled_len = read_buf.filled().len();
                *this.filled_len = filled_len;

                // Update the pointer (safe because buffer is pinned)
                *this.filled_ptr = this.buffer.as_ptr();

                let filled = unsafe {
                    std::slice::from_raw_parts(*this.filled_ptr, *this.filled_len)
                };

                Poll::Ready(Ok(filled))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

// Implement AsyncRead for our buffered reader
impl<R: AsyncRead> AsyncRead for AsyncBufReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // Get available data
        let available = match self.as_mut().poll_fill_buf(cx) {
            Poll::Ready(Ok(data)) => data,
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        };

        // Copy to output buffer
        let to_read = available.len().min(buf.remaining());
        buf.put_slice(&available[..to_read]);

        // Mark as consumed
        self.consume(to_read);

        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_basic_read() {
        let data = b"Hello, Pin!";
        let cursor = Cursor::new(data.to_vec());

        let mut reader = AsyncBufReader::new(cursor, 1024);

        // Read some bytes
        let mut buf = vec![0u8; 5];
        reader.as_mut().read_exact(&mut buf).await.unwrap();

        assert_eq!(&buf, b"Hello");
    }

    #[tokio::test]
    async fn test_filled_buffer() {
        let data = b"Test data for buffer";
        let cursor = Cursor::new(data.to_vec());

        let mut reader = AsyncBufReader::new(cursor, 1024);

        // Fill the buffer
        let filled = reader.as_mut().poll_fill_buf(&mut Context::from_waker(
            &futures::task::noop_waker()
        ));

        if let Poll::Ready(Ok(data)) = filled {
            assert_eq!(data, b"Test data for buffer");
        } else {
            panic!("Expected data");
        }
    }

    #[tokio::test]
    async fn test_consume() {
        let data = b"0123456789";
        let cursor = Cursor::new(data.to_vec());

        let mut reader = AsyncBufReader::new(cursor, 1024);

        // Fill buffer
        let _ = reader.as_mut().poll_fill_buf(&mut Context::from_waker(
            &futures::task::noop_waker()
        ));

        // Read 5 bytes
        let mut buf = vec![0u8; 5];
        reader.as_mut().read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"01234");

        // Available should now be "56789"
        let available = reader.as_ref().available();
        assert_eq!(available, b"56789");
    }

    #[tokio::test]
    async fn test_pin_prevents_move() {
        let data = b"data";
        let cursor = Cursor::new(data.to_vec());

        let reader = AsyncBufReader::new(cursor, 1024);

        // Cannot move out of Pin<Box<T>>
        // This would not compile:
        // let moved = *reader;

        // Can only access through Pin
        let _ = reader.as_ref().filled();
    }

    #[tokio::test]
    async fn test_multiple_reads() {
        let data = b"Line 1\nLine 2\nLine 3\n";
        let cursor = Cursor::new(data.to_vec());

        let mut reader = AsyncBufReader::new(cursor, 1024);

        let mut buf = String::new();
        reader.read_to_string(&mut buf).await.unwrap();

        assert_eq!(buf, "Line 1\nLine 2\nLine 3\n");
    }
}
