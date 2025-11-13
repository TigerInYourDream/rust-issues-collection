// BROKEN EXAMPLE: Demonstrates undefined behavior with self-referential structures
// This code compiles but exhibits dangerous undefined behavior

mod naive_future;
mod why_ub;

fn main() {
    println!("=== Pin and Self-Referential Structures - BROKEN EXAMPLES ===\n");

    // Example 1: Simple self-referential struct
    println!("Example 1: Self-referential struct with move");
    example1_self_referential_move();
    println!();

    // Example 2: Self-referential in Vec (reallocation)
    println!("Example 2: Self-referential in Vec with reallocation");
    example2_vec_reallocation();
    println!();

    // Example 3: Lifetime attempt (compilation error - commented out)
    println!("Example 3: Attempting to use lifetimes (see commented code)");
    // example3_lifetime_attempt();  // Does not compile
    println!("This example does not compile - see source code\n");

    // Example 4: Why this is UB - detailed analysis
    println!("Example 4: Detailed UB analysis");
    why_ub::demonstrate_ub();
    println!();

    println!("=== All examples completed ===");
    println!("\nWARNING: These examples demonstrate UNDEFINED BEHAVIOR.");
    println!("In production, this could cause:");
    println!("- Segmentation faults");
    println!("- Data corruption");
    println!("- Unpredictable crashes");
    println!("\nRun with Miri to detect UB:");
    println!("  cargo +nightly miri run");
}

/// Example 1: Simple self-referential struct
/// PROBLEM: Pointer becomes dangling when struct is moved
fn example1_self_referential_move() {
    struct SelfReferential {
        data: String,
        ptr_to_data: *const u8,
    }

    impl SelfReferential {
        fn new(text: &str) -> Self {
            let data = String::from(text);
            let ptr_to_data = data.as_ptr();

            Self {
                data,
                ptr_to_data, // PROBLEM: Points to data's heap buffer
            }
        }

        fn get_data(&self) -> &str {
            unsafe {
                // UNDEFINED BEHAVIOR: ptr_to_data may be dangling
                let slice = std::slice::from_raw_parts(
                    self.ptr_to_data,
                    self.data.len()
                );
                std::str::from_utf8_unchecked(slice)
            }
        }
    }

    // This works (by luck)
    let s1 = SelfReferential::new("hello");
    println!("  s1.data = {:?}", s1.get_data());

    // Move the struct - UB starts here
    let s2 = s1;
    println!("  s2.data = {:?} (might work, might be garbage)", s2.get_data());

    // Note: Depending on memory layout, this might:
    // 1. Work correctly (by luck)
    // 2. Print garbage
    // 3. Segfault
}

/// Example 2: Self-referential in Vec
/// PROBLEM: Vec reallocation moves elements, invalidating internal pointers
fn example2_vec_reallocation() {
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
    }

    let mut vec = Vec::new();
    vec.push(SelfReferential::new("first"));

    println!("  vec[0] before realloc: {:?}", vec[0].get_data());

    // Force reallocation by adding more elements
    vec.push(SelfReferential::new("second"));
    vec.push(SelfReferential::new("third"));
    vec.push(SelfReferential::new("fourth"));

    // UNDEFINED BEHAVIOR: vec[0] was moved during reallocation
    // Its internal pointer is now dangling
    println!("  vec[0] after realloc: {:?} (UNDEFINED BEHAVIOR)", vec[0].get_data());

    // This will likely print garbage or crash
}

/// Example 3: Attempting to use lifetimes
/// This does NOT compile - shown for educational purposes
#[allow(dead_code)]
fn example3_lifetime_attempt() {
    // This code does not compile:
    /*
    struct SelfRef<'a> {
        data: String,
        ptr: &'a str,
    }

    impl<'a> SelfRef<'a> {
        fn new(text: &str) -> Self {
            let data = String::from(text);
            let ptr = &data;  // ERROR: borrowed value does not live long enough
            Self { data, ptr }
        }
    }
    */

    println!("  This example does not compile due to lifetime errors");
    println!("  Rust's borrow checker prevents self-referential structs with lifetimes");
}
