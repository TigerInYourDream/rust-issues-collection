// BROKEN EXAMPLE: Using String for room display names
// This compiles but leads to runtime bugs and unclear semantics

use std::collections::HashMap;

/// Represents a Matrix room's basic information
#[derive(Debug, Clone)]
struct RoomInfo {
    room_id: String,
    /// Problem: Using Option<String> doesn't distinguish between:
    /// - A room with no name set
    /// - A room whose name hasn't been loaded yet
    /// - A room with an empty string as name
    room_name: Option<String>,
}

/// Simulates a rooms list manager
struct RoomsList {
    rooms: HashMap<String, RoomInfo>,
}

impl RoomsList {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    /// Updates the room name - but what if name is empty string?
    fn update_room_name(&mut self, room_id: String, new_name: String) {
        if let Some(room) = self.rooms.get_mut(&room_id) {
            // Problem: Cannot distinguish between:
            // 1. User explicitly set empty name
            // 2. Name not loaded yet
            // 3. Room has no name
            room.room_name = if new_name.is_empty() {
                None  // Is this correct? We lose information!
            } else {
                Some(new_name)
            };
        }
    }

    /// Gets displayable room name for UI
    fn get_display_name(&self, room_id: &str) -> String {
        self.rooms
            .get(room_id)
            .and_then(|room| room.room_name.clone())
            .unwrap_or_else(|| "Unknown Room".to_string())
    }
}

fn main() {
    let mut rooms = RoomsList::new();

    // Add a room with a name
    rooms.rooms.insert(
        "!abc:matrix.org".to_string(),
        RoomInfo {
            room_id: "!abc:matrix.org".to_string(),
            room_name: Some("General Chat".to_string()),
        },
    );

    // Add a room without a name (invited room, not loaded yet)
    rooms.rooms.insert(
        "!xyz:matrix.org".to_string(),
        RoomInfo {
            room_id: "!xyz:matrix.org".to_string(),
            room_name: None,  // But WHY is it None? Not loaded? Or truly no name?
        },
    );

    println!("Room 1: {}", rooms.get_display_name("!abc:matrix.org"));
    println!("Room 2: {}", rooms.get_display_name("!xyz:matrix.org"));

    // Problem scenario: Update with empty string
    rooms.update_room_name("!abc:matrix.org".to_string(), "".to_string());
    println!("Room 1 after empty update: {}", rooms.get_display_name("!abc:matrix.org"));

    // BUG: We cannot distinguish between:
    // - A room that truly has no name
    // - A room whose name is being loaded
    // - A room with empty string as name (which Matrix SDK might return)
    // This leads to incorrect UI display for invited rooms!
}

// Compiler doesn't catch these issues because everything compiles fine!
// But at runtime, we get incorrect behavior for invited rooms.
