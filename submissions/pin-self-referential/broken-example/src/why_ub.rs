// Detailed analysis of why self-referential structures cause undefined behavior

pub fn demonstrate_ub() {
    println!("  Memory Layout Analysis:");
    println!();

    struct SelfReferential {
        data: String,
        ptr_to_data: *const u8,
    }

    impl SelfReferential {
        fn new(text: &str) -> Self {
            let data = String::from(text);
            let ptr_to_data = data.as_ptr();
            Self { data, ptr_to_data }
        }

        fn get_data(&self) -> &str {
            unsafe {
                let slice = std::slice::from_raw_parts(
                    self.ptr_to_data,
                    self.data.len()
                );
                std::str::from_utf8_unchecked(slice)
            }
        }

        fn show_addresses(&self) {
            println!("    self address:       {:p}", self as *const _);
            println!("    data.as_ptr():      {:p}", self.data.as_ptr());
            println!("    stored ptr_to_data: {:p}", self.ptr_to_data);
            println!("    Match: {}", self.data.as_ptr() == self.ptr_to_data);
        }
    }

    // Initial state
    println!("  [1] Creating s1:");
    let s1 = SelfReferential::new("hello");
    s1.show_addresses();
    println!("    Content: {:?}", s1.get_data());
    println!();

    // After move
    println!("  [2] After move to s2:");
    let s2 = s1;
    s2.show_addresses();

    // In this specific case, String's heap pointer is copied correctly,
    // so ptr_to_data still points to the valid heap location.
    // However, this is fragile!
    println!("    Content: {:?}", s2.get_data());
    println!();

    println!("  [3] The REAL danger - reassignment:");
    let mut s3 = SelfReferential::new("world");
    println!("  Before reassignment:");
    s3.show_addresses();

    // Store the old pointer value for comparison
    let old_ptr = s3.ptr_to_data;

    // Reassign data - this changes the heap allocation
    s3.data = String::from("new string value that is much longer");

    println!();
    println!("  After reassignment:");
    s3.show_addresses();
    println!("    Old ptr_to_data: {:p}", old_ptr);
    println!("    Pointers match:  {}", s3.data.as_ptr() == s3.ptr_to_data);
    println!();

    if s3.data.as_ptr() != s3.ptr_to_data {
        println!("  DANGER: ptr_to_data is now DANGLING!");
        println!("  Reading from it is UNDEFINED BEHAVIOR");
        // Uncommenting the next line might crash:
        // println!("    Content: {:?}", s3.get_data());
    }

    println!();
    println!("  [4] Why this matters:");
    println!("  - The pointer becomes stale when:");
    println!("    a) String is reassigned");
    println!("    b) String grows beyond capacity (reallocation)");
    println!("    c) String is moved to a different memory location");
    println!("  - Rust's move semantics copy bytes but don't update internal pointers");
    println!("  - This is why Pin is necessary for self-referential structures");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ub_demonstration() {
        // This test demonstrates the UB pattern, but does not rely on panic
        // UB behavior is unpredictable - may work, may crash, may return garbage

        struct SelfRef {
            data: String,
            ptr: *const u8,
        }

        impl SelfRef {
            fn new(s: &str) -> Self {
                let data = String::from(s);
                let ptr = data.as_ptr();
                Self { data, ptr }
            }

            fn ptr_matches(&self) -> bool {
                self.ptr == self.data.as_ptr()
            }
        }

        let s = SelfRef::new("hello");
        assert!(s.ptr_matches(), "Initially pointer should match");

        // After move, in this simple case String's heap pointer is copied correctly
        // so ptr still points to valid memory (String's heap buffer)
        let s2 = s;
        // This may or may not match depending on implementation details
        // The key point is we cannot rely on it
        let _ = s2.ptr_matches();

        // The test passes to show the code compiles,
        // but the behavior is undefined in more complex scenarios
        // Run with Miri to detect UB: cargo +nightly miri test
    }
}
