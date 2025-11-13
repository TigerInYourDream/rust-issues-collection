// BROKEN EXAMPLE: Attempting to use closures with C function pointers
//
// This example demonstrates why Rust closures cannot be directly passed
// to C functions expecting function pointers, even when their signatures
// appear compatible.

use std::os::raw::{c_int, c_void};

// Simulating libc::qsort signature
unsafe extern "C" {
    fn qsort(
        base: *mut c_void,
        num: usize,
        size: usize,
        comparator: extern "C" fn(*const c_void, *const c_void) -> c_int,
    );
}

// PROBLEM 1: Attempting to use a closure directly
// This won't compile because closures are NOT function pointers
#[allow(dead_code)]
fn attempt_1_direct_closure() {
    let array = [5, 2, 8, 1, 9, 3];
    println!("Before sort: {:?}", array);

    // COMPILE ERROR: expected fn pointer, found closure
    // Uncommenting this will fail to compile:
    /*
    unsafe {
        qsort(
            array.as_mut_ptr() as *mut c_void,
            array.len(),
            std::mem::size_of::<i32>(),
            |a: *const c_void, b: *const c_void| {
                let a = *(a as *const i32);
                let b = *(b as *const i32);
                a.cmp(&b) as c_int
            },
        );
    }
    */

    println!("ERROR: Closures cannot be coerced to function pointers!");
}

// PROBLEM 2: Trying to capture environment in closure
// Even if we could pass closures, capturing variables makes it impossible
#[allow(dead_code)]
fn attempt_2_capturing_closure() {
    let array = [5, 2, 8, 1, 9, 3];
    let reverse = true; // External variable captured by closure

    println!("Attempting to sort with capture: reverse={}", reverse);

    // This closure captures 'reverse', changing its memory layout
    // It's no longer compatible with a simple function pointer
    let _comparator = |a: *const c_void, b: *const c_void| -> c_int {
        unsafe {
            let a = *(a as *const i32);
            let b = *(b as *const i32);
            if reverse {
                b.cmp(&a) as c_int
            } else {
                a.cmp(&b) as c_int
            }
        }
    };

    // Cannot pass _comparator to qsort - it's a closure with environment
    println!("ERROR: Capturing closures have non-trivial memory layout");
    println!("Array unchanged: {:?}", array);
}

// PROBLEM 3: Dangerous transmute attempt
// WARNING: This compiles but causes UNDEFINED BEHAVIOR
fn attempt_3_transmute() {
    let mut array = [5, 2, 8, 1, 9, 3];
    println!("\n=== DANGEROUS: Transmute Attempt ===");
    println!("Before sort: {:?}", array);

    // Create a non-capturing closure (zero-sized)
    let closure = |a: *const c_void, b: *const c_void| -> c_int {
        unsafe {
            let a = *(a as *const i32);
            let b = *(b as *const i32);
            a.cmp(&b) as c_int
        }
    };

    // UNDEFINED BEHAVIOR: Transmuting closure to function pointer
    // Even non-capturing closures have different ABI than function pointers!
    unsafe {
        // This compiles but is UB - closure ABI != function pointer ABI
        let fn_ptr: extern "C" fn(*const c_void, *const c_void) -> c_int =
            std::mem::transmute(&closure as *const _ as usize);

        // This may crash, corrupt memory, or appear to work (worst case!)
        qsort(
            array.as_mut_ptr() as *mut c_void,
            array.len(),
            std::mem::size_of::<i32>(),
            fn_ptr,
        );
    }

    println!("After sort: {:?}", array);
    println!("WARNING: Result may be incorrect or process may have crashed!");
}

// PROBLEM 4: Misunderstanding closure zero-size optimization
fn attempt_4_zst_confusion() {
    println!("\n=== Understanding Closure Layout ===");

    // Non-capturing closure
    let closure_no_capture = |x: i32| x + 1;
    println!(
        "Non-capturing closure size: {} bytes",
        std::mem::size_of_val(&closure_no_capture)
    );

    let y = 10;
    // Capturing closure
    let closure_with_capture = |x: i32| x + y;
    println!(
        "Capturing closure size: {} bytes",
        std::mem::size_of_val(&closure_with_capture)
    );

    // Function pointer size
    let fn_ptr: fn(i32) -> i32 = |x| x + 1;
    println!("Function pointer size: {} bytes", std::mem::size_of_val(&fn_ptr));

    println!("\nKey insight: Even zero-sized closures have different ABI!");
    println!("Closure calling convention != C function pointer convention");
}

// Helper function to print explanation
fn print_explanation() {
    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Why Closures Don't Work as C Function Pointers                   ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("1. MEMORY LAYOUT:");
    println!("   - Function pointer: just a code address (8 bytes on 64-bit)");
    println!("   - Closure: may include captured environment data");
    println!("   - Even non-capturing closures use different calling convention");
    println!();
    println!("2. ABI INCOMPATIBILITY:");
    println!("   - C expects: extern \"C\" calling convention");
    println!("   - Closures use: Rust calling convention (unspecified)");
    println!("   - Type system prevents accidental misuse");
    println!();
    println!("3. TRAIT DIFFERENCES:");
    println!("   - Function pointers: implement Copy");
    println!("   - Closures: may not be Copy (if capturing owned data)");
    println!("   - Closures implement Fn/FnMut/FnOnce, not fn pointer");
    println!();
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║          Closure vs Function Pointer FFI Problem                  ║");
    println!("║                    (Broken Examples)                               ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    // Show different problem scenarios
    attempt_1_direct_closure();
    attempt_2_capturing_closure();
    attempt_4_zst_confusion();

    print_explanation();

    // WARNING: This may crash!
    println!("\n⚠️  Press Enter to run DANGEROUS transmute attempt (may crash)...");
    println!("    Or Ctrl+C to exit safely.");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();

    attempt_3_transmute();

    println!("\n✗ All attempts failed or resulted in undefined behavior");
    println!("✓ See correct-example/ for proper solutions");
}
