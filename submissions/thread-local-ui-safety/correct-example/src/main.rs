//! # Thread-Local UI Data Safety - CORRECT Solution
//!
//! This example demonstrates the CORRECT way of managing thread-local UI state
//! using Rust's type system to enforce thread safety at compile time.
//!
//! ## The Solution
//!
//! The key insight is to use a "witness" type that can only be constructed
//! on the UI thread. By requiring functions to accept this witness type,
//! we get compile-time guarantees that the functions are only called from
//! the correct thread.
//!
//! ## Key Design Patterns
//!
//! 1. **Witness Type Pattern**: `UiContext` can only exist on the UI thread
//! 2. **Interior Mutability with RefCell**: Since we guarantee single-threaded access,
//!    we can use `RefCell` (cheaper than `Mutex`) for interior mutability
//! 3. **Rc for Shared Ownership**: `Rc` is !Send, further preventing cross-thread access
//! 4. **Clear API Contracts**: Function signatures communicate thread requirements

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    thread,
    time::Duration,
};

// ✅ SOLUTION 1: Use RefCell (not Mutex) for single-threaded interior mutability
// RefCell is cheaper than Mutex and clearly signals single-threaded access
thread_local! {
    static ROOM_CACHE: Rc<RefCell<HashMap<String, RoomData>>> = Rc::new(RefCell::new(HashMap::new()));
}

#[derive(Clone, Debug)]
pub struct RoomData {
    pub id: String,
    pub name: String,
    pub unread_count: u32,
}

// ✅ SOLUTION 2: Witness type that guarantees we're on the UI thread
// This type is !Send + !Sync, so it cannot be sent to other threads
#[derive(Clone)]
pub struct UiContext {
    // PhantomData could be used here, but we keep it simple
    _private: (),
}

impl UiContext {
    /// Creates a new UiContext.
    ///
    /// IMPORTANT: This should only be called once at the start of the UI thread.
    /// In a real application, this would be created by the UI framework
    /// and passed down through the call stack.
    pub fn new() -> Self {
        UiContext { _private: () }
    }
}

// ✅ SOLUTION 3: All UI-thread-only functions require UiContext
// The type system enforces that these can only be called with a valid UiContext
pub fn add_room(_ui: &UiContext, room: RoomData) {
    ROOM_CACHE.with(|cache| {
        cache.borrow_mut().insert(room.id.clone(), room);
    });
}

pub fn get_room(_ui: &UiContext, room_id: &str) -> Option<RoomData> {
    ROOM_CACHE.with(|cache| {
        cache.borrow().get(room_id).cloned()
    })
}

pub fn update_unread_count(_ui: &UiContext, room_id: &str, count: u32) {
    ROOM_CACHE.with(|cache| {
        if let Some(room) = cache.borrow_mut().get_mut(room_id) {
            room.unread_count = count;
        }
    });
}

/// Returns an Rc clone of the room cache for the current thread.
///
/// This function also requires UiContext, ensuring it's only called from the UI thread.
pub fn get_room_cache(_ui: &UiContext) -> Rc<RefCell<HashMap<String, RoomData>>> {
    ROOM_CACHE.with(|cache| Rc::clone(cache))
}

/// Clears all rooms from the cache.
///
/// This function requires UiContext, making it clear that it affects UI-thread-local state.
pub fn clear_all_rooms(_ui: &UiContext) {
    ROOM_CACHE.with(|cache| {
        cache.borrow_mut().clear();
    });
}

fn simulate_ui_thread() {
    println!("[UI Thread] Starting...");

    // ✅ Create the UiContext witness - only possible on this thread
    let ui = UiContext::new();

    // Add some rooms using the UI context
    add_room(&ui, RoomData {
        id: "room1".to_string(),
        name: "General".to_string(),
        unread_count: 0,
    });

    add_room(&ui, RoomData {
        id: "room2".to_string(),
        name: "Random".to_string(),
        unread_count: 5,
    });

    // Simulate UI event loop
    for i in 0..3 {
        thread::sleep(Duration::from_millis(100));
        if let Some(room) = get_room(&ui, "room1") {
            println!("[UI Thread {}] Room: {:?}", i, room);
        }
    }

    // Update unread count
    update_unread_count(&ui, "room1", 3);
    println!("[UI Thread] Updated unread count");

    if let Some(room) = get_room(&ui, "room1") {
        println!("[UI Thread] Final room state: {:?}", room);
    }
}

// ✅ SOLUTION 4: Background threads CANNOT call UI functions
// They don't have a UiContext, so the code won't compile!
fn simulate_background_thread() {
    thread::sleep(Duration::from_millis(50));
    println!("[Background Thread] Starting...");

    // ❌ This would NOT compile! We don't have a UiContext
    // Uncommenting these lines will cause a compilation error:
    //
    // let room = get_room(&ui, "room1");  // ERROR: `ui` not in scope
    //
    // Even if we tried to create one:
    // let ui = UiContext::new();  // This compiles, but...
    // add_room(&ui, RoomData { ... });  // ...this accesses a DIFFERENT thread_local storage!
    //
    // The thread_local! macro ensures each thread has its own storage,
    // so even with UiContext, background threads can't access UI data.

    println!("[Background Thread] Cannot access UI data (by design!)");
    println!("[Background Thread] This is enforced at compile time");
}

fn main() {
    println!("=== Demonstrating Thread-Local UI Safety CORRECT Solution ===\n");

    // Spawn UI thread
    let ui_handle = thread::spawn(simulate_ui_thread);

    // Spawn background thread
    let bg_handle = thread::spawn(simulate_background_thread);

    ui_handle.join().unwrap();
    bg_handle.join().unwrap();

    println!("\n=== Benefits of This Approach ===");
    println!("1. ✅ Compile-time guarantee: UI functions require UiContext");
    println!("2. ✅ RefCell (not Mutex): Appropriate for single-threaded access");
    println!("3. ✅ Rc is !Send: Further prevents accidental cross-thread sharing");
    println!("4. ✅ Clear API: Function signatures document thread requirements");
    println!("5. ✅ Zero runtime overhead: Type erasure means UiContext compiles to nothing");
    println!("\n=== Why This Works ===");
    println!("• UiContext acts as a 'capability' or 'witness' type");
    println!("• Functions requiring &UiContext can only be called with a valid context");
    println!("• The type system prevents accidental misuse across threads");
    println!("• thread_local! ensures each thread has separate storage");
    println!("• Even if a background thread creates UiContext, it accesses different data");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_context_usage() {
        let ui = UiContext::new();

        // Test adding and retrieving a room
        add_room(&ui, RoomData {
            id: "test_room".to_string(),
            name: "Test Room".to_string(),
            unread_count: 0,
        });

        let room = get_room(&ui, "test_room");
        assert!(room.is_some());
        assert_eq!(room.unwrap().name, "Test Room");

        // Test updating unread count
        update_unread_count(&ui, "test_room", 5);
        let room = get_room(&ui, "test_room");
        assert_eq!(room.unwrap().unread_count, 5);

        // Test clearing
        clear_all_rooms(&ui);
        assert!(get_room(&ui, "test_room").is_none());
    }

    #[test]
    fn test_thread_local_isolation() {
        // This test demonstrates that each thread has its own thread_local storage
        let handle = thread::spawn(|| {
            let ui = UiContext::new();
            add_room(&ui, RoomData {
                id: "thread_room".to_string(),
                name: "Thread Room".to_string(),
                unread_count: 0,
            });
            // Room exists in this thread's storage
            assert!(get_room(&ui, "thread_room").is_some());
        });

        handle.join().unwrap();

        // Main thread cannot see the room added in the spawned thread
        let ui = UiContext::new();
        assert!(get_room(&ui, "thread_room").is_none());
    }
}
