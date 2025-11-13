# Runtime Feature Detection Prevents Release SIGILL on Legacy CPUs

## 01. 提交者展示名称

[请在实际提交时填写]

## 02. 微信联系方式

[请在实际提交时填写]

## 03. 个人邮箱

[请在实际提交时填写]

## 04. 问题概括

跨平台发布启用 AVX2 后，Rust release 二进制在缺乏该指令集的 CPU 上出现 `illegal instruction` 崩溃

## 05. 问题行业描述

大规模向量检索 / 实时推荐系统：在多代 x86 服务器混布的生产集群里进行 SIMD 优化部署

## 06. 问题分类

- **主要**: 性能瓶颈/性能优化
- **次要**: 特性开关/配置/Cargo构建相关
- **关联**: 平台特定相关问题, 错误处理相关

## 07. 问题背景

我们为一个在线向量检索服务做 SIMD 加速：核心热点函数 `dot_product` 需要处理上万次浮点点积。团队在 macOS (Apple Silicon，使用 Rosetta x86_64) 和一台 AVX2 支持的 Ubuntu 构建机上进行性能调优，通过在 `.cargo/config.toml` 中写 `RUSTFLAGS = ["-C", "target-cpu=native"]` 并在代码里添加 `#[cfg(target_feature = "avx2")]` 分支来启用 AVX2 指令。CI 使用 GitHub Actions（`ubuntu-22.04` x86_64），生产环境则混布自研节点与云厂商实例，其中一部分老旧机器只有 SSE4.2。

在压测中 release 二进制表现极佳，于是我们直接将构建好的产物发布到集群。上线后不到 5 分钟，多台老机器同时报 `Illegal instruction`，服务实例被 systemd 重启，导致上游网关出现 5xx 峰值。回滚后问题消失，但我们需要一个既维持优化、又保证跨 CPU 安全的方案。

## 08. 问题描述

- **目标**：让 `dot_product` 在支持 AVX2 的 CPU 上走 SIMD 快路径，实现 >30% 的吞吐提升，同时保持单个可执行文件在所有部署 CPU 上运行。
- **困难**：`#[cfg(target_feature = "avx2")]` 仅根据编译时 CPU 能力决定分支，配合 `-C target-cpu=native` 会在构建机上选择 AVX2 路径，即使目标部署 CPU 不支持；CI 和开发机不会暴露问题，因为它们有 AVX2。
- **最困惑**：为什么 release 构建在 debug/本地都正常，一旦放到老机器就 SIGILL？即使我们写了 `#[cfg(not(target_feature = "avx2"))]` 的回退分支，运行时仍然直接崩溃。

## 09. 问题代码或问题详细描述

见 `broken-example/`：

- `.cargo/config.toml` 强制 `-C target-cpu=native`
- `src/main.rs` 的 `dot_product` 根据 `cfg!(target_feature = "avx2")` 选择 AVX2 专用实现
- build 机器拥有 AVX2，最终 release 二进制包含 `vmulps` 等 AVX 指令；部署在无 AVX2 的节点上时立刻触发 SIGILL

## 10. 错误信息

在无 AVX2 的 CPU 或使用 `qemu-x86_64 -cpu qemu64` 运行 `cargo build --release` 生成的产物，输出：

```text
Illegal instruction (core dumped)
```

`gdb` 回溯：

```text
Thread 1 received signal SIGILL, Illegal instruction.
0x0000555555559620 in dot_product_avx2 ()
=> 0x555555559620 <dot_product_avx2+32>:  c5 f4 59 c1    vmulps %ymm1,%ymm0,%ymm0
```

## 11. 解决问题的过程

1. **快速验证**：去掉 `-C target-cpu=native` 即恢复稳定，但吞吐下降约 32%。确认崩溃与 AVX 相关。
2. **排查编译信息**：使用 `rustc --print cfg`、`readelf -A`，发现编译出的二进制仍标记为通用 x86_64，没有自动记录需要 AVX2 —— 说明运行时完全不会检查目标 CPU。
3. **调研资料**：阅读《The Rust Reference: Target Feature》、`std::arch` 文档、issue [rust-lang/rust#44839](https://github.com/rust-lang/rust/issues/44839)。结论：编译时 `target_feature` 仅表示“允许使用该指令”，不会插入运行时检测。
4. **尝试方案**：
   - 独立构建“AVX2 版”和“通用版”二进制，通过部署时选择。问题：增加运维负担，且函数签名失去统一。
   - 在关键函数外部包裹 `std::is_x86_feature_detected!`；初版写成 `if cfg!(target_feature = "avx2") && std::is_x86_feature_detected!("avx2")`，但调试发现 `cfg!` 在编译阶段计算，会让分支被常量折叠掉。
5. **最终策略**：
   - 移除全局 `-C target-cpu=native`
   - 保留 `#[target_feature(enable = "avx2")]` 的实现，但通过 **纯运行时** 检测选择代码路径：

    ```rust
    if std::is_x86_feature_detected!("avx2") {
       unsafe { dot_product_avx2(lhs, rhs) }
    } else {
       dot_product_scalar(lhs, rhs)
    }
    ```

   - 对于需要额外调优的场景，提供可选 Cargo feature，在性能实验时通过 `cargo build --release --features force-avx2` 来验证。
6. **CI 增强**：
   - GitHub Actions matrix 增加 `qemu` 测试：`qemu-x86_64 -cpu qemu64` 运行 release 二进制，确保在无 AVX2 下不会崩溃
   - 在 AVX2 runner 上运行基准测试，确认性能保留

## 12. 解决方案

见 `correct-example/`：

- 不再依赖全局 `target-cpu`；二进制默认面向通用 x86_64
- `dot_product` 在运行时调用 `std::is_x86_feature_detected!("avx2")`，仅在检测到 AVX2 时进入 `#[target_feature]` 函数
- fallback 使用纯 scalar 实现，确保在 legacy CPU 上安全运行
- README 展示如何在 AVX2、有/无 AVX2 环境下验证与基准

## 13. 补充说明

- 参考资料：
  - [Rust Reference: Target Feature](https://doc.rust-lang.org/reference/attributes/codegen.html#the-target_feature-attribute)
  - [std::arch 条件检测宏示例](https://doc.rust-lang.org/std/arch/macro.is_x86_feature_detected.html)
  - Intel® 64 and IA-32 Architectures Software Developer's Manual: CPUID 特征位
- 工具：`qemu-x86_64`、`cargo-llvm-lines` 验证代码大小、`perf stat` 度量 AVX2 性能提升
- 后续计划：补充 property-based 测试验证 SIMD 与 scalar 结果一致；在部署流水线里显式标记“需要 AVX2”或“通用”版本。
