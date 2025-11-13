# Thread-Local UI 数据安全 - 正确解决方案

本示例演示了使用类型系统保证来管理线程局部 UI 状态的**正确**方式。

## 解决的问题

在 GUI 应用程序中，某些数据应该只能从主 UI 线程访问。单纯使用 `thread_local!` 存储可能导致：
- 没有编译时的线程安全强制约束
- API 不清晰，无法传达线程要求
- 误用同步原语（例如，对单线程数据使用 `Mutex`）

## 解决方案：见证者类型模式 (Witness Type Pattern)

本示例演示了使用"见证者"类型（`UiContext`）在编译时强制线程安全：

```rust
// 见证者类型 - 代表我们在 UI 线程上的"证明"
pub struct UiContext {
    _private: (),  // 私有字段防止外部构造
}

// 所有 UI 函数都需要见证者
pub fn add_room(_ui: &UiContext, room: RoomData) {
    // 实现
}
```

## 核心优势

1. **编译时安全**：不能在没有 `UiContext` 的情况下调用 `add_room`
2. **合适的原语**：使用 `RefCell`（而非 `Mutex`）实现单线程内部可变性
3. **清晰的 API 契约**：函数签名明确记录了线程要求
4. **零运行时开销**：`UiContext` 是零大小类型，编译后不占空间
5. **符合 Rust 惯例**：利用类型系统提供安全保证

## 环境信息

- **Rust 版本**: 1.85.0 (stable)
- **工具链**: aarch64-apple-darwin
- **操作系统**: macOS 15.1.1 (25B7)
- **架构**: Apple Silicon (ARM64)

## 构建和运行

```bash
# 检查代码是否编译
cargo check

# 运行测试
cargo test

# 运行演示
cargo run

# 运行 clippy 检查
cargo clippy
```

## 真实应用案例

这种模式在 [Robrix](https://github.com/project-robius/robrix) Matrix 客户端中被使用：

- `src/home/rooms_list.rs`: 线程局部的邀请房间存储
- `src/avatar_cache.rs`: 线程局部的头像缓存
- `src/profile/user_profile_cache.rs`: 线程局部的用户资料缓存

在这些文件中，像 `get_invited_rooms(_cx: &mut Cx)` 这样的函数使用 `Cx`（上下文）参数作为见证者类型，以保证只能在 UI 线程访问。

## 许可证

MIT OR Apache-2.0
