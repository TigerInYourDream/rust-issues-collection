// CORRECT EXAMPLE: Proper ways to use function pointers with FFI
//
// This example demonstrates multiple approaches to correctly pass
// comparison logic to C functions, including how to simulate
// closure-like behavior with context pointers.

use std::os::raw::{c_int, c_void};

// Import the C standard library qsort function
unsafe extern "C" {
    fn qsort(
        base: *mut c_void,
        num: usize,
        size: usize,
        comparator: extern "C" fn(*const c_void, *const c_void) -> c_int,
    );
}

// ============================================================================
// SOLUTION 1: Use plain function pointers (no capturing)
// ============================================================================

// Simple function pointer - compatible with C calling convention
extern "C" fn compare_ascending(a: *const c_void, b: *const c_void) -> c_int {
    unsafe {
        let a_val = *(a as *const i32);
        let b_val = *(b as *const i32);
        a_val.cmp(&b_val) as c_int
    }
}

extern "C" fn compare_descending(a: *const c_void, b: *const c_void) -> c_int {
    unsafe {
        let a_val = *(a as *const i32);
        let b_val = *(b as *const i32);
        b_val.cmp(&a_val) as c_int
    }
}

fn solution_1_function_pointers() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Solution 1: Plain Function Pointers                              ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");

    let mut array = [5, 2, 8, 1, 9, 3];
    println!("Original array: {:?}", array);

    // Sort ascending
    unsafe {
        qsort(
            array.as_mut_ptr() as *mut c_void,
            array.len(),
            std::mem::size_of::<i32>(),
            compare_ascending,
        );
    }
    println!("Ascending sort: {:?}", array);

    // Sort descending
    unsafe {
        qsort(
            array.as_mut_ptr() as *mut c_void,
            array.len(),
            std::mem::size_of::<i32>(),
            compare_descending,
        );
    }
    println!("Descending sort: {:?}\n", array);
}

// ============================================================================
// SOLUTION 2: Using qsort_r with context pointer (simulating captures)
// ============================================================================

// Many C libraries provide *_r variants that accept a context pointer
// This simulates how to pass additional data (like closure captures)

// Context structure to simulate closure environment
#[repr(C)]
struct SortContext {
    reverse: bool,
    threshold: i32, // Only sort values below this threshold
}

// Comparison function that uses context
extern "C" fn compare_with_context(
    a: *const c_void,
    b: *const c_void,
    context: *mut c_void,
) -> c_int {
    unsafe {
        let a_val = *(a as *const i32);
        let b_val = *(b as *const i32);
        let ctx = &*(context as *const SortContext);

        // Apply threshold filter
        if a_val > ctx.threshold {
            return 1;
        }
        if b_val > ctx.threshold {
            return -1;
        }

        // Compare based on reverse flag
        if ctx.reverse {
            b_val.cmp(&a_val) as c_int
        } else {
            a_val.cmp(&b_val) as c_int
        }
    }
}

// Wrapper function that simulates qsort_r
// (In real code, you'd use the actual qsort_r from libc)
fn qsort_with_context<T>(
    array: &mut [T],
    context: &SortContext,
    compare: extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int,
) {
    // Note: Real qsort_r exists in libc but is platform-specific
    // This demonstrates the pattern
    let ctx_ptr = context as *const SortContext as *mut c_void;

    // On most Unix systems, you'd use:
    // unsafe { libc::qsort_r(array.as_mut_ptr(), array.len(), size, ctx_ptr, compare) }

    // For demonstration, we'll manually sort
    // Note: Calling extern "C" functions doesn't require unsafe in this context
    for i in 0..array.len() {
        for j in (i + 1)..array.len() {
            let a = &array[i] as *const T as *const c_void;
            let b = &array[j] as *const T as *const c_void;
            if compare(a, b, ctx_ptr) > 0 {
                array.swap(i, j);
            }
        }
    }
}

fn solution_2_context_pointer() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Solution 2: Context Pointer (Simulating Closure Captures)        ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");

    let mut array = [5, 2, 8, 1, 9, 3];
    println!("Original array: {:?}", array);

    // Sort with context (like a closure capturing variables)
    let context = SortContext {
        reverse: false,
        threshold: 10,
    };

    qsort_with_context(&mut array, &context, compare_with_context);
    println!("Sorted (threshold=10, reverse=false): {:?}", array);

    // Sort in reverse
    let context = SortContext {
        reverse: true,
        threshold: 10,
    };

    qsort_with_context(&mut array, &context, compare_with_context);
    println!("Sorted (threshold=10, reverse=true): {:?}\n", array);
}

// ============================================================================
// SOLUTION 3: Rust-style wrapper with trait objects
// ============================================================================

// Define a trait for comparison
trait Comparator {
    fn compare(&self, a: i32, b: i32) -> std::cmp::Ordering;
}

// Implement different comparison strategies
struct AscendingComparator;
impl Comparator for AscendingComparator {
    fn compare(&self, a: i32, b: i32) -> std::cmp::Ordering {
        a.cmp(&b)
    }
}

struct DescendingComparator;
impl Comparator for DescendingComparator {
    fn compare(&self, a: i32, b: i32) -> std::cmp::Ordering {
        b.cmp(&a)
    }
}

struct ModuloComparator {
    modulo: i32,
}
impl Comparator for ModuloComparator {
    fn compare(&self, a: i32, b: i32) -> std::cmp::Ordering {
        (a % self.modulo).cmp(&(b % self.modulo))
    }
}

// Rust-style sort function using trait object
fn rust_sort(array: &mut [i32], comparator: &dyn Comparator) {
    array.sort_by(|a, b| comparator.compare(*a, *b));
}

fn solution_3_trait_objects() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Solution 3: Rust-Style Trait Objects (Idiomatic Approach)        ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");

    let mut array = [5, 2, 8, 1, 9, 3];
    println!("Original array: {:?}", array);

    // Use different comparators
    rust_sort(&mut array, &AscendingComparator);
    println!("Ascending sort: {:?}", array);

    rust_sort(&mut array, &DescendingComparator);
    println!("Descending sort: {:?}", array);

    rust_sort(&mut array, &ModuloComparator { modulo: 3 });
    println!("Modulo 3 sort: {:?}\n", array);
}

// ============================================================================
// SOLUTION 4: Non-capturing closure coercion to function pointer
// ============================================================================

fn solution_4_non_capturing_coercion() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Solution 4: Non-Capturing Closure to Function Pointer            ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");

    let mut array = [5, 2, 8, 1, 9, 3];
    println!("Original array: {:?}", array);

    // Non-capturing closures CAN be coerced to function pointers
    // But ONLY if they don't capture anything and match the signature
    let compare_fn: extern "C" fn(*const c_void, *const c_void) -> c_int = {
        extern "C" fn compare(a: *const c_void, b: *const c_void) -> c_int {
            unsafe {
                let a_val = *(a as *const i32);
                let b_val = *(b as *const i32);
                a_val.cmp(&b_val) as c_int
            }
        }
        compare
    };

    unsafe {
        qsort(
            array.as_mut_ptr() as *mut c_void,
            array.len(),
            std::mem::size_of::<i32>(),
            compare_fn,
        );
    }
    println!("Sorted array: {:?}\n", array);
}

// ============================================================================
// Helper functions
// ============================================================================

fn print_key_lessons() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Key Lessons                                                       ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("✓ Use extern \"C\" fn for FFI function pointers");
    println!("✓ Non-capturing closures CAN be coerced, but prefer explicit fn");
    println!("✓ Capturing closures CANNOT be used as C function pointers");
    println!("✓ Use context pointers (like qsort_r) to pass additional data");
    println!("✓ Prefer Rust-style trait objects when not crossing FFI boundary");
    println!();
    println!("Remember:");
    println!("  - C expects function pointers: just a code address");
    println!("  - Closures may carry environment: different memory layout");
    println!("  - Type system prevents mistakes at compile time");
    println!();
}

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║       Closure vs Function Pointer FFI - Correct Solutions         ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    solution_1_function_pointers();
    solution_2_context_pointer();
    solution_3_trait_objects();
    solution_4_non_capturing_coercion();
    print_key_lessons();

    println!("✓ All solutions work correctly and safely!");
}
