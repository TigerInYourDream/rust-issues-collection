# Dot Product SIMD (Broken)

This crate demonstrates a production regression we hit after enabling AVX2 optimizations with `target-cpu=native`. The resulting release binary segfaults (`SIGILL`) on hosts whose CPUs do not support AVX2.

## Tested Environment

- rustc 1.80.0 (or later stable)
- cargo 1.80.0
- Target triple: `x86_64-unknown-linux-gnu`
- Host with AVX2 available (for compilation)
- To reproduce locally without legacy hardware you can use `qemu-x86_64` with the `qemu64` CPU model (lacks AVX extensions).

## Reproducing the Illegal Instruction

1. Build the release binary on any AVX2-capable machine:

   ```bash
   cargo build --release
   ```

2. Run the binary under a CPU model without AVX2 support. Two options:
   - **Real hardware**: copy `target/release/dot-prod-simd-broken` to an older Intel host (e.g. pre-Haswell) or a cloud VM without AVX2, then execute it.
   - **Emulation (recommended for demo)**:

     ```bash
     qemu-x86_64 -cpu qemu64 target/release/dot-prod-simd-broken
     ```

3. Observe the crash:

   ```text
   Illegal instruction (core dumped)
   ```

   With `gdb` or `lldb` you can confirm the faulting instruction is `vmulps` emitted by the AVX2 path.

## Why It Breaks

The code relies on `#[cfg(target_feature = "avx2")]` and a global `-C target-cpu=native` flag. The crate compiles the AVX2 path because the build host supports the feature. When the resulting binary is shipped to a CPU lacking AVX2, the process traps immediately when the optimizer executes the emitted AVX instruction sequence.

## Next Steps

- Examine `src/main.rs` and look at the `dot_product` implementation that blindly selects the AVX2 specialization at compile time.
- The fixed project introduces runtime feature detection and safe dispatch.
