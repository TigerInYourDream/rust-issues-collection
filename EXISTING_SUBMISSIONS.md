# Rust Issues Collection - Existing Submissions

This document provides a comprehensive overview of all existing Rust issue submissions in this repository.

---

## Table of Contents

1. [Tokio Runtime Sharing: Arc vs Mutex Dilemma](#1-tokio-runtime-sharing-arc-vs-mutex-dilemma)
2. [Thread-Local UI Safety: Witness Type Pattern](#2-thread-local-ui-safety-witness-type-pattern)
3. [Async Drop Trap: Synchronous Drop with Async Resources](#3-async-drop-trap-synchronous-drop-with-async-resources)
4. [Room DisplayName Type Safety](#4-room-displayname-type-safety)
5. [Arc Strong Count Leak Blocks Logout Cleanup](#5-arc-strong-count-leak-blocks-logout-cleanup)

---

## 1. Tokio Runtime Sharing: Arc vs Mutex Dilemma

### Problem Summary
**Category**: Async/Concurrency/Race Conditions
**Difficulty**: ⭐⭐⭐⭐⭐
**Value**: 200 RMB tier

Tokio runtime sharing presents a fundamental dilemma: **Arc<Runtime>** provides easy sharing but no lifecycle control, while **Mutex<Option<Runtime>>** provides control but poor ergonomics.

### Industry Context
GUI application async runtime development, particularly Matrix clients requiring graceful logout and restart flows.

### The Problem

In the Robrix Matrix client (Makepad-based GUI), we need a global Tokio runtime that:
1. Shares across multiple components (TSP workers, event handlers, UI updates)
2. Can be gracefully shutdown during logout/re-login flows

Two approaches, both with critical flaws:

#### Arc<Runtime> Approach
```rust
static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

struct TspWorker {
    runtime: Arc<Runtime>,  // Cloned Arc
}

// PROBLEM: Cannot force shutdown
// Even if we drop the original Arc, workers still hold clones
// No way to guarantee runtime actually shuts down
```

**Problems**:
- ❌ Cannot force all Arc clones to drop
- ❌ No way to ensure timely shutdown
- ❌ Risk of dangling references after shutdown attempt

#### Mutex<Option<Runtime>> Approach
```rust
static TOKIO_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

// PROBLEM: Cannot hold MutexGuard across .await points
// Must lock/unlock repeatedly for continuous work
// Lock contention and deadlock risks
```

**Problems**:
- ❌ MutexGuard not Send - cannot hold across .await
- ❌ Must lock/unlock repeatedly (performance overhead)
- ❌ Lock contention and deadlock risks
- ❌ Difficult to use ergonomically

### The Solution: Arc + CancellationToken

Combine Arc for sharing with CancellationToken for shutdown coordination:

```rust
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
static SHUTDOWN_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

// Tasks listen for shutdown signal
async fn task_with_graceful_shutdown() {
    let token = SHUTDOWN_TOKEN.get().unwrap().clone();

    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            _ = do_work() => { /* ... */ }
        }
    }

    // Cleanup always runs
    perform_cleanup().await;
}

// Shutdown sequence
async fn graceful_shutdown() {
    // 1. Signal all tasks to stop
    SHUTDOWN_TOKEN.get().unwrap().cancel();

    // 2. Wait for tasks to complete
    wait_for_tasks().await;

    // 3. Now safe to shutdown runtime
}
```

### Benefits
- ✅ Easy sharing via Arc cloning
- ✅ Long-lived references without lock contention
- ✅ Controlled shutdown via CancellationToken
- ✅ Graceful cleanup prevents deadpool-runtime panic
- ✅ Works well with TSP and long-running tasks

### Key Insights
Don't try to directly control Arc's lifetime. Instead, control the **tasks running on the runtime** through an independent signaling channel.

### Files
- `submissions/tokio-runtime-sharing/broken-example/` - Demonstrates Arc and Mutex problems
- `submissions/tokio-runtime-sharing/correct-example/` - Complete Arc + CancellationToken solution
- `submissions/tokio-runtime-sharing/SUBMISSION.md` - Full submission details

### Related Patterns
This solution builds upon the `rust-logout-issue` state machine pattern. Combined, they provide:
1. Logout sequence state machine (from rust-logout-issue)
2. Runtime sharing with Arc + CancellationToken (this example)

---

## 2. Thread-Local UI Safety: Witness Type Pattern

### Problem Summary
**Category**: Best Practices / Async-Concurrency
**Difficulty**: ⭐⭐⭐⭐
**Value**: 100-200 RMB tier

How to safely manage thread-local UI state in Rust GUI applications using the type system to prevent cross-thread data access at compile-time.

### Industry Context
GUI/Desktop application development - multi-threaded desktop apps where UI state must remain on the main thread while background tasks handle network I/O. Common in Matrix clients, chat apps, and any GUI with async background work.

### The Problem

Coming from JavaScript (single-threaded) or languages where Mutex is used everywhere, developers may naively use `thread_local!` without proper safeguards.

#### Broken Approach
```rust
use std::sync::Mutex;

// ❌ PROBLEM 1: Using Mutex for single-threaded data
thread_local! {
    static ROOM_CACHE: Mutex<HashMap<String, RoomData>> =
        Mutex::new(HashMap::new());
}

// ❌ PROBLEM 2: No compile-time guarantee this is called from UI thread
pub fn add_room(room: RoomData) {
    ROOM_CACHE.with(|cache| {
        cache.lock().unwrap().insert(room.id.clone(), room);
    });
}

// ❌ PROBLEM 3: Background threads can call this - compiles without error!
fn background_thread() {
    // Compiles but accesses DIFFERENT thread_local storage!
    let room = get_room("room1");  // Returns None even if exists on UI thread
}
```

**Why This is Dangerous**:
- **No compile-time errors** - code compiles and runs
- **Silent failures** - each thread has its own `thread_local!` copy
- **Data inconsistency nightmare** - UI thread and background threads see different data

**Runtime Behavior**:
```
[UI Thread] Added room1
[UI Thread] Room: Some(RoomData { id: "room1", ... })
[Background Thread] Looking for room1
[Background Thread] Room: None  // ← BUG! It exists, but in different storage
```

### The Solution: Witness Type Pattern

Create a special marker type that can only be created on the UI thread, and require it as a parameter:

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

// Use RefCell instead of Mutex for single-threaded interior mutability
thread_local! {
    static ROOM_CACHE: Rc<RefCell<HashMap<String, RoomData>>> =
        Rc::new(RefCell::new(HashMap::new()));
}

// All UI functions require the witness
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

### Benefits
- ✅ **Compile-time safety**: Cannot call UI functions without UiContext
- ✅ **Zero runtime overhead**: UiContext is zero-sized, compiles to nothing
- ✅ **Clear API contracts**: Function signatures document thread requirements
- ✅ **Idiomatic Rust**: Leverages type system for safety
- ✅ **Maintainable**: New developers cannot misuse the API

### Three Key Patterns

**Pattern 1: Witness Type**
```rust
#[derive(Clone)]
pub struct UiContext {
    _private: (),  // Zero-sized capability type
}
```

**Pattern 2: RefCell for Interior Mutability**
```rust
thread_local! {
    static ROOM_CACHE: Rc<RefCell<HashMap<String, RoomData>>> = ...;
}
// RefCell (not Mutex) - appropriate for single-threaded access
// Rc (not Arc) - not thread-safe, further prevents misuse
```

**Pattern 3: Witness-Guarded API**
```rust
pub fn add_room(_ui: &UiContext, room: RoomData) { }
// Compiler enforces at every call site
```

### Alternative Approaches Considered

**1. Runtime Thread ID Check**
```rust
assert_eq!(thread::current().id(), *UI_THREAD_ID.get().unwrap());
```
- ❌ Runtime cost on every call
- ❌ Panics instead of compile errors
- ❌ Easy to forget

**2. Sealed Trait Pattern**
```rust
pub trait UiThread: private::Sealed {}
```
- ✅ Also effective
- ❌ More complex API
- ❌ Harder for beginners

**3. Type State Pattern with Generics**
```rust
struct RoomCache<State> { }
impl RoomCache<UiThreadState> { }
```
- ✅ Very type-safe
- ❌ Overly complex for this use case
- ❌ Harder to integrate with existing code

### Files
- `submissions/thread-local-ui-safety/broken-example/` - Demonstrates the problem
- `submissions/thread-local-ui-safety/correct-example/` - Complete solution with tests
- `submissions/thread-local-ui-safety/SUBMISSION.md` - Full submission details

### Real-World Usage
This pattern is used in:
- **Robrix**: Matrix client UI state management
- **Makepad**: UI framework context passing
- **egui**: Implicit context parameters

---

## 3. Async Drop Trap: Synchronous Drop with Async Resources

### Problem Summary
**Category**: Async/Concurrency / Design Patterns
**Difficulty**: ⭐⭐⭐⭐⭐
**Value**: 200 RMB tier

The `Drop` trait is synchronous (`fn drop(&mut self)`), but async resources require async cleanup (`.await`). This fundamental mismatch leads to resource leaks, data corruption, and deadlocks.

### Industry Context
Async application development requiring graceful shutdown: background services, database connection pools, network servers, GUI applications with background tasks.

### The Problem

When developing the Robrix Matrix client, we needed to manage background async tasks holding important resources (database connections, temp files, network connections). During logout/shutdown, we hit a fundamental issue:

**The Root Conflict**:
- Rust's `Drop` trait is **synchronous**
- Cleaning up async resources requires **async operations** (`.await`)
- This mismatch causes severe problems

#### Three Broken Approaches

**Approach 1: Call abort() in Drop**
```rust
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        self.task_handle.abort();  // ❌ Cleanup code never executes!
    }
}
```
**Problem**: `abort()` immediately terminates the task. Cleanup code (delete temp files, flush buffers) never runs.

**Approach 2: Use block_on in Drop**
```rust
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        tokio::runtime::Handle::current()
            .block_on(async { self.task_handle.await.ok() });
        // ❌ Panics or deadlocks if Drop called from async context
    }
}
```
**Problem**: `block_on` cannot be called from async context. Will panic or deadlock.

**Approach 3: Provide async shutdown but can't enforce usage**
```rust
async fn shutdown(self) { /* ... */ }
```
**Problem**: If developer forgets to call `shutdown()` and just drops, still leaks resources.

### Real-World Impact

In production, this causes:
- Database transactions not committed
- Files not flushed (partial writes)
- Network connections not properly closed
- Temporary resource leaks

### The Solution: Explicit Async Cleanup + Drop Safety Net

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

            // Cleanup code ALWAYS runs
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

### Three Key Design Patterns

**1. Explicit Async Cleanup Methods**
```rust
async fn shutdown(self) -> Result<()>
async fn shutdown_with_timeout(self, timeout: Duration) -> Result<()>
```

**2. Signaling Mechanisms**
- `tokio::sync::watch` - for sending shutdown signals
- `tokio::sync::Notify` - for waiting on cleanup completion
- Or use `tokio_util::sync::CancellationToken`

**3. Drop Safety Net**
```rust
impl Drop {
    // Detect if shutdown() was forgotten
    // Emit warning
    // abort() as last resort
}
```

### Why No Async Drop?

Rust core team is discussing this (RFC tracking issue #126482), but faces technical challenges:

1. Drop may be called during panic unwinding - can't be async
2. Drop timing is deterministic (RAII) - async introduces indeterminism
3. Potential performance issues (every drop needs runtime)

**Current best practice**: Provide explicit async cleanup method, Drop only as safety net.

### Applicable Scenarios

This pattern applies to any async cleanup scenario:
- **Database connection pools** - commit transactions, close connections
- **Network services** - complete requests, graceful disconnect
- **File operations** - flush buffers, sync to disk
- **Background tasks** - complete in-progress work
- **GUI applications** - save state, clean temp files

### Files
- `submissions/async-drop-trap/broken-example/` - Demonstrates the problems
- `submissions/async-drop-trap/correct-example/` - Complete solution
- `submissions/async-drop-trap/SUBMISSION.md` - Full submission details

### Comparison with Other Languages

- **C++**: No async, RAII cleanup synchronously in destructor
- **Go**: Has `defer`, but no deterministic destruction
- **C#**: Has `IAsyncDisposable` interface, requires explicit `DisposeAsync()` call
- **Rust**: Chose explicit async cleanup + Drop safety net combination

Rust's approach balances compile-time safety with runtime flexibility.

---

## 4. Room DisplayName Type Safety

### Problem Summary
**Category**: Type System / Design Patterns
**Difficulty**: ⭐⭐⭐
**Value**: 50-100 RMB tier

Using the newtype pattern to create compile-time distinctions between semantically different strings (room IDs vs display names).

### Industry Context
Chat applications, Matrix clients - any system handling multiple types of identifiers that should not be confused.

### The Problem

In Matrix clients, rooms have multiple identifiers:
- **Room ID**: Unique identifier (e.g., "!abc123:matrix.org")
- **Display Name**: Human-readable name (e.g., "Rust Programming")

Using plain `String` for both creates opportunities for bugs:

```rust
// ❌ Easy to mix up - both are String
fn update_room(room_id: String, display_name: String) { }

// Bug: arguments swapped, but compiles fine!
update_room(display_name, room_id);
```

### The Solution: Newtype Pattern

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

// Now this won't compile:
fn update_room(room_id: RoomId, display_name: DisplayName) { }

update_room(display_name, room_id);  // ❌ Type error!
```

### Benefits
- ✅ Compile-time prevention of argument confusion
- ✅ Self-documenting code
- ✅ Zero runtime overhead
- ✅ Easy to add domain-specific methods

### Files
- `submissions/room-displayname-type-safety/broken-example/` - Shows the problem
- `submissions/room-displayname-type-safety/correct-example/` - Newtype solution
- `submissions/room-displayname-type-safety/SUBMISSION.md` - Full details

---

## 5. Arc Strong Count Leak Blocks Logout Cleanup

### Problem Summary
**Category**: Async/Concurrency/Race Conditions  
**Difficulty**: ⭐⭐⭐⭐⭐  
**Value**: 200 RMB tier

Logout waits for `ClientInner::drop` to send a oneshot confirmation before shutting down the Tokio runtime. Unfortunately every background task cloned the same `Arc<ClientInner>`, so the strong count never reached zero and `drop` never ran. The state machine timed out and operators forcibly terminated the runtime, immediately hitting the `deadpool-runtime there is no reactor running` panic because leaked tasks were still polling the old reactor.

### Industry Context
Matrix/IM desktop apps (Robrix) that must recycle long-lived runtimes and database pools during logout/login cycles without leaking SDK internals.

### The Problem

```rust
fn spawn_background_tasks(inner: Arc<ClientInner>) {
    for task_id in 0..3 {
        let task_inner = inner.clone(); // ❌ keeps strong Arc alive forever
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(80)).await;
                log::info!("task {task_id} still owns Arc {}", task_inner.id);
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

Because the drop logic is attached to the `Arc` target, it never executes until every strong reference disappears. Infinite-loop tasks ensure that never happens.

### The Solution
Swap strong clones for `Weak<ClientInner>` and only upgrade inside the loop. The temporary `Arc` is dropped before the `await`, so once the GUI layer releases its final handle, the next `upgrade()` fails and the task exits on its own:

```rust
fn spawn_background_tasks(inner: &Arc<ClientInner>) {
    for task_id in 0..3 {
        let weak = Arc::downgrade(inner);
        tokio::spawn(async move {
            loop {
                match weak.upgrade() {
                    Some(state) => log::info!("task {task_id} borrowed {}", state.id),
                    None => {
                        log::info!("task {task_id} noticed drop, exiting");
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
        });
    }
}
```

Once all strong refs disappear, `ClientInner::drop` immediately fires, sends the oneshot, and the logout state machine proceeds to `Runtime::shutdown_background()` without triggering deadpool panics.

### Files
- `submissions/arc-strong-count-shutdown/broken-example/`
- `submissions/arc-strong-count-shutdown/correct-example/`
- `submissions/arc-strong-count-shutdown/SUBMISSION.md`

---

## Summary Statistics

| Issue | Category | Difficulty | Value Tier | LOC (approx) |
|-------|----------|------------|------------|--------------|
| Tokio Runtime Sharing | Async/Concurrency | ⭐⭐⭐⭐⭐ | 200 RMB | ~400 |
| Thread-Local UI Safety | Best Practices | ⭐⭐⭐⭐ | 100-200 RMB | ~300 |
| Async Drop Trap | Async/Design Patterns | ⭐⭐⭐⭐⭐ | 200 RMB | ~500 |
| Room DisplayName Type Safety | Type System | ⭐⭐⭐ | 50-100 RMB | ~200 |
| Arc Strong Count Leak | Async/Concurrency | ⭐⭐⭐⭐⭐ | 200 RMB | ~250 |

**Total Estimated Value**: 750-900 RMB

---

## Common Themes

All submissions share these characteristics:

1. **Real-world scenarios** from Robrix Matrix client development
2. **Compile-time safety** leveraging Rust's type system
3. **Zero-cost abstractions** where possible
4. **Idiomatic patterns** that respect Rust conventions
5. **Educational value** teaching important Rust concepts

---

## Environment Information

All examples tested with:
- **Rust Version**: 1.85.0 (stable) or 1.90.0 (stable)
- **Toolchain**: aarch64-apple-darwin (Apple Silicon)
- **OS**: macOS 15.1.1
- **Architecture**: ARM64

All projects pass:
- `cargo check`
- `cargo test`
- `cargo clippy`

**License**: MIT for all submissions

---

## Related Documentation

### Tokio Resources
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)
- [CancellationToken Docs](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)

### Rust Language Resources
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust for Rustaceans](https://rust-for-rustaceans.com/) - API Design chapter
- [Async Drop Tracking Issue](https://github.com/rust-lang/rust/issues/126482)

### Project Resources
- [Robrix Matrix Client](https://github.com/project-robius/robrix) - Source of these issues
- [Makepad Framework](https://github.com/makepad/makepad) - UI framework used

---

*Last Updated: 2025-10-20*
