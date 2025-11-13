// Naive attempt at creating a self-referential Future
// This code demonstrates why manual Future implementation is tricky

// NOTE: This file shows conceptual issues and does not compile
// It is included for educational purposes

/*
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// PROBLEM: Attempting to create a self-referential async buffer reader
/// This approach fails because we cannot safely store a reference to our own buffer
pub struct NaiveBufReader<R> {
    inner: R,
    buffer: Vec<u8>,
    // PROBLEM: Cannot store a slice that references buffer
    // filled: &[u8],  // ERROR: missing lifetime specifier
    filled_ptr: *const u8,  // Using raw pointer to bypass borrow checker
    filled_len: usize,
}

impl<R> NaiveBufReader<R> {
    pub fn new(inner: R) -> Self {
        let buffer = vec![0u8; 8192];
        let filled_ptr = buffer.as_ptr();

        Self {
            inner,
            buffer,
            filled_ptr,  // DANGER: This pointer can become dangling
            filled_len: 0,
        }
    }

    /// UNSAFE: This method reads from a potentially dangling pointer
    pub unsafe fn filled(&self) -> &[u8] {
        std::slice::from_raw_parts(self.filled_ptr, self.filled_len)
    }
}

// Implementing Future would require Pin, but even with Pin,
// the manual pointer management is error-prone

impl<R: std::io::Read> Future for NaiveBufReader<R> {
    type Output = std::io::Result<usize>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // PROBLEM: How do we safely access fields?
        // Cannot use self.buffer directly because self is Pin<&mut Self>

        // Option 1: Use get_mut() - but this requires T: Unpin
        // let this = self.get_mut();  // ERROR if NaiveBufReader is !Unpin

        // Option 2: Use get_unchecked_mut() - unsafe and error-prone
        let this = unsafe { self.get_unchecked_mut() };

        // Try to read into buffer
        match this.inner.read(&mut this.buffer) {
            Ok(n) => {
                this.filled_len = n;
                // Update pointer - but what if buffer was reallocated?
                this.filled_ptr = this.buffer.as_ptr();
                Poll::Ready(Ok(n))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
*/

// The correct approach requires:
// 1. Using Pin properly
// 2. Using pin_project or pin_project_lite
// 3. Marking the struct as !Unpin with PhantomPinned
// 4. Careful unsafe code with clear invariants

// See the correct-example for the proper implementation
