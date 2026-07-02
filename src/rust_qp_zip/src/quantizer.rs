//! Lattice-based vector quantization and residual error-correction.
//! 
//! This module implements lattice-based compression for post-quantum signatures.
//! It reduces the storage footprint of high-dimensional lattice vectors by
//! projecting them onto a discrete lattice and encoding the residual errors.

use alloc::vec::Vec;
use crate::{Result, QPZipError};

/// Lattice dimension for quantization
pub const LATTICE_DIMENSION: usize = 256;
/// Modulus for lattice coefficients
pub const LATTICE_MODULUS: i32 = 8380417; // Dilithium-style modulus Q

/// Quantizer state and parameters
pub struct Quantizer {
    /// Scaling factor for quantization
    scale: f64,
    /// Lattice dimension
    dimension: usize,
}

impl Quantizer {
    /// Create a new Quantizer instance
    pub fn new(scale: f64) -> Self {
        Self {
            scale,
            dimension: LATTICE_DIMENSION,
        }
    }

    /// Quantize a high-dimensional vector into discrete lattice points
    pub fn quantize(&self, input: &[f64]) -> Result<(Vec<i32>, Vec<f64>)> {
        if input.len() != self.dimension {
            return Err(QPZipError::InvalidInput);
        }

        let mut quantized = Vec::with_capacity(self.dimension);
        let mut residuals = Vec::with_capacity(self.dimension);

        for &val in input {
            // Project onto discrete lattice
            let scaled = val * self.scale;
            let rounded = scaled.round() as i32;
            
            // Keep within modulus bounds
            let bounded = rounded.rem_euclid(LATTICE_MODULUS);
            quantized.push(bounded);

            // Calculate residual error
            let residual = val - (rounded as f64 / self.scale);
            residuals.push(residual);
        }

        Ok((quantized, residuals))
    }

    /// Reconstruct the original vector from quantized points and residuals
    pub fn reconstruct(&self, quantized: &[i32], residuals: &[f64]) -> Result<Vec<f64>> {
        if quantized.len() != self.dimension || residuals.len() != self.dimension {
            return Err(QPZipError::InvalidInput);
        }

        let mut reconstructed = Vec::with_capacity(self.dimension);

        for i in 0..self.dimension {
            let mut q_val = quantized[i];
            if q_val > LATTICE_MODULUS / 2 {
                q_val -= LATTICE_MODULUS;
            }
            let r_val = residuals[i];

            // Reconstruct original value
            let val = (q_val as f64 / self.scale) + r_val;
            reconstructed.push(val);
        }

        Ok(reconstructed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantization_roundtrip() {
        let quantizer = Quantizer::new(1024.0);
        let mut input = [0.0; LATTICE_DIMENSION];
        for i in 0..LATTICE_DIMENSION {
            input[i] = (i as f64) * 0.12345;
        }

        let (quantized, residuals) = quantizer.quantize(&input).unwrap();
        let reconstructed = quantizer.reconstruct(&quantized, &residuals).unwrap();

        for i in 0..LATTICE_DIMENSION {
            assert!((input[i] - reconstructed[i]).abs() < 1e-9);
        }
    }
}