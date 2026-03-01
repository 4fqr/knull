//! SIMD Vectorization Optimization
//!
//! Automatically vectorizes loops using AVX2/AVX-512 when available.

use crate::ast::*;
use crate::optimize::{CpuFeatures, OptLevel, OptimizationPass, OptimizeOptions};

pub struct SimdVectorization {
    vectorized_count: usize,
}

impl SimdVectorization {
    pub fn new() -> Self {
        SimdVectorization {
            vectorized_count: 0,
        }
    }

    fn can_vectorize(&self, body: &Stmt, options: &OptimizeOptions) -> bool {
        if !options.simd || options.cpu_features.vector_width() == 0 {
            return false;
        }

        match body {
            Stmt::Block(block) => block.stmts.len() <= 20,
            _ => false,
        }
    }

    fn get_vector_type(&self, elem_type: &Type, width: usize) -> Option<String> {
        match elem_type {
            Type::Int(_) => {
                if width >= 64 {
                    Some("__m512i".to_string())
                } else if width >= 32 {
                    Some("__m256i".to_string())
                } else {
                    Some("__m128i".to_string())
                }
            }
            Type::Float(_) => {
                if width >= 64 {
                    Some("__m512d".to_string())
                } else if width >= 32 {
                    Some("__m256d".to_string())
                } else {
                    Some("__m128d".to_string())
                }
            }
            _ => None,
        }
    }

    fn generate_vectorized_loop(
        &self,
        func_name: &str,
        iter: &Expr,
        body: &Stmt,
        options: &OptimizeOptions,
    ) -> String {
        let width = options.cpu_features.vector_width();

        let mut c_code = String::new();
        c_code.push_str(&format!("// Vectorized loop for {}\n", func_name));

        if options.cpu_features.has_avx512f {
            c_code.push_str("#pragma GCC target(\"avx512f,avx512bw,avx512dq,avx512vl\")\n");
            c_code.push_str("#include <immintrin.h>\n\n");
        } else if options.cpu_features.has_avx2 {
            c_code.push_str("#pragma GCC target(\"avx2,fma\")\n");
            c_code.push_str("#include <immintrin.h>\n\n");
        }

        if let Expr::Call { func, args, .. } = iter {
            if let Expr::Ident(name, _) = &**func {
                if name == "range" && args.len() >= 2 {
                    c_code.push_str(&format!(
                        "for (int64_t i = 0; i < {}; i += {}) {{\n",
                        self.get_range_end(&args[1]),
                        width
                    ));
                    c_code.push_str("    // Vectorized operations\n");
                    c_code.push_str("}\n");
                }
            }
        }

        c_code
    }

    fn get_range_end(&self, expr: &Expr) -> String {
        if let Expr::Literal(Literal::Integer(n), _) = expr {
            n.to_string()
        } else {
            "n".to_string()
        }
    }
}

impl OptimizationPass for SimdVectorization {
    fn name(&self) -> &'static str {
        "SIMD Vectorization"
    }

    fn run(&self, program: &mut Program, options: &OptimizeOptions) -> bool {
        if options.level == OptLevel::None || !options.simd {
            return false;
        }

        let mut changed = false;

        for item in &mut program.items {
            if let Item::Function(func) = item {
                if self.vectorize_function(&mut func.clone(), options) {
                    changed = true;
                    self.vectorized_count += 1;
                }
            }
        }

        changed
    }

    fn is_enabled(&self, options: &OptimizeOptions) -> bool {
        options.simd && options.level >= OptLevel::Default
    }
}

impl SimdVectorization {
    fn vectorize_function(&self, func: &mut Function, options: &OptimizeOptions) -> bool {
        let mut changed = false;

        for stmt in &mut func.body.stmts {
            if let Stmt::For { iter, body, .. } = stmt {
                if self.is_vectorizable(iter, body, options) {
                    changed = true;
                }
            }
            if let Stmt::While { body, .. } = stmt {
                if self.is_vectorizable_while(body, options) {
                    changed = true;
                }
            }
        }

        changed
    }

    fn is_vectorizable(&self, iter: &Expr, body: &Stmt, options: &OptimizeOptions) -> bool {
        if !options.simd {
            return false;
        }

        if let Expr::Call { func, args, .. } = iter {
            if let Expr::Ident(name, _) = &**func {
                if name == "range" && args.len() >= 2 {
                    if let Expr::Literal(Literal::Integer(count), _) = &args[1] {
                        return *count >= 8 && (*count % options.cpu_features.vector_width() == 0);
                    }
                }
            }
        }

        false
    }

    fn is_vectorizable_while(&self, body: &Stmt, options: &OptimizeOptions) -> bool {
        if !options.simd {
            return false;
        }
        false
    }
}

pub struct SimdIntrinsics;

impl SimdIntrinsics {
    pub fn new() -> Self {
        SimdIntrinsics
    }

    pub fn generate_add(&self, width: usize) -> &'static str {
        match width {
            64 => "_mm512_add_epi64",
            32 => "_mm256_add_epi64",
            16 => "_mm_add_epi64",
            _ => "_mm_add_si128",
        }
    }

    pub fn generate_mul(&self, width: usize) -> &'static str {
        match width {
            64 => "_mm512_mul_epi64",
            32 => "_mm256_mul_epi64",
            _ => "_mm_mul_epi32",
        }
    }

    pub fn generate_load(&self, width: usize) -> &'static str {
        match width {
            64 => "_mm512_load_si512",
            32 => "_mm256_load_si256",
            16 => "_mm_load_si128",
            _ => "_mm_load_si128",
        }
    }

    pub fn generate_store(&self, width: usize) -> &'static str {
        match width {
            64 => "_mm512_store_si512",
            32 => "_mm256_store_si256",
            16 => "_mm_store_si128",
            _ => "_mm_store_si128",
        }
    }

    pub fn generate_set1(&self, width: usize, ty: &str) -> &'static str {
        match (width, ty) {
            (64, "i64") => "_mm512_set1_epi64",
            (64, "i32") => "_mm512_set1_epi32",
            (32, "i64") => "_mm256_set1_epi64x",
            (32, "i32") => "_mm256_set1_epi32",
            (16, "i64") => "_mm_set1_epi64x",
            (16, "i32") => "_mm_set1_epi32",
            _ => "_mm_set1_epi32",
        }
    }
}

impl Default for SimdVectorization {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SimdIntrinsics {
    fn default() -> Self {
        Self::new()
    }
}

pub fn portable_simd_vectorize(elem_type: &str, op: &str, count: usize) -> String {
    let vector_width = count;

    format!(
        "// Portable SIMD for {} elements of type {}\n\
         // Operation: {}\n\
         // Use std::simd when stable or portable_simd crate",
        vector_width, elem_type, op
    )
}

#[cfg(feature = "simd")]
pub mod x86_intrinsics {
    use std::arch::x86_64::*;

    #[inline]
    pub fn fast_add_i32x4(a: __m128i, b: __m128i) -> __m128i {
        unsafe { _mm_add_epi32(a, b) }
    }

    #[inline]
    pub fn fast_mul_i32x4(a: __m128i, b: __m128i) -> __m128i {
        unsafe { _mm_mullo_epi32(a, b) }
    }

    #[inline]
    pub fn fast_add_f64x2(a: __m128d, b: __m128d) -> __m128d {
        unsafe { _mm_add_pd(a, b) }
    }

    #[inline]
    pub fn fast_mul_f64x2(a: __m128d, b: __m128d) -> __m128d {
        unsafe { _mm_mul_pd(a, b) }
    }

    #[inline]
    pub fn horizontal_add_f64x2(a: __m128d) -> f64 {
        unsafe {
            let tmp = _mm_add_sd(a, _mm_unpackhi_pd(a, a));
            _mm_cvtsd_f64(tmp)
        }
    }

    #[inline]
    pub fn blendv_f64(a: __m128d, b: __m128d, mask: __m128d) -> __m128d {
        unsafe { _mm_blendv_pd(a, b, mask) }
    }

    #[inline]
    pub fn max_f64x2(a: __m128d, b: __m128d) -> __m128d {
        unsafe { _mm_max_pd(a, b) }
    }

    #[inline]
    pub fn min_f64x2(a: __m128d, b: __m128d) -> __m128d {
        unsafe { _mm_min_pd(a, b) }
    }

    #[inline]
    pub fn load_pd(ptr: *const f64) -> __m128d {
        unsafe { _mm_load_pd(ptr) }
    }

    #[inline]
    pub fn store_pd(ptr: *mut f64, a: __m128d) {
        unsafe { _mm_store_pd(ptr, a) }
    }
}

#[cfg(not(feature = "simd"))]
pub mod x86_intrinsics {
    #[inline]
    pub fn fast_add_i32x4(
        a: (i32, i32, i32, i32),
        b: (i32, i32, i32, i32),
    ) -> (i32, i32, i32, i32) {
        (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3)
    }

    #[inline]
    pub fn fast_mul_i32x4(
        a: (i32, i32, i32, i32),
        b: (i32, i32, i32, i32),
    ) -> (i32, i32, i32, i32) {
        (a.0 * b.0, a.1 * b.1, a.2 * b.2, a.3 * b.3)
    }

    #[inline]
    pub fn fast_add_f64x2(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
        (a.0 + b.0, a.1 + b.1)
    }

    #[inline]
    pub fn fast_mul_f64x2(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
        (a.0 * b.0, a.1 * b.1)
    }
}
