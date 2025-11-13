#![cfg(target_arch = "x86_64")]

use std::arch::x86_64::*;
use std::hint::black_box;

fn main() {
    // Two fixed vectors so the optimizer cannot fold the computation away.
    let lhs = black_box([1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let rhs = black_box([0.5f32, 1.5, -2.0, 3.25, 4.75, -5.5, 6.125, 7.875]);

    // This will unconditionally pick the AVX2 specialization whenever the
    // build host CPU exposes the feature. The release binary will fault on
    // CPUs that lack AVX2 support.
    let dot = dot_product(&lhs, &rhs);
    println!("dot_product: {dot:.4}");
}

#[inline(always)]
fn dot_product(lhs: &[f32; 8], rhs: &[f32; 8]) -> f32 {
    if cfg!(target_feature = "avx2") {
        unsafe { dot_product_avx2(lhs, rhs) }
    } else {
        dot_product_scalar(lhs, rhs)
    }
}

#[target_feature(enable = "avx2")]
unsafe fn dot_product_avx2(lhs: &[f32; 8], rhs: &[f32; 8]) -> f32 {
    let a = _mm256_loadu_ps(lhs.as_ptr());
    let b = _mm256_loadu_ps(rhs.as_ptr());
    let mul = _mm256_mul_ps(a, b);
    horizontal_sum(mul)
}

unsafe fn horizontal_sum(v: __m256) -> f32 {
    let high = _mm256_extractf128_ps(v, 1);
    let low = _mm256_castps256_ps128(v);
    let sum = _mm_add_ps(high, low);
    let shuf = _mm_movehdup_ps(sum);
    let sums = _mm_add_ps(sum, shuf);
    let shuf = _mm_movehl_ps(shuf, sums);
    let sums = _mm_add_ss(sums, shuf);
    _mm_cvtss_f32(sums)
}

fn dot_product_scalar(lhs: &[f32; 8], rhs: &[f32; 8]) -> f32 {
    lhs.iter().zip(rhs.iter()).map(|(l, r)| l * r).sum()
}
