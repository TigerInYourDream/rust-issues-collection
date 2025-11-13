# Thread-Local UI 数据安全 - 错误示例

本示例演示了管理线程局部 UI 状态的**错误**方式，可能导致微妙的 bug 和混乱。

## 演示的问题

1. **没有编译时强制约束**：函数可以从任何线程调用而不会报错
2. **误导性的同步原语**：对只应在单线程访问的数据使用 `Mutex`
3. **API 不清晰**：函数签名没有传达线程要求
4. **静默 bug**：后台线程访问不同的 `thread_local!` 存储，导致数据不一致

## 出了什么问题

```rust
// ❌ 没有指示这应该只从 UI 线程调用
pub fn add_room(room: RoomData) {
    ROOM_CACHE.with(|cache| {
        cache.lock().unwrap().insert(room.id.clone(), room);
    });
}

// 后台线程可以调用这个 - 编译没有错误！
fn background_task() {
    add_room(some_room); // 访问的是**不同**的 thread_local 存储
}
```

## 为什么这段代码能编译但是有问题

- 每个线程都有自己的 `thread_local!` 存储
- 后台线程可以调用 UI 函数而不会产生编译错误
- 后台线程所做的修改不会影响 UI 线程
- 这会导致令人困惑且难以追踪的 bug

## 运行这个示例

```bash
cargo run
```

你会看到输出显示后台线程如何能够访问 API，但使用的是完全独立的数据。

## 修复方法

查看 `correct-example` 目录了解使用见证者类型(witness types)的正确解决方案。

## 环境信息

- **Rust 版本**: 1.85.0 (stable)
- **工具链**: aarch64-apple-darwin
- **操作系统**: macOS 15.1.1 (25B7)
- **架构**: Apple Silicon (ARM64)

## 许可证

MIT OR Apache-2.0
