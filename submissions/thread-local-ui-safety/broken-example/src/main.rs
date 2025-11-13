//! # Thread-Local UI Data Safety Problem
//!
//! This example demonstrates the INCORRECT way of managing thread-local UI state
//! that can lead to data races and undefined behavior.
//!
//! ## The Problem
//!
//! When building GUI applications with Rust, we often need to maintain state
//! that should only be accessed from the main UI thread. A naive approach is
//! to use `thread_local!` storage, but without proper guards, this can lead to:
//!
//! 1. Accidental access from background threads (compile-time issue)
//! 2. Data races if the wrong synchronization primitives are used
//! 3. Unclear API boundaries about thread safety
//!
//! ## Why This Is Wrong
//!
//! The code below compiles and runs, but has several critical flaws:
//! - No compile-time guarantee that only the UI thread accesses the data
//! - Using `Mutex` for single-threaded data is overkill and misleading
//! - API doesn't communicate thread-safety requirements clearly

use std::{
    collections::HashMap,
    sync::Mutex,
    thread,
    time::Duration,
};

// ❌ PROBLEM 1: Using Mutex for data that should only be accessed from one thread
// This compiles, but Mutex suggests multi-threaded access, which is NOT what we want
thread_local! {
    static ROOM_CACHE: Mutex<HashMap<String, RoomData>> = Mutex::new(HashMap::new());
}

#[derive(Clone, Debug)]
pub struct RoomData {
    pub id: String,
    pub name: String,
    pub unread_count: u32,
}

// ❌ PROBLEM 2: No compile-time guarantee this is called from the UI thread
// Anyone can call this function from any thread!
pub fn add_room(room: RoomData) {
    ROOM_CACHE.with(|cache| {
        cache.lock().unwrap().insert(room.id.clone(), room);
    });
}

// ❌ PROBLEM 3: Same issue - no type-level enforcement of thread safety
pub fn get_room(room_id: &str) -> Option<RoomData> {
    ROOM_CACHE.with(|cache| {
        cache.lock().unwrap().get(room_id).cloned()
    })
}

// ❌ PROBLEM 4: This function can be called from anywhere!
pub fn update_unread_count(room_id: &str, count: u32) {
    ROOM_CACHE.with(|cache| {
        if let Some(room) = cache.lock().unwrap().get_mut(room_id) {
            room.unread_count = count;
        }
    });
}

fn simulate_ui_thread() {
    println!("[UI Thread] Starting...");

    // Add some rooms from the "UI thread"
    add_room(RoomData {
        id: "room1".to_string(),
        name: "General".to_string(),
        unread_count: 0,
    });

    add_room(RoomData {
        id: "room2".to_string(),
        name: "Random".to_string(),
        unread_count: 5,
    });

    // Simulate UI event loop
    for _ in 0..3 {
        thread::sleep(Duration::from_millis(100));
        if let Some(room) = get_room("room1") {
            println!("[UI Thread] Room: {:?}", room);
        }
    }
}

// ❌ PROBLEM 5: Background threads can accidentally access UI-only data!
// This compiles without errors but is semantically wrong
fn simulate_background_thread() {
    thread::sleep(Duration::from_millis(50));
    println!("[Background Thread] Trying to access room data...");

    // This should NOT be allowed, but it compiles!
    // On a different thread, thread_local storage will be different,
    // so this actually accesses a SEPARATE instance of ROOM_CACHE
    // This is confusing and error-prone!
    match get_room("room1") {
        Some(room) => println!("[Background Thread] Found room: {:?}", room),
        None => println!("[Background Thread] Room not found (different thread_local storage!)"),
    }

    // Even worse, we can "update" data that doesn't affect the UI thread
    update_unread_count("room1", 999);
    println!("[Background Thread] Updated count (but UI thread won't see it!)");
}

fn main() {
    println!("=== Demonstrating Thread-Local UI Safety Problems ===\n");

    // Spawn UI thread
    let ui_handle = thread::spawn(simulate_ui_thread);

    // Spawn background thread that shouldn't access UI data
    let bg_handle = thread::spawn(simulate_background_thread);

    ui_handle.join().unwrap();
    bg_handle.join().unwrap();

    println!("\n=== Problems Demonstrated ===");
    println!("1. No compile-time error when accessing UI data from background thread");
    println!("2. Misleading use of Mutex for single-threaded data");
    println!("3. Confusing behavior: each thread gets its own thread_local storage");
    println!("4. Silent bugs: background thread's changes don't affect UI thread");
    println!("5. No clear API contract about which thread should call these functions");
}
