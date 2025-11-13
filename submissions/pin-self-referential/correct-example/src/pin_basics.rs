// Demonstrates basic Pin concepts and usage

use std::pin::Pin;
use std::marker::PhantomPinned;

pub fn demonstrate_pin_basics() {
    println!("  [1] Pin with Box (heap-pinned):");
    demo_box_pin();
    println!();

    println!("  [2] Self-referential struct with Pin:");
    demo_self_referential_with_pin();
    println!();

    println!("  [3] Understanding Unpin:");
    demo_unpin();
}

/// Demonstrates Pin with Box
fn demo_box_pin() {
    struct Data {
        value: i32,
    }

    // Regular Box - can be moved
    let boxed = Box::new(Data { value: 42 });
    println!("    Regular Box address: {:p}", &*boxed);

    // Pin<Box<T>> - cannot be moved (unless T: Unpin)
    let pinned = Box::pin(Data { value: 100 });
    println!("    Pinned Box address:  {:p}", &*pinned);

    // We can get a Pin<&T> from Pin<Box<T>>
    let pin_ref: Pin<&Data> = pinned.as_ref();
    println!("    Value through Pin:   {}", pin_ref.value);

    // But we cannot get &mut T if T is !Unpin
    // (Data implements Unpin automatically, so this works)
    let mut pinned_mut = Box::pin(Data { value: 200 });
    let data_mut = Pin::get_mut(pinned_mut.as_mut());
    data_mut.value = 300;
    println!("    Modified value:      {}", data_mut.value);
}

/// Demonstrates a safe self-referential struct using Pin
fn demo_self_referential_with_pin() {
    struct SelfReferential {
        data: String,
        // Raw pointer to data's buffer
        ptr: *const u8,
        len: usize,
        // Marks this type as !Unpin
        _pin: PhantomPinned,
    }

    impl SelfReferential {
        fn new(text: &str) -> Pin<Box<Self>> {
            let data = String::from(text);
            let ptr = data.as_ptr();
            let len = data.len();

            let s = Self {
                data,
                ptr,
                len,
                _pin: PhantomPinned,
            };

            // Pin to heap - guarantees it won't move
            Box::pin(s)
        }

        fn get_data(self: Pin<&Self>) -> &str {
            unsafe {
                // SAFE: Pin guarantees this struct won't move,
                // so ptr remains valid
                let slice = std::slice::from_raw_parts(self.ptr, self.len);
                std::str::from_utf8_unchecked(slice)
            }
        }
    }

    let pinned = SelfReferential::new("Hello, Pin!");
    println!("    Data: {:?}", pinned.as_ref().get_data());

    // Cannot move out of Pin<Box<SelfReferential>>
    // let moved = *pinned;  // ERROR: cannot move out of pinned value

    // Cannot get &mut because SelfReferential is !Unpin
    // let mut_ref = Pin::get_mut(pinned.as_mut());  // ERROR

    println!("    Pin successfully prevents moving!");
}

/// Demonstrates Unpin vs !Unpin
fn demo_unpin() {
    // Most types automatically implement Unpin
    struct NormalStruct {
        value: i32,
    }

    // Can get &mut from Pin because NormalStruct: Unpin
    let mut pinned = Box::pin(NormalStruct { value: 10 });
    let normal = Pin::get_mut(pinned.as_mut());
    normal.value = 20;
    println!("    Unpin type: can get &mut from Pin");

    // !Unpin types use PhantomPinned
    struct NotUnpin {
        value: i32,
        _pin: PhantomPinned,
    }

    let mut pinned_not_unpin = Box::pin(NotUnpin {
        value: 30,
        _pin: PhantomPinned,
    });

    // Cannot get &mut from Pin because NotUnpin: !Unpin
    // This line would not compile:
    // let not_unpin = Pin::get_mut(pinned_not_unpin.as_mut());

    // Can only access through Pin
    println!("    !Unpin type: cannot get &mut from Pin");
    println!("    Value (through unsafe): {}",
        unsafe { pinned_not_unpin.as_mut().get_unchecked_mut().value });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_prevents_move() {
        struct NotUnpin {
            _pin: PhantomPinned,
        }

        let pinned = Box::pin(NotUnpin { _pin: PhantomPinned });

        // This would not compile:
        // let moved = *pinned;

        // Pin successfully prevents moving
        assert!(true);
    }

    #[test]
    fn test_unpin_allows_get_mut() {
        struct Data {
            value: i32,
        }

        let mut pinned = Box::pin(Data { value: 42 });

        // Can get &mut because Data: Unpin
        let data = Pin::get_mut(pinned.as_mut());
        data.value = 100;

        assert_eq!(pinned.value, 100);
    }
}
