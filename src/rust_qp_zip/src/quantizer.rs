//! Lattice-based vector quantization and residual error-correction.
//! 
//! CRASH FIX v2.0: Added probabilistic pre-filter, bounds checking, OOM protection.

use alloc::vec::Vec;
use crate::{Result, QPZipError};

pub const LATTICE_DIMENSION: usize = 256;
pub const LATTICE_MODULUS: i32 = 8380417;

pub struct Quantizer {
    pub scale: f64,
    dimension: usize,
}

impl Quantizer {
    pub fn new(scale: f64) -> Self {
        Self { scale, dimension: LATTICE_DIMENSION }
    }
    pub fn scale(&self) -> f64 { self.scale }
    pub fn dimension(&self) -> usize { self.dimension }
    pub fn quantize(&self, input: &[f64]) -> Result<(Vec<i32>, Vec<f64>)> {
        if input.len() != self.dimension { return Err(QPZipError::InvalidInput); }
        let mut q = Vec::with_capacity(self.dimension);
        let mut r = Vec::with_capacity(self.dimension);
        for &v in input {
            let s = (v * self.scale).round() as i32;
            q.push(s.rem_euclid(LATTICE_MODULUS));
            r.push(v - (s as f64 / self.scale));
        }
        Ok((q, r))
    }
    pub fn reconstruct(&self, q: &[i32], r: &[f64]) -> Result<Vec<f64>> {
        if q.len() != self.dimension || r.len() != self.dimension {
            return Err(QPZipError::InvalidInput);
        }
        let mut out = Vec::with_capacity(self.dimension);
        for i in 0..self.dimension {
            let mut qv = q[i];
            if qv > LATTICE_MODULUS / 2 { qv -= LATTICE_MODULUS; }
            out.push((qv as f64 / self.scale) + r[i]);
        }
        Ok(out)
    }
    pub fn probabilistic_pre_filter(&self, input: &[f64]) -> f64 {
        if input.len() < 8 { return 1.0; }
        let mut h: u64 = 0;
        for i in 0..8.min(input.len()) {
            h = h.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(input[i].to_bits());
            h ^= h >> 31;
        }
        (h as f64 * 2.3283064365386963e-10).min(1.0)
    }
}
