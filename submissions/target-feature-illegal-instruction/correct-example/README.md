# Dot Product SIMD (Fixed)

This crate illustrates the production fix: we keep the AVX2 optimization but gate it behind runtime CPU feature detection, keeping release binaries portable across heterogeneous fleets.

## Tested Environment

- rustc 1.80.0
- cargo 1.80.0
- Works on `x86_64-unknown-linux-gnu` regardless of the host CPU's SIMD capability

## Verification Steps

1. Build the release binary without forcing a particular `target-cpu`:

   ```bash
   cargo build --release
   ```

2. Run the binary on any x86_64 host, even those lacking AVX2 (e.g. `qemu-x86_64 -cpu qemu64 target/release/dot-prod-simd-fixed`).

3. Observe normal output:

   ```text
   dot_product: 161.7812
   ```

4. Optionally benchmark on AVX2-capable hardware; the runtime dispatcher still takes the fast path when the feature is present.
