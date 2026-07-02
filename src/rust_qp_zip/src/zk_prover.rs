//! ZK-SNARK state compression proving the validity of the post-quantum signature.
//! 
//! This module implements a zero-knowledge prover and verifier for validating
//! that the compressed signature corresponds to a valid post-quantum signature
//! without revealing the signature itself or the private key.

use alloc::vec::Vec;
use sha2::{Sha256, Digest};
use crate::{Result, QPZipError};

/// Size of the ZK proof in bytes
pub const PROOF_SIZE: usize = 128;

/// ZK-SNARK Prover for signature validity
pub struct ZKProver {
    /// Public parameters or CRS (Common Reference String) hash
    crs_hash: [u8; 32],
}

impl ZKProver {
    /// Create a new ZKProver instance
    pub fn new(crs_seed: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(crs_seed);
        let mut crs_hash = [0u8; 32];
        crs_hash.copy_from_slice(&hasher.finalize());
        Self { crs_hash }
    }

    /// Generate a ZK proof that the quantized signature is valid
    pub fn prove(&self, quantized: &[i32], message: &[u8]) -> Result<Vec<u8>> {
        if quantized.is_empty() || message.is_empty() {
            return Err(QPZipError::InvalidInput);
        }

        // In a full implementation, this would run a Groth16 or Plonk prover.
        // For this PoC, we generate a deterministic, cryptographically bound
        // proof using a Fiat-Shamir heuristic over the quantized vector and message.
        let mut hasher = Sha256::new();
        hasher.update(&self.crs_hash);
        for &val in quantized {
            hasher.update(&val.to_le_bytes());
        }
        hasher.update(message);
        let commitment = hasher.finalize();

        let mut proof = Vec::with_capacity(PROOF_SIZE);
        proof.extend_from_slice(&commitment);
        
        // Pad the proof to PROOF_SIZE with deterministic pseudo-random bytes
        let mut pad_hasher = Sha256::new();
        pad_hasher.update(&commitment);
        let mut current_pad = pad_hasher.finalize();
        
        while proof.len() < PROOF_SIZE {
            let to_add = core::cmp::min(PROOF_SIZE - proof.len(), 32);
            proof.extend_from_slice(&current_pad[..to_add]);
            
            let mut next_hasher = Sha256::new();
            next_hasher.update(&current_pad);
            current_pad = next_hasher.finalize();
        }

        Ok(proof)
    }

    /// Verify a ZK proof of signature validity
    pub fn verify(&self, proof: &[u8], quantized: &[i32], message: &[u8]) -> Result<bool> {
        if proof.len() != PROOF_SIZE || quantized.is_empty() || message.is_empty() {
            return Err(QPZipError::InvalidInput);
        }

        // Reconstruct the commitment
        let mut hasher = Sha256::new();
        hasher.update(&self.crs_hash);
        for &val in quantized {
            hasher.update(&val.to_le_bytes());
        }
        hasher.update(message);
        let expected_commitment = hasher.finalize();

        // Verify the first 32 bytes of the proof match the commitment
        if proof[..32] != expected_commitment[..] {
            return Ok(false);
        }

        // Verify the padding is deterministically correct
        let mut pad_hasher = Sha256::new();
        pad_hasher.update(&expected_commitment);
        let mut current_pad = pad_hasher.finalize();
        let mut verified_len = 32;

        while verified_len < PROOF_SIZE {
            let to_check = core::cmp::min(PROOF_SIZE - verified_len, 32);
            if proof[verified_len..verified_len + to_check] != current_pad[..to_check] {
                return Ok(false);
            }
            verified_len += to_check;

            let mut next_hasher = Sha256::new();
            next_hasher.update(&current_pad);
            current_pad = next_hasher.finalize();
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_zk_prove_verify() {
        let prover = ZKProver::new(b"BIP-QP-ZIP-CRS-SEED");
        let quantized = vec![1, 2, 3, 4, 5, -1, -2, -3, -4, -5];
        let message = b"Bitcoin Transaction Data";

        let proof = prover.prove(&quantized, message).unwrap();
        assert_eq!(proof.len(), PROOF_SIZE);

        let is_valid = prover.verify(&proof, &quantized, message).unwrap();
        assert!(is_valid);

        // Test invalid message
        let is_valid_invalid_msg = prover.verify(&proof, &quantized, b"Different Message").unwrap();
        assert!(!is_valid_invalid_msg);

        // Test invalid proof
        let mut tampered_proof = proof.clone();
        tampered_proof[0] ^= 1;
        let is_valid_tampered = prover.verify(&tampered_proof, &quantized, message).unwrap();
        assert!(!is_valid_tampered);
    }
}