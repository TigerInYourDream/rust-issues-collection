# Rust数据标注信息提交

## **01** 提交者展示名称

[请填写你的展示名称]

## **02** 微信联系方式

[请填写你的微信号]

## **03** 个人邮箱

[请填写你的邮箱]

## **04** 问题概括

手动实现异步状态机时，自引用结构在移动后导致内部指针失效，引发未定义行为

## **05** 问题行业描述

高性能异步框架开发、自定义Future实现、嵌入式异步运行时

## **06** 问题分类

请将您的Rust问题进行分类,只能选择一个最符合问题特征的分类:

(X) 所有权与借用相关问题
( ) 生命周期管理问题
( ) 异步中的生命周期管理问题
( ) 异步/并发/竞态条件相关问题
( ) 性能瓶颈/性能优化
( ) 错误处理相关
( ) Unsafe代码的误用等
( ) 类型不匹配，推理错误
( ) 字符串处理与解析
( ) 宏相关问题
( ) FFI相关问题
( ) 从其它语言中带来的坏习惯/如何写地道Rust代码
( ) 调试方法
( ) 日志系统和处理
( ) Trait的使用
( ) 泛型的使用
( ) 平台特定相关问题
( ) 设计模式相关
( ) 特性开关/配置/Cargo构建相关
( ) 库使用问题
( ) 框架使用问题
( ) 其他

## **07** 问题背景

在开发高性能异步系统时，为了避免 async-trait 的动态分发开销，需要手动实现 Future trait。在实现异步缓冲读取器 AsyncBufReader 时，需要在结构体内部持有指向自身 buffer 字段的指针，以避免每次访问都重新创建切片。这种自引用结构在其他语言（C++）中很常见，但在 Rust 中会因为移动语义导致内部指针失效。

问题出现在生产环境的异步 I/O 库中，当 Future 被移动到不同的内存位置（如存入 Vec、返回函数、或被 tokio runtime 调度）时，内部指针变成悬垂指针，导致 segfault 或读取到错误数据。

## **08** 问题描述

想要实现一个异步缓冲读取器，包含以下字段：

- inner: 底层异步读取器（如 TcpStream）
- buffer: 缓冲区 Vec<u8> 或 Box<[u8]>
- filled: 指向 buffer 中已填充部分的切片

遇到的困难：

1. 使用引用 `filled: &[u8]` 无法编译，因为无法指定生命周期参数（自引用）
2. 使用裸指针 `filled_ptr: *const u8` 可以编译，但结构体移动后指针失效
3. 不理解为什么 Future::poll 需要 Pin<&mut Self>，以及如何安全访问字段

最困惑的地方：

- 为什么 Rust 不允许自引用结构？
- Pin 如何保证内存地址不变？
- 什么时候用 get_mut() vs get_unchecked_mut()？
- Unpin 和 !Unpin 的区别是什么？

## **09** 问题代码或问题详细描述

```rust
use std::ptr;

// PROBLEM: Self-referential struct without Pin
struct SelfReferential {
    data: String,
    ptr_to_data: *const u8,  // Points to data's heap buffer
}

impl SelfReferential {
    fn new(text: &str) -> Self {
        let data = String::from(text);
        let ptr_to_data = data.as_ptr();
        Self { data, ptr_to_data }
    }

    fn get_data(&self) -> &str {
        unsafe {
            // UNDEFINED BEHAVIOR: ptr_to_data may be dangling
            let slice = std::slice::from_raw_parts(
                self.ptr_to_data,
                self.data.len()
            );
            std::str::from_utf8_unchecked(slice)
        }
    }
}

fn main() {
    let s1 = SelfReferential::new("hello");
    println!("s1: {}", s1.get_data());  // Works

    let s2 = s1;  // Move invalidates pointer!
    println!("s2: {}", s2.get_data());  // UB: may crash or garbage

    // Put in Vec - forces move during reallocation
    let mut vec = Vec::new();
    vec.push(SelfReferential::new("world"));
    vec.push(SelfReferential::new("rust"));  // Realloc moves vec[0]

    println!("vec[0]: {}", vec[0].get_data());  // CRASH or garbage data
}
```

## **10** 错误信息

使用引用的编译错误：

```
error[E0106]: missing lifetime specifier
  --> src/lib.rs:5:13
   |
5  |     filled: &[u8],
   |             ^ expected named lifetime parameter
```

尝试自引用的编译错误：

```
error[E0515]: cannot return value referencing local variable `data`
  --> src/main.rs:12:9
   |
10 |         let ptr = &data;
   |                    ---- `data` is borrowed here
11 |
12 |         Self { data, ptr }
   |         ^^^^^^^^^^^^^^^^^^ returns a value referencing data owned by the current function
```

Miri 检测到的 UB：

```bash
$ cargo +nightly miri run

error: Undefined Behavior: dereferencing pointer failed:
       0x1234[noalloc] is a dangling pointer (it has no provenance)
  --> src/main.rs:25:13
   |
25 |             std::slice::from_raw_parts(self.ptr_to_data, self.data.len())
   |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## **11** 解决问题的过程

尝试过的方案：

1. 使用生命周期参数 `&'a [u8]` - 失败，因为 Rust 不允许自引用
2. 使用 `Option<&[u8]>` 延迟初始化 - 失败，同样的借用问题
3. 使用裸指针 - 编译通过但有 UB，必须手动保证不移动
4. 重新设计避免自引用（使用索引）- 安全但每次访问有计算开销
5. 使用 Pin + PhantomPinned - 成功，既安全又零成本

最终解决方案：

- 使用 `Pin<Box<Self>>` 保证结构体永远不会移动
- 添加 `PhantomPinned` 标记类型为 !Unpin
- 使用 `pin_project` 宏安全地访问 pinned 字段
- 在构造函数中立即 pin 到堆上：`Box::pin(reader)`

关键突破点：理解 Pin 是类型级别的保证，不是数据结构。Pin<P> 表示指针 P 指向的值永远不会被移动，通过限制 API（只有 Unpin 类型才能调用 get_mut()）来强制这个保证。

## **12** 解决方案

见附件 `pin-self-referential.zip`

项目结构：

- broken-example/: 展示自引用结构的 UB 问题
- correct-example/: 使用 Pin + pin_project 的正确实现

运行验证：

```bash
cd broken-example && cargo check && cargo test && cargo clippy
cd correct-example && cargo check && cargo test && cargo clippy
```

环境信息：

- Rust 版本: 1.85.0
- 工具链: aarch64-apple-darwin
- 操作系统: macOS 15.1.1
- License: MIT

## **13** 补充说明

解决过程中查阅的资源：

- Rust Async Book - Pinning 章节
- std::pin 模块文档,rr
- Pin RFC (RFC 2349)
- "Pinning in plain English" by Adam Chalmers
- "Pin and suffering" by fasterthanlime
- pin-project crate 文档

这个问题在标准库的 Future trait 设计、tokio 的异步 I/O、以及手动实现状态机时都会遇到。理解 Pin 对于深入使用 Rust 异步编程至关重要。

替代方案对比：

1. 使用索引而非指针 - 100% 安全但有轻微计算开销
2. 分离所有权避免自引用 - 最简单最安全
3. Pin + 裸指针 - 最高性能但需要 unsafe

使用 Miri 检测工具验证了所有 unsafe 代码的正确性。
