# Thread-Local UI 数据安全：使用见证者类型实现编译时线程安全

## 01. 提交者展示名称
TigerInYourDream

## 02. 微信联系方式
[请在实际提交时填写]

## 03. 个人邮箱
[请在实际提交时填写]

## 04. 问题概括
如何在 Rust GUI 应用中安全地管理线程局部 UI 状态，使用类型系统在编译时防止跨线程数据访问

## 05. 问题行业描述
GUI/桌面应用程序开发 - 特别是多线程桌面应用，UI 状态必须保留在主线程上，而后台任务处理网络 I/O 和数据处理。这种模式在 Matrix 客户端、聊天应用程序以及任何具有异步后台工作的 GUI 应用中都很常见。

## 06. 问题分类
- **主要**: 从其它语言中带来的坏习惯/如何写地道Rust代码
- **次要**: 异步/并发/竞态条件相关问题
- **相关**: trait的使用, 设计模式相关

## 07. 问题背景

在使用 Rust 构建 GUI 应用程序时，我们经常需要维护**只应从主 UI 线程访问**的状态。从 JavaScript（单线程）等语言迁移过来，或者习惯于在 Go/C++ 中到处使用互斥锁的开发者，可能会天真地使用 `thread_local!` 存储而没有适当的保护措施。

挑战产生的原因：
1. **线程局部存储是每线程的**：每个线程都有自己的副本，导致数据不一致
2. **没有编译时强制**：访问线程局部数据的函数可以从任何线程调用
3. **误导性的同步**：对单线程数据使用 `Mutex` 暗示它用于多线程访问
4. **静默 bug**：后台线程访问仅供 UI 使用的数据不会导致编译错误，但访问的是不同的存储

这个问题出现在 [Robrix Matrix 客户端](https://github.com/project-robius/robrix)中，当管理邀请房间、头像缓存和用户资料数据时，这些数据应该只由 UI 线程修改。

**真实场景**：
- UI 线程维护聊天室列表
- 后台线程从服务器获取新房间数据
- 后台线程尝试更新房间列表
- **BUG**：更新进入后台线程的 thread_local 存储，UI 永远看不到！

## 08. 问题描述

### 我想要实现什么功能
一个 UI 数据缓存（房间、头像、用户资料），具有以下特点：
- 只能从 UI 线程访问
- 使用适合单线程的同步原语
- **不可能**意外地从后台线程访问
- API 清晰地传达线程安全要求

### 遇到了什么困难
1. **缺乏编译时保证**：单独使用 `thread_local!` 不能阻止后台线程调用 UI 函数

2. **错误的同步原语**：对线程局部数据使用 `Mutex<T>` 是误导性的 - 它暗示多线程访问，但我们实际上想要单线程的 `RefCell<T>`

3. **API 契约不清晰**：像 `fn get_room(id: &str)` 这样的函数没有表明它们应该只从 UI 线程调用

4. **令人困惑的行为**：当后台线程访问 thread_local 存储时，它们获得**独立的存储**，导致数据一致性噩梦

### 你最困惑的地方是什么
代码**可以编译和运行**而没有错误，但行为不正确：
```rust
// UI 线程
add_room(room1);  // 添加到 UI 线程的存储

// 后台线程
add_room(room2);  // 编译通过！但添加到不同的存储
get_room(room1);  // 返回 None - 不同的存储！
```

这种静默失败模式极其危险且难以调试。

## 09. 问题代码或问题详细描述

```rust
use std::{
    collections::HashMap,
    sync::Mutex,
    thread,
};

// ❌ PROBLEM 1: Using Mutex for single-threaded data
thread_local! {
    static ROOM_CACHE: Mutex<HashMap<String, RoomData>> = Mutex::new(HashMap::new());
}

#[derive(Clone, Debug)]
pub struct RoomData {
    pub id: String,
    pub name: String,
    pub unread_count: u32,
}

// ❌ PROBLEM 2: No compile-time guarantee this is called from UI thread
pub fn add_room(room: RoomData) {
    ROOM_CACHE.with(|cache| {
        cache.lock().unwrap().insert(room.id.clone(), room);
    });
}

// ❌ PROBLEM 3: Background threads can call this - compiles without error!
pub fn get_room(room_id: &str) -> Option<RoomData> {
    ROOM_CACHE.with(|cache| {
        cache.lock().unwrap().get(room_id).cloned()
    })
}

fn background_thread() {
    // This compiles but accesses DIFFERENT thread_local storage!
    let room = get_room("room1");  // Returns None even if room1 exists on UI thread
}
```

## 10. 错误信息

**没有编译时错误！**这就是问题所在 - 代码可以编译和运行，但行为不正确。

运行时行为显示数据不一致：
```
[UI Thread] Added room1
[UI Thread] Room: Some(RoomData { id: "room1", ... })
[Background Thread] Looking for room1
[Background Thread] Room: None  // ← 错误！它存在，但在不同的存储中
```

**概念错误**：每个线程都有自己的 `thread_local!` 存储，所以后台线程看不到 UI 线程的数据，即使 API 暗示它们应该看到。

## 11. 解决问题的过程

### 初步尝试

**尝试 1：使用 Arc<Mutex<T>> 与 static**
```rust
static ROOM_CACHE: Arc<Mutex<HashMap<...>>> = ...;
```
- ❌ **问题**：需要 `lazy_static!` 或 `OnceLock`，增加运行时开销
- ❌ **问题**：仍然允许后台线程访问 UI 数据（现在它们可以看到了，这也是错误的！）

**尝试 2：添加注释和文档**
```rust
/// **警告：只能从 UI 线程调用！**
pub fn get_room(room_id: &str) -> Option<RoomData>
```
- ❌ **问题**：文档容易被忽略，不由编译器强制执行

**尝试 3：研究 Robrix 代码库模式**
- ✅ **发现**：在 `rooms_list.rs` 中找到了 `Cx` 参数模式：
  ```rust
  pub fn get_invited_rooms(_cx: &mut Cx) -> Rc<RefCell<...>>
  ```
- ✅ **洞察**：`_cx` 参数充当"见证者"或"能力"，证明我们在 UI 线程上！

### 最终解决方案：见证者类型模式

关键洞察是创建一个代表"在 UI 线程上的证明"的类型，并要求它作为参数：

```rust
// Witness type - represents proof we're on UI thread
pub struct UiContext {
    _private: (),  // Private field prevents external construction
}

impl UiContext {
    pub fn new() -> Self {
        UiContext { _private: () }
    }
}

// All UI functions require the witness
pub fn add_room(_ui: &UiContext, room: RoomData) {
    // Implementation
}
```

**为什么这样有效**：
1. **编译时强制**：没有 `UiContext` 就不能调用 `add_room`
2. **清晰的 API**：函数签名记录了线程要求
3. **零运行时成本**：`UiContext` 是零大小的，编译为无
4. **符合 Rust 惯例**：利用类型系统实现安全

## 12. 解决方案（见附件 zip）

解决方案使用三个关键模式：

### 模式 1：见证者类型
```rust
#[derive(Clone)]
pub struct UiContext {
    _private: (),
}
```
- 充当"能力"的零大小类型
- 只能通过调用 `UiContext::new()` 创建
- 不是 `Send` 或 `Sync`，因此不能发送到其他线程

### 模式 2：使用 RefCell 的内部可变性
```rust
thread_local! {
    static ROOM_CACHE: Rc<RefCell<HashMap<String, RoomData>>> =
        Rc::new(RefCell::new(HashMap::new()));
}
```
- `RefCell` 而不是 `Mutex` - 适合单线程访问
- `Rc` 而不是 `Arc` - 不是线程安全的，进一步防止误用
- 比 `Mutex` 更便宜的运行时检查

### 模式 3：见证者保护的 API
```rust
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
```
- 每个函数都需要 `&UiContext`
- 编译器在每个调用点强制执行
- API 清楚地传达意图

### 优势
- ✅ **类型安全**：不可能从错误的线程调用（不会编译）
- ✅ **零成本**：`UiContext` 完全优化掉
- ✅ **清晰**：函数签名记录线程要求
- ✅ **惯用**：按预期使用 Rust 的类型系统
- ✅ **可维护**：新开发者无法误用 API

## 13. 补充说明

### 帮助我的资源
- [Rust API Guidelines - Witness Pattern](https://rust-lang.github.io/api-guidelines/)
- [Jon Gjengset 的 "Rust for Rustaceans"](https://rust-for-rustaceans.com/) - API 设计章节
- [Robrix 源代码](https://github.com/project-robius/robrix) - `rooms_list.rs`、`avatar_cache.rs` 中的真实用法

### 考虑过的替代方法

**1. 运行时线程 ID 检查**
```rust
static UI_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();

pub fn add_room(room: RoomData) {
    assert_eq!(thread::current().id(), *UI_THREAD_ID.get().unwrap());
    // ...
}
```
- ❌ 每次调用都有运行时成本
- ❌ panic 而不是编译错误
- ❌ 容易忘记检查

**2. Sealed Trait 模式**
```rust
pub trait UiThread: private::Sealed {}
pub fn add_room<T: UiThread>(proof: &T, room: RoomData)
```
- ✅ 也很有效
- ❌ API 更复杂
- ❌ 新手更难理解

**3. 带泛型的类型状态模式**
```rust
struct RoomCache<State> {
    // ...
}
impl RoomCache<UiThreadState> {
    pub fn add_room(&mut self, room: RoomData) { }
}
```
- ✅ 非常类型安全
- ❌ 对于这个用例过于复杂
- ❌ 更难与现有代码集成

### 社区反馈
这种模式在 Rust GUI 社区中得到了很好的反响。它被用于：
- **Robrix**：Matrix 客户端 UI 状态管理
- **Makepad**：UI 框架的上下文传递
- **egui**：隐式上下文参数

### 相关模式
- **Phantom Types**：类似的编译时强制
- **Capability-Based Security**：见证者类型作为不可伪造的能力
- **Effect Systems**：在类型中跟踪副作用

---

**zip 中包含的文件**：
- `broken-example/`：演示问题的错误实现
- `correct-example/`：带测试的完整解决方案
- 两者都通过 `cargo check`、`cargo test`、`cargo clippy`

**环境信息**：
- Rust 版本：1.85.0 (stable)
- 工具链：aarch64-apple-darwin
- 操作系统：macOS 15.1.1
- 架构：Apple Silicon (ARM64)
