# Rust数据标注信息提交

## **04** 问题概括

`tokio::select!` 中使用“零延迟”分支导致 CPU 100% 占用，真实工作分支被饿死

## **05** 问题行业描述

即时通信 / 实时事件处理服务，需要在主线程中消费高频消息队列

## **06** 问题分类

[X] 异步/并发/竞态条件相关问题
[X] 性能瓶颈/性能优化

## **07** 问题背景

在重构一个实时事件路由器时，我们把所有后台任务都通过 `tokio::sync::mpsc::UnboundedSender` 推入同一个消费者循环。为了避免阻塞 `recv().await`，当队列短暂为空时，我们额外加了一个“零延迟”分支，希望能够在下一轮检查队列。

在生产环境（macOS + Tokio multi-thread runtime）中，这个改动带来了灾难性的副作用：一旦队列暂时变空，进程的某个核心会瞬间飙到 100%，并且消费者永远不会再读取新的消息，它一直卡在“空转”分支里，导致后续事件堆积和 UI 卡死。

## **08** 问题描述

### 想要实现的功能

- 持续从 `UnboundedReceiver` 读取消息
- 在队列暂时为空时“轻量级”地等待一下再检查
- 仍然保持 UI 线程响应（不能长时间阻塞）

### 遇到的困难

- 我们把 `tokio::select!` 改成 `biased;` 模式，并添加一个立即就绪的分支（等价于 `tokio::time::sleep(Duration::ZERO)`）。
- 由于该分支永远优先返回 `Poll::Ready`，循环根本不会再去 `await recv()`。
- 结果是 CPU 被满速占用，同时真实的消息处理分支完全饿死（processed count 始终为 0）。

### 最困惑的地方

直觉上“零延迟等待”应该只会让出调度一次，但实际上 `async {}` 这样的立即完成未来会在 `select!` 中立刻完成，无限次抢占其它分支。Tokio 的 `biased` 机制更是强化了这个问题。

## **09** 问题代码或问题详细描述

```rust
loop {
    tokio::select! {
        biased;
        // Attempted zero-delay backoff (equivalent to sleep(0))
        _ = async {} => {
            idle_ticks += 1;
            if idle_ticks >= SPIN_LIMIT {
                break; // guard to avoid locking up tests
            }
        }
        message = rx.recv() => {
            match message {
                Some(job) => handle(job),
                None => break, // sender dropped
            }
        }
    }
}
```

- 由于 `_ = async {}` 每次 poll 都立即完成，`select!` 总是命中该分支。
- `biased;` 让第一个分支拥有最高优先级，`recv()` 永远得不到运行机会。
- `idle_ticks` 会在几十毫秒内冲到 150000，CPU 占满，`processed` 始终为 0。

## **10** 错误信息

这是运行时行为问题，可以通过日志与监控复现：

```
[INFO  broken_example] Processed 0 messages in 0 ms, idle ticks: 150000
Busy loop detected: hit the spin limit (150000). The biased select! kept the zero-duration sleep branch hot.
```

- `Processed 0 messages` 说明真实业务完全饿死。
- `idle ticks` 瞬间达到保护阈值，证明循环没有任何等待。
- 同时可以在活动监视器中看到单核 100% 占用。

## **11** 解决问题的过程

1. 通过加入 `idle_ticks` 计数和日志确认卡在“零延迟”分支。
2. 使用 `tokio-console` 观察 future 状态，发现 `recv()` future 从未被 poll。
3. 实验性地将 `biased;` 移除，CPU 占用下降，但仍出现大量无意义轮询，说明核心问题是“立即完成的 Future”。
4. 最终决定把空闲分支改成真正的协作式等待：采用 `tokio::time::Interval`，并设置 `MissedTickBehavior::Delay`，确保进入下一帧前至少等待 5ms。
5. 在新实现里，`processed` 恢复递增，`idle_ticks` 保持在个位数，CPU 占用恢复正常。

## **12** 解决方案

见目录 `submissions/tokio-select-spin/`（以及同名压缩包 `tokio-select-spin.zip`）：

- `broken-example/`：包含触发 CPU 热循环的最小复现（带测试和日志）
- `correct-example/`：使用 `tokio::time::Interval` 的修复版本（同样附带回归测试）

两个示例都通过了 `cargo fmt`、`cargo test`、`cargo clippy -- -D warnings`（本地验证命令均使用 `CARGO_NET_OFFLINE=true` 以满足无网络环境）。

环境信息：

- Rust 版本：1.85.0 (stable)
- 工具链：aarch64-apple-darwin
- 操作系统：macOS 15.1.1 (Apple Silicon)
- Tokio：1.48.0

许可证：MIT
