# Rust数据标注信息提交

## **04** 问题概括

Arc 强引用导致 Matrix 客户端无法完成退出：背景任务持有 Arc 使得 Drop 永远不会触发，登出状态机被卡死。

## **05** 问题行业描述

GUI + 异步同步服务（Matrix 客户端 / IM 应用）的运行时管理。

## **06** 问题分类

- [X] 异步/并发/竞态条件相关问题
- [ ] 所有权与借用相关问题
- [ ] 生命周期管理问题
- [ ] 其他

## **07** 问题背景

在 Robrix Matrix 客户端里，我们维护一个长期存在的 Matrix SDK 客户端（内含 deadpool-sqlite 连接池）。应用登出流程是一个状态机：停止同步 → 服务器登出 → 清理客户端 → 等待 `oneshot` 确认 → 关闭 Tokio runtime → 重启 runtime。为了确认所有后台任务都退出，我们在 `ClientInner::drop` 里通过 `oneshot` 通知状态机“可以进入下一步”。

然而，每个后台任务都会 `tokio::spawn` 一个无限循环，并且捕获了 `Arc<ClientInner>` 的强引用。即使应用逻辑已经从全局状态中移除了最后一个 `Arc`，那些任务仍然持有它，导致 `ClientInner::drop` 永远不会执行，`oneshot` 也不会发出信号，整条状态机就卡在“等待清理完成”这一步。如果此时强行关闭 runtime，deadpool-runtime 会 panic：“there is no reactor running…”。

## **08** 问题描述

- 想实现：退出登录时阻塞等待客户端内部资源释放，再安全地关闭 Tokio runtime。
- 实际情况：`CLIENT_OBJECT: Arc<ClientInner>` 被后台任务克隆，强引用计数 > 1。
- Drop 逻辑位于 `ClientInner`（Arc 指向的对象）上，而不是 GUI 层手里的 wrapper。
- 后台任务是无限循环，所以它们永远不会自然 drop，Arc 强引用也就永远不为 0。
- 状态机一直在 `tokio::time::timeout` 里等待 `oneshot`，最终超时并报错；强行关闭 runtime 则触发 deadpool panic。

## **09** 问题代码或问题详细描述

```rust
fn spawn_background_tasks(inner: Arc<ClientInner>) {
    for task_id in 0..3 {
        let task_inner = inner.clone();      // ❌ 强引用泄漏
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(80)).await;
                log::info!("task {task_id} still owns Arc for client {}", task_inner.id);
            }
        });
    }
}

async fn wait_for_cleanup(rx: oneshot::Receiver<()>) -> Result<()> {
    tokio::time::timeout(Duration::from_millis(300), rx)
        .await
        .context("logout stalled: client drop signal never arrived")?
        .context("drop sender dropped before sending signal")
}
```

## **10** 错误信息

运行 `cargo run`（broken-example）得到：

```
[ERROR] Logout failed: logout stalled: client drop signal never arrived
[ERROR] Strong count after dropping last user handle: 3
[ERROR] Drop handler never fired because background tasks leaked the Arc
```

如果在真实项目中继续强制关闭 runtime，会看到：

```
thread 'main' panicked at deadpool-runtime-0.1.4/src/lib.rs:101:22:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

## **11** 解决问题的过程

1. 初始实现中假设“只要把 `Arc` 从全局状态里移除，Drop 就会触发”。日志显示 `Arc::strong_count` 仍然为 3，Drop 从未执行。
2. 尝试在 Drop 里 `abort` 后台任务，但 Drop 根本不会触发，方案失败。
3. 考虑将任务句柄集中存储并在登出时 `JoinHandle::abort()`，但 Matrix SDK 内部也会 spawn 任务，无法完全追踪。
4. 最终采用 `Weak<ClientInner>`：后台任务每次循环只升级到临时 `Arc`，完成一轮工作后立即释放。
5. 当最后一个用户态 `Arc` 被 drop 后，下一次 `upgrade()` 失败，任务主动退出，Arc 强引用变为 0，`ClientInner::drop` 马上执行并发出 oneshot。
6. 登出状态机收到信号后才能安全地执行 `Runtime::shutdown_background()`，彻底避免 deadpool panic。

## **12** 解决方案（压缩成.zip包提交，请不要提交图片）

- `submissions/arc-strong-count-shutdown/broken-example`: 可复现的失败场景。
- `submissions/arc-strong-count-shutdown/correct-example`: 使用 `Weak` + 显式退出的修复方案。
- 两个 Cargo 项目都通过 `cargo check`, `cargo test`, `cargo clippy -- -D warnings`。
- README 记录了环境信息（Rust 1.90.0 stable-aarch64-apple-darwin / macOS 15.1）。
- 打包时请在该目录下执行 `cargo clean` 并压缩。

## **13** 补充说明

- 文档参考：`Arc`/`Weak` 官方文档、Tokio shutdown 文档、Matrix SDK + deadpool-runtime 源码。
- 该模式与 `rust-logout-issue` 的状态机结合后，能提供“等待 Drop 信号再关 runtime”的完整退出链路。
- 与单纯的 `JoinHandle::abort` 不同，`Weak` 方案对第三方库 spawn 的任务同样有效，因为它们只能通过 `Weak` 升级获取短暂访问权。
