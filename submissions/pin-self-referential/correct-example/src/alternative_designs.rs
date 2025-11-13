// Alternative designs that avoid self-reference
// These are often simpler and safer than using Pin

pub fn demonstrate_alternatives() {
    println!("  [1] Using indices instead of pointers:");
    demo_index_based();
    println!();

    println!("  [2] Separating ownership:");
    demo_separated_ownership();
    println!();

    println!("  [3] Lazy computation:");
    demo_lazy_computation();
}

/// Strategy 1: Use indices instead of pointers
/// Indices are stable across moves
fn demo_index_based() {
    struct IndexBased {
        data: Vec<u8>,
        filled_range: std::ops::Range<usize>,
    }

    impl IndexBased {
        fn new() -> Self {
            Self {
                data: vec![0; 1024],
                filled_range: 0..0,
            }
        }

        fn fill_with(&mut self, bytes: &[u8]) {
            let len = bytes.len().min(self.data.len());
            self.data[..len].copy_from_slice(&bytes[..len]);
            self.filled_range = 0..len;
        }

        fn get_filled(&self) -> &[u8] {
            &self.data[self.filled_range.clone()]
        }
    }

    let mut buffer = IndexBased::new();
    buffer.fill_with(b"Hello, indices!");

    println!("    Filled data: {:?}",
        std::str::from_utf8(buffer.get_filled()).unwrap());

    // Can move freely - indices remain valid
    let buffer2 = buffer;
    println!("    After move:  {:?}",
        std::str::from_utf8(buffer2.get_filled()).unwrap());

    println!("    Pros: Simple, safe, no Pin needed");
    println!("    Cons: Recomputes slice on every access");
}

/// Strategy 2: Separate ownership
/// Don't store references at all - compute them on demand
fn demo_separated_ownership() {
    struct Buffer {
        data: Box<[u8]>,
        filled_len: usize,
    }

    impl Buffer {
        fn new(size: usize) -> Self {
            Self {
                data: vec![0u8; size].into_boxed_slice(),
                filled_len: 0,
            }
        }

        fn fill_with(&mut self, bytes: &[u8]) {
            let len = bytes.len().min(self.data.len());
            self.data[..len].copy_from_slice(&bytes[..len]);
            self.filled_len = len;
        }

        // Compute the slice on each call - no stored reference
        fn get_filled(&self) -> &[u8] {
            &self.data[..self.filled_len]
        }
    }

    struct Reader {
        buffer: Buffer,
        pos: usize,
    }

    impl Reader {
        fn new() -> Self {
            Self {
                buffer: Buffer::new(1024),
                pos: 0,
            }
        }

        fn available(&self) -> &[u8] {
            &self.buffer.get_filled()[self.pos..]
        }
    }

    let mut reader = Reader::new();
    reader.buffer.fill_with(b"Separated ownership!");

    println!("    Available: {:?}",
        std::str::from_utf8(reader.available()).unwrap());

    println!("    Pros: Very safe, clear ownership");
    println!("    Cons: Recomputes on every access");
}

/// Strategy 3: Lazy computation
/// Use a function to compute the value when needed
fn demo_lazy_computation() {
    struct LazyBuffer {
        data: Vec<u8>,
        filled_len: usize,
    }

    impl LazyBuffer {
        fn new() -> Self {
            Self {
                data: vec![0; 1024],
                filled_len: 0,
            }
        }

        fn fill_with(&mut self, bytes: &[u8]) {
            let len = bytes.len().min(self.data.len());
            self.data[..len].copy_from_slice(&bytes[..len]);
            self.filled_len = len;
        }

        // Simple method is preferred over closures for borrowed data
        fn filled(&self) -> &[u8] {
            &self.data[..self.filled_len]
        }
    }

    let mut buffer = LazyBuffer::new();
    buffer.fill_with(b"Lazy evaluation!");

    println!("    Filled: {:?}",
        std::str::from_utf8(buffer.filled()).unwrap());

    println!("    Pros: Flexible, can cache if needed");
    println!("    Cons: Slight indirection overhead");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_index_based_is_movable() {
        struct IndexBased {
            data: Vec<u8>,
            range: std::ops::Range<usize>,
        }

        impl IndexBased {
            fn new(bytes: &[u8]) -> Self {
                let mut data = vec![0; 100];
                data[..bytes.len()].copy_from_slice(bytes);
                Self {
                    data,
                    range: 0..bytes.len(),
                }
            }

            fn get(&self) -> &[u8] {
                &self.data[self.range.clone()]
            }
        }

        let buf1 = IndexBased::new(b"test");
        let buf2 = buf1; // Move

        assert_eq!(buf2.get(), b"test");
    }

    #[test]
    fn test_separated_ownership() {
        struct Buffer {
            data: Vec<u8>,
            len: usize,
        }

        impl Buffer {
            fn new() -> Self {
                Self {
                    data: vec![0; 100],
                    len: 0,
                }
            }

            fn write(&mut self, bytes: &[u8]) {
                self.data[..bytes.len()].copy_from_slice(bytes);
                self.len = bytes.len();
            }

            fn read(&self) -> &[u8] {
                &self.data[..self.len]
            }
        }

        let mut buf = Buffer::new();
        buf.write(b"hello");

        assert_eq!(buf.read(), b"hello");

        // Can move freely
        let buf2 = buf;
        assert_eq!(buf2.read(), b"hello");
    }
}
