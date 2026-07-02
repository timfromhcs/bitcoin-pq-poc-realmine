//! Deterministic extraction logic exposed as a C-FFI static library.
//!
//! CRASH FIX v2.0: Added bounds checking on all slice accesses, safe fallible conversions.

use alloc::vec::Vec;
use crate::quantizer::{Quantizer, LATTICE_DIMENSION};
use crate::zk_prover::{ZKProver, PROOF_SIZE};
use crate::{Result, QPZipError};

pub struct Extractor {
    quantizer: Quantizer,
    prover: ZKProver,
}

impl Extractor {
    pub fn new(scale: f64, crs_seed: &[u8]) -> Self {
        Self { quantizer: Quantizer::new(scale), prover: ZKProver::new(crs_seed) }
    }

    fn validate_len(wp: &[u8], required: usize) -> Result<()> {
        if wp.len() < required { return Err(QPZipError::InvalidInput); }
        Ok(())
    }

    fn read_i24(wp: &[u8], offset: usize) -> Result<i32> {
        if offset + 3 > wp.len() { return Err(QPZipError::InvalidInput); }
        let mut bytes = [0u8; 4];
        bytes[0..3].copy_from_slice(&wp[offset..offset + 3]);
        let mut val = i32::from_le_bytes(bytes);
        if val & 0x00800000 != 0 { val |= 0xFF000000u32 as i32; }
        Ok(val)
    }

    fn read_f32(wp: &[u8], offset: usize) -> Result<f64> {
        if offset + 4 > wp.len() { return Err(QPZipError::InvalidInput); }
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&wp[offset..offset + 4]);
        Ok(f32::from_le_bytes(bytes) as f64)
    }

    pub fn extract_and_validate(&self, wp: &[u8], msg: &[u8]) -> Result<Vec<f64>> {
        let min = 32 + PROOF_SIZE + LATTICE_DIMENSION * 3 + LATTICE_DIMENSION * 4;
        Self::validate_len(wp, min)?;
        let proof = &wp[32..32 + PROOF_SIZE];
        let mut quantized = Vec::with_capacity(LATTICE_DIMENSION);
        for i in 0..LATTICE_DIMENSION {
            quantized.push(Self::read_i24(wp, 32 + PROOF_SIZE + i * 3)?);
        }
        let rstart = 32 + PROOF_SIZE + LATTICE_DIMENSION * 3;
        let mut residuals = Vec::with_capacity(LATTICE_DIMENSION);
        for i in 0..LATTICE_DIMENSION {
            residuals.push(Self::read_f32(wp, rstart + i * 4)?);
        }
        if !self.prover.verify(proof, &quantized, msg)? {
            return Err(QPZipError::ProofVerificationFailed);
        }
        self.quantizer.reconstruct(&quantized, &residuals)
    }

    pub fn serialize_compressed(&self, pk: &[u8; 32], quantized: &[i32], residuals: &[f64], msg: &[u8]) -> Result<Vec<u8>> {
        if quantized.len() != LATTICE_DIMENSION || residuals.len() != LATTICE_DIMENSION {
            return Err(QPZipError::InvalidInput);
        }
        let mut program = Vec::new();
        program.extend_from_slice(pk);
        program.extend_from_slice(&self.prover.prove(quantized, msg)?);
        for &v in quantized { program.extend_from_slice(&v.to_le_bytes()[0..3]); }
        for &v in residuals { program.extend_from_slice(&(v as f32).to_le_bytes()); }
        Ok(program)
    }
}

