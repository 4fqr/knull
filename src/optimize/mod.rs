//! Knull Optimization Module
//!
//! This module provides advanced compiler optimizations including:
//! - LTO (Link-Time Optimization)
//! - SIMD Vectorization
//! - Inline Expansion
//! - Constant Folding
//! - Dead Code Elimination
//! - Loop Optimizations
//! - Escape Analysis

pub mod analysis;
pub mod constants;
pub mod loops;
pub mod simd;

pub use analysis::*;
pub use constants::*;
pub use loops::*;
pub use simd::*;
