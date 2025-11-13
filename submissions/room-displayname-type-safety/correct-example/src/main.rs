// CORRECT EXAMPLE: Using enum to represent room display name states
// Type-safe approach that prevents bugs at compile time

use std::collections::HashMap;
use std::fmt;

/// Strongly-typed enum representing all possible room display name states
/// This matches the Matrix SDK's RoomDisplayName enum
#[derive(Debug, Clone, PartialEq)]
enum RoomDisplayName {
    /// Room has a proper name
    Named(String),
    /// Room name is calculated from members (DM)
    Calculated(String),
    /// Room has an alias but no name
    Aliased(String),
    /// Room has no name, was empty previously (tombstoned)
    EmptyWas(String),
    /// Room has no name at all
    Empty,
}

impl RoomDisplayName {
    /// Convert to displayable string for UI
    fn to_display_string(&self) -> String {
        match self {
            RoomDisplayName::Named(name) => name.clone(),
            RoomDisplayName::Calculated(name) => name.clone(),
            RoomDisplayName::Aliased(alias) => alias.clone(),
            RoomDisplayName::EmptyWas(prev) => format!("Empty (was {})", prev),
            RoomDisplayName::Empty => "Unnamed Room".to_string(),
        }
    }

    /// Check if this is a placeholder/empty name
    fn is_placeholder(&self) -> bool {
        matches!(self, RoomDisplayName::Empty | RoomDisplayName::EmptyWas(_))
    }
}

impl fmt::Display for RoomDisplayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

/// Represents a Matrix room's basic information
#[derive(Debug, Clone)]
struct RoomInfo {
    room_id: String,
    /// Now we use Option<RoomDisplayName> which clearly distinguishes:
    /// - None: Name not loaded yet / Room state not synced
    /// - Some(RoomDisplayName::Empty): Room explicitly has no name
    /// - Some(RoomDisplayName::Named(s)): Room has a proper name
    room_name: Option<RoomDisplayName>,
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

    /// Updates the room name with type-safe handling
    fn update_room_name(&mut self, room_id: String, new_name: RoomDisplayName) {
        if let Some(room) = self.rooms.get_mut(&room_id) {
            // For invited rooms, skip placeholder updates
            // because we might have initially set name to None,
            // but SDK's cached name might already reflect the update
            if new_name.is_placeholder() {
                println!("  [SKIP] Ignoring placeholder name update for {}", room_id);
                return;
            }

            println!("  [UPDATE] Setting name to: {:?}", new_name);
            room.room_name = Some(new_name);
        }
    }

    /// Gets displayable room name for UI
    fn get_display_name(&self, room_id: &str) -> String {
        self.rooms
            .get(room_id)
            .and_then(|room| room.room_name.as_ref())
            .map(|name| name.to_display_string())
            .unwrap_or_else(|| "Invite to Unnamed Room".to_string())
    }
}

fn main() {
    let mut rooms = RoomsList::new();

    // Add a room with a proper name
    rooms.rooms.insert(
        "!abc:matrix.org".to_string(),
        RoomInfo {
            room_id: "!abc:matrix.org".to_string(),
            room_name: Some(RoomDisplayName::Named("General Chat".to_string())),
        },
    );

    // Add an invited room without name loaded yet
    rooms.rooms.insert(
        "!xyz:matrix.org".to_string(),
        RoomInfo {
            room_id: "!xyz:matrix.org".to_string(),
            room_name: None,  // Clear: name not loaded yet
        },
    );

    // Add a room with explicitly empty name
    rooms.rooms.insert(
        "!def:matrix.org".to_string(),
        RoomInfo {
            room_id: "!def:matrix.org".to_string(),
            room_name: Some(RoomDisplayName::Empty),  // Clear: has no name
        },
    );

    println!("=== Initial State ===");
    println!("Room 1: {}", rooms.get_display_name("!abc:matrix.org"));
    println!("Room 2: {}", rooms.get_display_name("!xyz:matrix.org"));
    println!("Room 3: {}", rooms.get_display_name("!def:matrix.org"));

    println!("\n=== Trying to update with placeholder (Empty) ===");
    // This update will be skipped - preventing bugs!
    rooms.update_room_name("!abc:matrix.org".to_string(), RoomDisplayName::Empty);
    println!("Room 1 after empty update: {}", rooms.get_display_name("!abc:matrix.org"));

    println!("\n=== Valid update with proper name ===");
    rooms.update_room_name(
        "!xyz:matrix.org".to_string(),
        RoomDisplayName::Named("Private Chat".to_string())
    );
    println!("Room 2 after proper update: {}", rooms.get_display_name("!xyz:matrix.org"));

    println!("\n=== Benefits ===");
    println!("✅ Type system enforces clear semantics");
    println!("✅ Cannot accidentally confuse empty string with None");
    println!("✅ Can skip placeholder updates for invited rooms");
    println!("✅ UI always gets correct display text");
    println!("✅ Matches Matrix SDK's RoomDisplayName type");
}

// Key Takeaways:
// 1. Use enums instead of String when a value has distinct semantic states
// 2. Type safety prevents bugs that would only appear at runtime
// 3. Pattern matching makes intent explicit and catches missing cases
// 4. Aligning internal types with SDK types reduces conversion errors
