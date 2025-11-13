use std::hint::black_box;

fn main() {
    let lhs = black_box([1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let rhs = black_box([0.5f32, 1.5, -2.0, 3.25, 4.75, -5.5, 6.125, 7.875]);

    // Runtime feature detection keeps the binary portable across CPUs with
    // different SIMD capability levels.
    let dot = dot_product(&lhs, &rhs);
    println!("dot_product: {dot:.4}");
}

#[inline(always)]
fn dot_product(lhs: &[f32; 8], rhs: &[f32; 8]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            return unsafe { simd::dot_product_avx2(lhs, rhs) };
        }
    }

    dot_product_scalar(lhs, rhs)
}

fn dot_product_scalar(lhs: &[f32; 8], rhs: &[f32; 8]) -> f32 {
    lhs.iter().zip(rhs.iter()).map(|(l, r)| l * r).sum()
}

#[cfg(target_arch = "x86_64")]
mod simd {
    use std::arch::x86_64::*;

    #[target_feature(enable = "avx2")]
    pub unsafe fn dot_product_avx2(lhs: &[f32; 8], rhs: &[f32; 8]) -> f32 {
        let a = _mm256_loadu_ps(lhs.as_ptr());
        let b = _mm256_loadu_ps(rhs.as_ptr());
        let mul = _mm256_mul_ps(a, b);
        horizontal_sum(mul)
    }

    #[inline(always)]
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
}
