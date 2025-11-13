//! Sealed Trait Pattern - 密封 trait 模式示例
//!
//! 这个模式用于防止外部 crate 实现某个 trait，
//! 同时保持 trait 本身是 public 的（可以被使用，但不能被实现）

// ========== 基本的 Sealed Trait 实现 ==========

mod private {
    // 这个模块是私有的，外部无法访问
    pub trait Sealed {}

    // 只有在这个模块内部才能实现 Sealed
    impl Sealed for super::UiThreadMarker {}
}

// 公开的 trait，但继承自私有的 Sealed trait
// 外部代码可以使用这个 trait，但无法实现它
pub trait UiThread: private::Sealed {
    // trait 方法...
}

// 我们自己的类型可以实现 UiThread（因为它已经实现了 Sealed）
pub struct UiThreadMarker;

impl UiThread for UiThreadMarker {
    // 实现细节...
}

// ========== 用于 UI 线程安全的完整示例 ==========

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct RoomData {
    pub id: String,
    pub name: String,
}

thread_local! {
    static ROOM_CACHE: Rc<RefCell<HashMap<String, RoomData>>> =
        Rc::new(RefCell::new(HashMap::new()));
}

// ✅ 方案 1: Sealed Trait 模式
mod sealed_approach {
    use super::*;

    mod private {
        pub trait Sealed {}
        impl Sealed for super::UiThreadToken {}
    }

    /// 只有实现了这个 trait 的类型才能访问 UI 函数
    /// 由于 Sealed，外部无法实现这个 trait
    pub trait UiThread: private::Sealed {}

    /// UI 线程令牌 - 只有这个类型实现了 UiThread
    pub struct UiThreadToken {
        _private: (),
    }

    impl UiThread for UiThreadToken {}

    impl UiThreadToken {
        pub fn new() -> Self {
            UiThreadToken { _private: () }
        }
    }

    /// 使用泛型 + trait bound 来约束
    /// 只有实现了 UiThread 的类型才能调用这个函数
    pub fn add_room<T: UiThread>(_proof: &T, room: RoomData) {
        ROOM_CACHE.with(|cache| {
            cache.borrow_mut().insert(room.id.clone(), room);
        });
    }

    pub fn get_room<T: UiThread>(_proof: &T, room_id: &str) -> Option<RoomData> {
        ROOM_CACHE.with(|cache| {
            cache.borrow().get(room_id).cloned()
        })
    }
}

// ✅ 方案 2: 简单的 Witness Type（我们之前使用的）
mod witness_approach {
    use super::*;

    /// 简单的见证者类型
    pub struct UiContext {
        _private: (),
    }

    impl UiContext {
        pub fn new() -> Self {
            UiContext { _private: () }
        }
    }

    /// 直接要求具体的类型
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
}

// ========== 使用示例对比 ==========

fn main() {
    println!("=== Sealed Trait Pattern vs Witness Type Pattern ===\n");

    // 使用 Sealed Trait 方式
    {
        use sealed_approach::*;

        let token = UiThreadToken::new();

        add_room(&token, RoomData {
            id: "room1".to_string(),
            name: "General".to_string(),
        });

        let room = get_room(&token, "room1");
        println!("[Sealed Trait] Room: {:?}", room);
    }

    // 使用简单 Witness Type 方式
    {
        use witness_approach::*;

        let ui = UiContext::new();

        add_room(&ui, RoomData {
            id: "room2".to_string(),
            name: "Random".to_string(),
        });

        let room = get_room(&ui, "room2");
        println!("[Witness Type] Room: {:?}", room);
    }

    println!("\n=== 模式对比 ===");
    println!("\n【Sealed Trait Pattern】");
    println!("✅ 优点:");
    println!("  • 更灵活 - 可以为多个类型实现 trait");
    println!("  • API 更通用 - 使用泛型 <T: UiThread>");
    println!("  • 防止外部实现 - 通过 private::Sealed 限制");
    println!("❌ 缺点:");
    println!("  • 更复杂 - 需要额外的 mod private");
    println!("  • 对新手不友好 - 理解成本高");
    println!("  • 函数签名更长 - pub fn add_room<T: UiThread>(...)");

    println!("\n【Witness Type Pattern】");
    println!("✅ 优点:");
    println!("  • 简单直接 - 只需要一个结构体");
    println!("  • 易于理解 - 新手友好");
    println!("  • 清晰的 API - pub fn add_room(_ui: &UiContext, ...)");
    println!("  • 零运行时开销 - 类型擦除");
    println!("❌ 缺点:");
    println!("  • 不太灵活 - 只能用一个具体类型");
    println!("  • 外部可以创建 UiContext（虽然访问的是不同的 thread_local）");
}

// ========== 为什么 Sealed Trait 是"密封"的 ==========

// 假设有外部代码尝试实现 UiThread:
/*
// ❌ 这段代码无法编译！
pub struct MyFakeUiThread;

impl sealed_approach::UiThread for MyFakeUiThread {}
//                                 ^^^^^^^^^^^^^^^^
// ERROR: the trait `private::Sealed` is not implemented for `MyFakeUiThread`
//
// 因为 MyFakeUiThread 无法实现 private::Sealed（private 模块在外部不可见）
// 所以也就无法实现 UiThread（因为 UiThread: private::Sealed）
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sealed_trait_approach() {
        use sealed_approach::*;

        let token = UiThreadToken::new();

        add_room(&token, RoomData {
            id: "test".to_string(),
            name: "Test".to_string(),
        });

        assert!(get_room(&token, "test").is_some());
    }

    #[test]
    fn test_witness_type_approach() {
        use witness_approach::*;

        let ui = UiContext::new();

        add_room(&ui, RoomData {
            id: "test".to_string(),
            name: "Test".to_string(),
        });

        assert!(get_room(&ui, "test").is_some());
    }
}
