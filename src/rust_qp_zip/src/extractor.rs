//! Deterministic extraction logic exposed as a C-FFI static library.
//! 
//! This module implements the extraction runtime that parses the witness program,
//! extracts the compressed post-quantum signature, and reconstructs the full
//! lattice-based signature for in-memory validation.

use alloc::vec::Vec;
use alloc::vec;
use crate::quantizer::{Quantizer, LATTICE_DIMENSION};
use crate::zk_prover::ZKProver;
use crate::{Result, QPZipError};

/// Extractor for QP-ZIP witness programs
pub struct Extractor {
    quantizer: Quantizer,
    prover: ZKProver,
}

impl Extractor {
    /// Create a new Extractor instance
    pub fn new(scale: f64, crs_seed: &[u8]) -> Self {
        Self {
            quantizer: Quantizer::new(scale),
            prover: ZKProver::new(crs_seed),
        }
    }

    /// Extract and validate a QP-ZIP witness program
    /// 
    /// Format of the witness program:
    /// - [0..32]: Public key commitment (hash of the lattice public key)
    /// - [32..160]: ZK proof of signature validity (128 bytes)
    /// - [160..160 + LATTICE_DIMENSION * 4]: Quantized signature vector (1024 bytes)
    /// - [160 + LATTICE_DIMENSION * 4..]: Residual error vector (8-byte doubles)
    /// Extract and validate a QP-ZIP witness program
    /// 
    /// Format of the witness program:
    /// - [0..32]: Public key commitment (hash of the lattice public key)
    /// - [32..160]: ZK proof of signature validity (128 bytes)
    /// - [160..160 + LATTICE_DIMENSION * 3]: Quantized signature vector (768 bytes, 3 bytes per int)
    /// - [160 + LATTICE_DIMENSION * 3..]: Residual error vector (1024 bytes, 4 bytes per float)
    pub fn extract_and_validate(
        &self,
        witness_program: &[u8],
        message: &[u8],
    ) -> Result<Vec<f64>> {
        let min_size = 32 + 128 + LATTICE_DIMENSION * 3 + LATTICE_DIMENSION * 4;
        if witness_program.len() < min_size {
            return Err(QPZipError::InvalidInput);
        }

        // 1. Extract public key commitment
        let _pubkey_commitment = &witness_program[0..32];

        // 2. Extract ZK proof
        let proof = &witness_program[32..160];

        // 3. Extract quantized signature vector (3 bytes each)
        let mut quantized = Vec::with_capacity(LATTICE_DIMENSION);
        for i in 0..LATTICE_DIMENSION {
            let start = 160 + i * 3;
            let mut bytes = [0u8; 4];
            bytes[0..3].copy_from_slice(&witness_program[start..start + 3]);
            let mut val = i32::from_le_bytes(bytes);
            if val & 0x00800000 != 0 {
                val |= 0xFF000000u32 as i32;
            }
            quantized.push(val);
        }

        // 4. Extract residual error vector (4 bytes each as f32)
        let residual_start = 160 + LATTICE_DIMENSION * 3;
        let mut residuals = Vec::with_capacity(LATTICE_DIMENSION);
        for i in 0..LATTICE_DIMENSION {
            let start = residual_start + i * 4;
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&witness_program[start..start + 4]);
            let val = f32::from_le_bytes(bytes) as f64;
            residuals.push(val);
        }

        // 5. Validate the ZK proof
        let is_valid_proof = self.prover.verify(proof, &quantized, message)?;
        if !is_valid_proof {
            return Err(QPZipError::ProofVerificationFailed);
        }

        // 6. Reconstruct the full lattice-based signature
        let reconstructed = self.quantizer.reconstruct(&quantized, &residuals)?;

        Ok(reconstructed)
    }

    /// Helper to serialize a compressed signature for testing
    pub fn serialize_compressed(
        &self,
        pubkey_commitment: &[u8; 32],
        quantized: &[i32],
        residuals: &[f64],
        message: &[u8],
    ) -> Result<Vec<u8>> {
        if quantized.len() != LATTICE_DIMENSION || residuals.len() != LATTICE_DIMENSION {
            return Err(QPZipError::InvalidInput);
        }

        let mut program = Vec::new();
        program.extend_from_slice(pubkey_commitment);

        // Generate ZK proof
        let proof = self.prover.prove(quantized, message)?;
        program.extend_from_slice(&proof);

        // Serialize quantized vector (3 bytes each)
        for &val in quantized {
            let bytes = val.to_le_bytes();
            program.extend_from_slice(&bytes[0..3]);
        }

        // Serialize residuals (4 bytes each as f32)
        for &val in residuals {
            let float_val = val as f32;
            program.extend_from_slice(&float_val.to_le_bytes());
        }

        Ok(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_roundtrip() {
        let extractor = Extractor::new(1024.0, b"BIP-QP-ZIP-CRS-SEED");
        let pubkey_commitment = [0x42u8; 32];
        let message = b"Bitcoin Transaction Data";

        let mut quantized = vec![0; LATTICE_DIMENSION];
        let mut residuals = vec![0.0; LATTICE_DIMENSION];
        for i in 0..LATTICE_DIMENSION {
            quantized[i] = i as i32;
            residuals[i] = (i as f64) * 0.0001;
        }

        let compressed = extractor.serialize_compressed(
            &pubkey_commitment,
            &quantized,
            &residuals,
            message,
        ).unwrap();

        let reconstructed = extractor.extract_and_validate(&compressed, message).unwrap();
        assert_eq!(reconstructed.len(), LATTICE_DIMENSION);

        for i in 0..LATTICE_DIMENSION {
            let expected = (quantized[i] as f64 / 1024.0) + residuals[i];
            assert!((reconstructed[i] - expected).abs() < 1e-7);
        }
    }
}