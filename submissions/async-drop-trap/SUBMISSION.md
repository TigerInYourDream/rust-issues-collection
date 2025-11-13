# Rust数据标注信息提交 - 异步资源的 Drop 陷阱

## **04** 问题概括

Drop trait 是同步的，但异步资源需要异步清理——如何正确处理异步资源的生命周期管理

## **05** 问题行业描述

异步应用开发，特别是需要优雅关闭的后台服务、数据库连接池、网络服务器、GUI应用中的后台任务等场景

## **06** 问题分类

[X] 异步/并发/竞态条件相关问题
[X] 设计模式相关

## **07** 问题背景

在开发 Robrix Matrix 客户端时，我们需要管理后台异步任务，这些任务持有重要资源（如数据库连接、临时文件、网络连接）。当应用需要退出登录或关闭时，我们遇到了一个根本性问题：

Rust 的 `Drop` trait 是**同步**的（`fn drop(&mut self)`），但清理异步资源需要**异步**操作（`.await`）。这导致了几个严重问题：

1. **简单地调用 `abort()` 会导致资源泄漏** - 任务中的清理代码永远不会执行
2. **无法在 Drop 中使用 `.await`** - Rust 不支持 `async fn drop()`
3. **在 Drop 中使用 `block_on` 可能死锁** - 如果 Drop 在异步上下文中被调用
4. **手动清理不可靠** - 资源可能仍被中止的任务持有

这个问题不仅影响 Robrix，在任何使用 tokio/async-std 的异步应用中都会遇到，特别是：

- Web 服务器需要优雅关闭时
- 数据库连接池需要在关闭前提交事务时
- GUI 应用中后台任务持有 UI 资源时
- 任何需要确保数据一致性的异步清理场景

## **08** 问题描述

### 想要实现的功能

一个后台工作线程（Background Worker），具有以下特点：

1. 在后台异步运行，处理数据并写入临时文件
2. 当需要关闭时，能够**优雅地完成清理工作**：
   - 完成正在进行的操作
   - 刷新缓冲区
   - 删除临时文件
   - 释放资源
3. 保证清理代码一定会执行，不会泄漏资源

### 遇到的困难

**方法1：在 Drop 中直接 abort 任务**

```rust
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        self.task_handle.abort();  // ❌ 清理代码不会执行！
    }
}
```

问题：`abort()` 会立即终止任务，任务中的清理代码（如删除临时文件）永远不会运行。

**方法2：在 Drop 中使用 block_on**

```rust
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        tokio::runtime::Handle::current()
            .block_on(async { self.task_handle.await.ok() });
        // ❌ 如果 Drop 在 async 上下文中被调用，这会 panic 或死锁
    }
}
```

问题：`block_on` 不能在异步上下文中调用，会导致 panic 或死锁。

**方法3：提供 async shutdown 但无法强制使用**

```rust
async fn shutdown(self) { /* ... */ }
```

问题：如果开发者忘记调用 `shutdown()`，直接 drop，仍然会泄漏资源。

### 最困惑的地方

为什么 Rust 没有 `async fn drop()`？这似乎是类型系统的一个基本限制：

- Drop 必须是同步的，因为它可能在任何时候被调用（包括 panic unwinding）
- 但异步资源的清理天然需要异步操作
- 这两者的矛盾导致了设计上的困境

## **09** 问题代码或问题详细描述

```rust
use tokio::task::JoinHandle;
use std::path::PathBuf;
use std::fs::File;

/// A background worker that processes data
struct BackgroundWorker {
    task_handle: JoinHandle<()>,
    temp_file: PathBuf,
}

impl BackgroundWorker {
    fn new(temp_file: PathBuf) -> Self {
        let file_path = temp_file.clone();

        let task_handle = tokio::spawn(async move {
            let mut file = File::create(&file_path).unwrap();

            // Do some work...
            for i in 0..10 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                writeln!(file, "Processing item {}", i).unwrap();
            }

            // ⚠️ CRITICAL: Cleanup code that should run
            drop(file);
            std::fs::remove_file(&file_path).ok();  // Clean up temp file
            println!("Cleanup complete!");
        });

        Self { task_handle, temp_file }
    }
}

// ❌ PROBLEM: Synchronous Drop with async resources
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        // Issue 1: abort() kills task immediately, cleanup never runs
        self.task_handle.abort();

        // Issue 2: Cannot use .await in Drop
        // self.task_handle.await.ok();  // ❌ Compile error

        // Issue 3: block_on can deadlock if called from async context
        // Handle::current().block_on(async { self.task_handle.await });  // ❌ Panic
    }
}

#[tokio::main]
async fn main() {
    let worker = BackgroundWorker::new("/tmp/temp.log".into());

    tokio::time::sleep(Duration::from_millis(300)).await;

    // When worker is dropped, the task is aborted
    // and the cleanup code NEVER runs!
    drop(worker);
}
```

## **10** 错误信息

这不是一个编译错误，而是一个**运行时行为问题**：

**现象1：资源泄漏**

```
[Worker] Starting background task...
[Worker] Processed item 0
[Worker] Processed item 1
[Worker] Processed item 2
[Drop] Aborting task
✗ Cleanup code never executed
✗ Temporary file not removed: /tmp/temp.log
✗ Buffer not flushed - data loss
```

**现象2：如果尝试在 Drop 中使用 block_on**

```
thread 'main' panicked at 'Cannot start a runtime from within a runtime'
```

**现象3：数据一致性问题**
在生产环境中，这会导致：

- 数据库事务未提交
- 文件未刷新（部分写入）
- 网络连接未正确关闭
- 临时资源泄漏

## **11** 解决问题的过程

### 尝试过的方案

**方案1：使用 ManuallyDrop**

```rust
struct BackgroundWorker {
    task_handle: ManuallyDrop<JoinHandle<()>>,
}
```

- 问题：需要手动管理内存，容易忘记调用清理
- 问题：没有解决异步清理的根本问题

**方案2：使用 Drop guard 模式**

```rust
struct ShutdownGuard {
    worker: BackgroundWorker,
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        panic!("Must call shutdown() explicitly!");
    }
}
```

- 问题：panic 在 Drop 中是不好的实践
- 问题：仍然需要异步清理

**方案3：研究 tokio 和其他异步库的实践**

- **发现**：tokio 的 `Runtime` 提供了 `shutdown_background()` 和 `shutdown_timeout()`
- 发现**：许多异步库都提供显式的 async shutdown 方法
- 洞察**：Drop 应该是安全网，而不是主要清理机制

### 突破点

关键洞察来自于 Robrix 的 logout 流程实现：

1. 不依赖 Drop 进行异步清理
2. 提供显式的 `async fn shutdown()` 方法
3. 使用信号机制（watch channel / CancellationToken）通知任务关闭
4. 等待清理完成后再 join 任务
5. Drop 只作为安全网，警告用户忘记调用 shutdown

### 最终解决方案

```rust
use tokio::sync::{watch, Notify};

struct BackgroundWorker {
    task_handle: Option<JoinHandle<()>>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_complete: Arc<Notify>,
}

impl BackgroundWorker {
    fn new() -> Self {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let shutdown_complete = Arc::new(Notify::new());
        let shutdown_complete_clone = shutdown_complete.clone();

        let task_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Monitor shutdown signal
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    // Do work
                    _ = do_work() => {}
                }
            }

            // Cleanup code always runs
            perform_cleanup().await;
            shutdown_complete_clone.notify_one();
        });

        Self {
            task_handle: Some(task_handle),
            shutdown_tx,
            shutdown_complete,
        }
    }

    // Explicit async shutdown method
    async fn shutdown(mut self) {
        // 1. Signal shutdown
        self.shutdown_tx.send(true).ok();

        // 2. Wait for cleanup to complete
        self.shutdown_complete.notified().await;

        // 3. Join task
        if let Some(handle) = self.task_handle.take() {
            handle.await.ok();
        }
    }
}

// Drop as safety net
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        if let Some(handle) = &self.task_handle {
            if !handle.is_finished() {
                eprintln!("WARNING: Dropped without shutdown()!");
                handle.abort();
            }
        }
    }
}
```

## **12** 解决方案

见附件 `async-drop-trap.zip`，包含：

- `broken-example/`：演示问题的错误实现
- `correct-example/`：演示正确的解决方案

两个示例都通过了 `cargo check`、`cargo test` 和 `cargo clippy`。

环境信息：

- Rust 版本：1.85.0 (stable)
- 工具链：aarch64-apple-darwin
- 操作系统：macOS 15.1.1
- 架构：Apple Silicon (ARM64)

许可证：MIT

## **13** 补充说明

### 关键设计模式

这个解决方案组合了几个关键模式：

**1. 显式异步清理方法**

```rust
async fn shutdown(self) -> Result<()>
async fn shutdown_with_timeout(self, timeout: Duration) -> Result<()>
```

**2. 信号机制**

- `tokio::sync::watch` - 用于发送关闭信号
- `tokio::sync::Notify` - 用于等待清理完成
- 或者使用 `tokio_util::sync::CancellationToken`

**3. Drop 安全网**

```rust
impl Drop {
    // 检测是否忘记调用 shutdown
    // 发出警告
    // 作为最后手段进行 abort
}
```

### 相关文档

- Tokio graceful shutdown: https://tokio.rs/tokio/topics/shutdown
- tokio-util CancellationToken: https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html
- Async drop tracking issue: https://github.com/rust-lang/rust/issues/126482

### 与 Robrix 实践的关联

这个解决方案直接源于 Robrix Matrix 客户端的退出登录流程：

1. 使用状态机管理退出流程
2. 显式调用 async shutdown 方法
3. 等待所有清理完成
4. 最后才关闭 tokio runtime

参见本仓库的 `rust-logout-issue` 示例。

### 适用场景

这个模式适用于任何需要异步清理的场景：

- **数据库连接池** - 提交事务、关闭连接
- **网络服务** - 完成请求、优雅断开连接
- **文件操作** - 刷新缓冲区、同步到磁盘
- **后台任务** - 完成正在处理的工作
- **GUI 应用** - 保存状态、清理临时文件

### 为什么不能有 async fn drop()

Rust 核心团队正在讨论这个问题（RFC tracking issue #126482），但面临技术挑战：

1. Drop 可能在 panic unwinding 时调用，不能是 async
2. Drop 的调用时机是确定的（RAII），async 会引入不确定性
3. 可能导致性能问题（每个 drop 都需要 runtime）

当前的最佳实践是：**提供显式的 async cleanup 方法，Drop 只作为安全网。**

### 其他语言的对比

- **C++**: 没有异步，RAII 在析构函数中同步清理
- **Go**: 有 `defer`，但没有确定性析构
- **C#**: 有 `IAsyncDisposable` 接口，需要显式调用 `DisposeAsync()`
- **Rust**: 选择了显式异步清理 + Drop 安全网的组合

Rust 的方案在编译时安全性和运行时灵活性之间取得了良好平衡。
