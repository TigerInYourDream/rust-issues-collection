# Rust数据标注信息提交

## **04** 问题概括

Tokio Runtime 共享的 Arc 与 Mutex 抉择：无法同时实现易于共享和可控关闭

## **05** 问题行业描述

GUI 应用异步运行时开发，特别是 Matrix 客户端中需要优雅退出登录和重启的场景

## **06** 问题分类

[X] 异步/并发/竞态条件相关问题

## **07** 问题背景

在 Robrix Matrix 客户端（基于 Makepad 框架的 GUI 应用）中，我们有一个全局的 Tokio runtime 需要满足以下需求：
1. 在多个组件间共享（时序数据处理 workers、事件处理器、UI 更新等）
2. 在退出登录/重新登录流程中可控地关闭和重启

实现退出登录功能时遇到了挑战。应用需要：
- 停止同步服务
- 执行服务器端登出
- 关闭 Matrix SDK 客户端（它内部使用 rusqlite 和 deadpool 连接池）
- 关闭 Tokio runtime
- 重启一个新的 runtime 供下次登录使用

考虑了两种方案，但都有严重缺陷：
- Arc<Runtime>：易于共享，但无法强制关闭
- Mutex<Option<Runtime>>：可以可控地关闭，但在长时间运行的任务中极难使用

## **08** 问题描述

### 想要实现的功能：
一个全局的 Tokio runtime，需要满足：
1. TSP（时序数据处理）workers 等长时间运行的后台任务能够方便地访问
2. 在退出登录时能够优雅地关闭，确保所有异步任务完成后再终止 runtime
3. 退出登录后能够重启，为下次登录做准备

### 遇到的困难：

**使用 Arc<Runtime> 方案：**
- TSP workers 可以轻松克隆 Arc 并使用
- 但是，当触发退出登录时，没有办法强制所有 Arc 克隆都 drop
- 即使我们 drop 了本地的引用，TSP workers 可能仍然持有克隆
- 无法保证 runtime 在需要时真正关闭
- 存在悬空 Arc 引用指向已关闭 runtime 的风险

**使用 Mutex<Option<Runtime>> 方案：**
- 可以使用 `take()` 强制移除 runtime 进行关闭
- 但是，TSP workers 无法长时间持有 MutexGuard（它不是 Send）
- 必须反复加锁和解锁来执行持续的工作，造成性能开销
- 存在锁竞争和死锁的风险，当多个组件访问 runtime 时
- 代码变得复杂且容易出错

### 最困惑的地方：
为什么 Rust 让同时拥有"易于共享"和"可控的生命周期管理"变得如此困难？这似乎是类型系统中的一个基本权衡：Arc 为了共享而放弃了控制，而 Mutex 提供了控制但牺牲了人机工程学。

## **09** 问题代码或问题详细描述

### Arc 方案的问题：

```rust
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

struct TspWorker {
    runtime: Arc<Runtime>,  // Cloned Arc
}

impl TspWorker {
    fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }

    fn start_processing(&self) {
        // Spawn long-running tasks using the cloned Arc
        self.runtime.spawn(async {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                // Process data...
            }
        });
    }
}

fn main() {
    let runtime = Arc::new(Runtime::new().unwrap());
    TOKIO_RUNTIME.set(runtime.clone()).ok();

    // TSP worker holds an Arc clone
    let worker = TspWorker::new(runtime.clone());
    worker.start_processing();

    // Later, during logout...
    // PROBLEM: How do we force shutdown?
    // Even if we drop 'runtime', the worker still holds a clone!
    drop(runtime);

    // Arc strong count > 0, cannot guarantee shutdown
    // No way to force all clones to release
}
```

### Mutex 方案的问题：

```rust
use std::sync::Mutex;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

struct TspWorker;

impl TspWorker {
    fn start_processing(&self) {
        // PROBLEM 1: Cannot hold MutexGuard across async calls
        let rt_guard = TOKIO_RUNTIME.lock().unwrap();

        if let Some(rt) = rt_guard.as_ref() {
            rt.spawn(async { /* ... */ });
        }

        // Must drop guard immediately
        drop(rt_guard);

        // PROBLEM 2: For continuous work, must lock repeatedly
        loop {
            let rt_guard = TOKIO_RUNTIME.lock().unwrap();  // Lock again!
            if let Some(rt) = rt_guard.as_ref() {
                rt.block_on(async { /* work */ });
            }
            drop(rt_guard);  // Unlock

            // Inefficient, risk of lock contention
        }
    }
}

fn main() {
    *TOKIO_RUNTIME.lock().unwrap() = Some(Runtime::new().unwrap());

    let worker = TspWorker;
    worker.start_processing();

    // Shutdown: Can use take() to force removal
    let mut guard = TOKIO_RUNTIME.lock().unwrap();
    if let Some(rt) = guard.take() {
        rt.shutdown_background();  // Controlled shutdown
    }
}
```

## **10** 错误信息

这不是一个编译错误，而是一个设计困境。两种方案都能编译成功，但存在运行时/架构层面的问题：

**Arc 方案的问题（概念性）：**
```
问题：Runtime 实际上永远不会关闭，因为 Arc 克隆仍然存在
- Arc 强引用计数：3（期望为 0 才能关闭）
- 无法强制所有持有者 drop 它们的引用
- 存在任务继续使用逻辑上已"关闭"的 runtime 的风险
```

**Mutex 方案的问题（概念性）：**
```
问题：在长时间运行的任务中无法人机工程学地使用 runtime
- MutexGuard 不是 Send，无法跨 .await 点持有
- 必须反复加锁/解锁，造成开销
- 如果多个组件同时加锁，存在死锁风险
```

在相关的 `deadpool-runtime` panic 问题中，会出现如下错误：
```
thread 'main' panicked at deadpool-runtime-0.1.4/src/lib.rs:101:22:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

这发生在 runtime 关闭时任务仍在运行的情况 - Arc 方案无法防止这种情况。

## **11** 解决问题的过程

### 尝试过的方案：

1. **纯 Arc<Runtime> 方案**：
   - 尝试依赖所有组件手动 drop 它们的 Arc 克隆
   - 失败了，因为没有办法追踪或强制所有克隆 drop
   - 无法保证及时关闭

2. **纯 Mutex<Option<Runtime>> 方案**：
   - 尝试使用 Mutex 进行独占访问，用 `take()` 关闭
   - 失败了，因为 TSP workers 需要长时间访问，但 MutexGuard 不是 Send
   - 必须反复加锁/解锁，造成性能问题

3. **RwLock<Option<Runtime>> 方案**（考虑过）：
   - 类似 Mutex，但允许多个读者
   - 仍然无法跨 .await 点持有 guard
   - 与 Mutex 有相同的根本问题

4. **研究类似问题**：
   - 找到了 `rust-logout-issue` 示例，它解决了 runtime 关闭时机问题
   - 意识到即使有正确的关闭序列，Arc 也不提供生命周期控制

### 突破：

发现解决方案是**组合** Arc 和一个独立的关闭协调机制：
- 使用 Arc<Runtime> 实现易于共享（解决人机工程学问题）
- 使用 CancellationToken 实现关闭协调（解决控制问题）
- 实现一个优雅的关闭序列，等待任务完成

关键洞察：不要试图直接控制 Arc 的生命周期。相反，通过一个独立的信号通道来控制*运行在 runtime 上的任务*。

### 最终解决方案：

```rust
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
static SHUTDOWN_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

// Components clone Arc freely
fn get_runtime() -> Arc<Runtime> {
    Arc::clone(TOKIO_RUNTIME.get().unwrap())
}

// Tasks listen for shutdown signal
async fn task_with_graceful_shutdown() {
    let token = SHUTDOWN_TOKEN.get().unwrap().clone();

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                // Cleanup and exit
                break;
            }
            _ = tokio::time::sleep(Duration::from_millis(500)) => {
                // Do work
            }
        }
    }
}

// Shutdown sequence
async fn graceful_shutdown() {
    // 1. Signal all tasks to stop
    SHUTDOWN_TOKEN.get().unwrap().cancel();

    // 2. Wait for tasks to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 3. Now safe to shutdown runtime
}
```

这结合了两种方案的优点：
- ✅ 易于共享（Arc）
- ✅ 长期持有引用（Arc）
- ✅ 可控关闭（CancellationToken）
- ✅ 优雅清理（等待任务完成）

## **12** 解决方案

见附件 `tokio-runtime-sharing.zip`，包含：
- `broken-example/`：演示 Arc 和 Mutex 的问题
- `correct-example/`：演示 Arc + CancellationToken 解决方案

两个示例都通过了 `cargo check`、`cargo test` 和 `cargo clippy`。

环境信息：
- Rust 版本：1.90.0 (stable)
- 工具链：stable-aarch64-apple-darwin
- 操作系统：macOS 15.1 (Build 25B7)
- 架构：Apple Silicon (ARM64)

许可证：MIT

## **13** 补充说明

### 相关文档：
- Tokio CancellationToken: https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html
- Tokio graceful shutdown: https://tokio.rs/tokio/topics/shutdown
- Arc 文档: https://doc.rust-lang.org/std/sync/struct.Arc.html

### 与其他问题的关联：
这个解决方案建立在本仓库中的 `rust-logout-issue` 示例之上，后者解决了 runtime 关闭时机问题。两种模式的组合提供了一个完整的解决方案：
1. 退出登录序列的状态机（来自 rust-logout-issue）
2. Runtime 共享的 Arc + CancellationToken（本示例）

### 考虑过的其他方案：
- 使用 `tokio::runtime::Handle` 替代 Arc<Runtime>：仍然不能解决关闭协调问题
- 使用 channels 分发 runtime 访问：过于复杂，不提供长期访问
- 使用带 Drop 的自定义 guard 类型：无法强制所有 guards 同时 drop

### 关键收获：
Rust 的所有权系统擅长防止数据竞争，但共享资源的生命周期管理需要显式协调。Arc 提供共享但不提供生命周期控制。解决方案是分离关注点：使用 Arc 进行共享，使用独立的机制（CancellationToken）进行生命周期协调。

这是 Rust 异步编程中的常见模式，应该得到更广泛的记录和推广。
