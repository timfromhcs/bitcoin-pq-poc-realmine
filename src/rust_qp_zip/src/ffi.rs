//! C-FFI bindings for the BIP-QP-ZIP library.
//! 
//! This module exposes the Rust cryptographic primitives to C++ via a stable C ABI.
//! It handles memory safety, error translation, and pointer marshaling.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_double, c_int};
use core::slice;
use crate::extractor::Extractor;
use crate::quantizer::{Quantizer, LATTICE_DIMENSION};
use crate::{QPZipError, QPZipContext};

/// Create a new Extractor instance
#[no_mangle]
pub extern "C" fn qp_zip_extractor_new(scale: c_double, crs_seed: *const u8, crs_seed_len: usize) -> *mut Extractor {
    if crs_seed.is_null() || crs_seed_len == 0 {
        return core::ptr::null_mut();
    }
    let seed = unsafe { slice::from_raw_parts(crs_seed, crs_seed_len) };
    let extractor = Box::new(Extractor::new(scale, seed));
    Box::into_raw(extractor)
}

/// Free an Extractor instance
#[no_mangle]
pub extern "C" fn qp_zip_extractor_free(extractor: *mut Extractor) {
    if !extractor.is_null() {
        unsafe {
            let _ = Box::from_raw(extractor);
        }
    }
}

/// Extract and validate a QP-ZIP witness program
/// 
/// Returns 0 on success, or a non-zero error code on failure.
/// Reconstructed signature is written to `out_reconstructed` (must be pre-allocated with LATTICE_DIMENSION doubles).
#[no_mangle]
pub extern "C" fn qp_zip_extract_and_validate(
    extractor: *mut Extractor,
    witness_program: *const u8,
    witness_program_len: usize,
    message: *const u8,
    message_len: usize,
    out_reconstructed: *mut c_double,
) -> c_int {
    if extractor.is_null() || witness_program.is_null() || message.is_null() || out_reconstructed.is_null() {
        return QPZipError::InvalidInput.as_i32();
    }

    let extractor = unsafe { &*extractor };
    let program = unsafe { slice::from_raw_parts(witness_program, witness_program_len) };
    let msg = unsafe { slice::from_raw_parts(message, message_len) };

    match extractor.extract_and_validate(program, msg) {
        Ok(reconstructed) => {
            if reconstructed.len() != LATTICE_DIMENSION {
                return QPZipError::ExtractionFailed.as_i32();
            }
            unsafe {
                core::ptr::copy_nonoverlapping(
                    reconstructed.as_ptr(),
                    out_reconstructed,
                    LATTICE_DIMENSION,
                );
            }
            QPZipError::Success.as_i32()
        }
        Err(err) => err.as_i32(),
    }
}

/// Helper to serialize a compressed signature for testing
/// 
/// Returns 0 on success, or a non-zero error code on failure.
/// Serialized program is written to `out_program` (must be pre-allocated with the correct size).
/// The required size is 32 (commitment) + 128 (proof) + LATTICE_DIMENSION * 4 (quantized) + LATTICE_DIMENSION * 8 (residuals) = 3232 bytes.
#[no_mangle]
pub extern "C" fn qp_zip_serialize_compressed(
    extractor: *mut Extractor,
    pubkey_commitment: *const u8, // 32 bytes
    quantized: *const i32, // LATTICE_DIMENSION elements
    residuals: *const c_double, // LATTICE_DIMENSION elements
    message: *const u8,
    message_len: usize,
    out_program: *mut u8,
    out_program_len: *mut usize,
) -> c_int {
    if extractor.is_null() || pubkey_commitment.is_null() || quantized.is_null() || residuals.is_null() || message.is_null() || out_program.is_null() || out_program_len.is_null() {
        return QPZipError::InvalidInput.as_i32();
    }

    let extractor = unsafe { &*extractor };
    let mut commitment = [0u8; 32];
    unsafe {
        core::ptr::copy_nonoverlapping(pubkey_commitment, commitment.as_mut_ptr(), 32);
    }

    let q_vec = unsafe { slice::from_raw_parts(quantized, LATTICE_DIMENSION) };
    let r_vec = unsafe { slice::from_raw_parts(residuals, LATTICE_DIMENSION) };
    let msg = unsafe { slice::from_raw_parts(message, message_len) };

    match extractor.serialize_compressed(&commitment, q_vec, r_vec, msg) {
        Ok(program) => {
            let req_len = program.len();
            unsafe {
                if *out_program_len < req_len {
                    *out_program_len = req_len;
                    return QPZipError::MemoryError.as_i32();
                }
                core::ptr::copy_nonoverlapping(program.as_ptr(), out_program, req_len);
                *out_program_len = req_len;
            }
            QPZipError::Success.as_i32()
        }
        Err(err) => err.as_i32(),
    }
}

// ============================================================================
// Quantizer FFI Functions
// ============================================================================

/// Create a new Quantizer instance
/// 
/// @param scale The scaling factor for quantization
/// @return A pointer to the new Quantizer, or NULL on failure
#[no_mangle]
pub extern "C" fn qp_zip_quantizer_new(scale: c_double) -> *mut Quantizer {
    let quantizer = Box::new(Quantizer::new(scale));
    Box::into_raw(quantizer)
}

/// Free a Quantizer instance
/// 
/// @param quantizer The Quantizer to free
#[no_mangle]
pub extern "C" fn qp_zip_quantizer_free(quantizer: *mut Quantizer) {
    if !quantizer.is_null() {
        unsafe {
            let _ = Box::from_raw(quantizer);
        }
    }
}

/// Quantize a high-dimensional vector into discrete lattice points
/// 
/// @param quantizer The Quantizer instance
/// @param input The input vector (array of doubles)
/// @param input_len The length of the input vector (must be LATTICE_DIMENSION)
/// @param out_quantized Output buffer for quantized values (must be pre-allocated with input_len i32s)
/// @param out_residuals Output buffer for residual values (must be pre-allocated with input_len doubles)
/// @return 0 on success, or a non-zero error code on failure
#[no_mangle]
pub extern "C" fn qp_zip_quantize(
    quantizer: *mut Quantizer,
    input: *const c_double,
    input_len: usize,
    out_quantized: *mut i32,
    out_residuals: *mut c_double,
) -> c_int {
    if quantizer.is_null() || input.is_null() || out_quantized.is_null() || out_residuals.is_null() {
        return QPZipError::InvalidInput.as_i32();
    }

    let quantizer = unsafe { &*quantizer };
    let input_slice = unsafe { slice::from_raw_parts(input, input_len) };

    // Convert input to Vec<f64>
    let input_vec: Vec<f64> = input_slice.iter().map(|&x| x as f64).collect();

    match quantizer.quantize(&input_vec) {
        Ok((quantized, residuals)) => {
            unsafe {
                core::ptr::copy_nonoverlapping(quantized.as_ptr(), out_quantized, quantized.len());
                core::ptr::copy_nonoverlapping(residuals.as_ptr(), out_residuals, residuals.len());
            }
            QPZipError::Success.as_i32()
        }
        Err(err) => err.as_i32(),
    }
}

/// Reconstruct the original vector from quantized points and residuals
/// 
/// @param quantizer The Quantizer instance
/// @param quantized The quantized values (array of i32s)
/// @param residuals The residual values (array of doubles)
/// @param input_len The length of the vectors (must be LATTICE_DIMENSION)
/// @param out_reconstructed Output buffer for reconstructed values (must be pre-allocated with input_len doubles)
/// @return 0 on success, or a non-zero error code on failure
#[no_mangle]
pub extern "C" fn qp_zip_reconstruct(
    quantizer: *mut Quantizer,
    quantized: *const i32,
    residuals: *const c_double,
    input_len: usize,
    out_reconstructed: *mut c_double,
) -> c_int {
    if quantizer.is_null() || quantized.is_null() || residuals.is_null() || out_reconstructed.is_null() {
        return QPZipError::InvalidInput.as_i32();
    }

    let quantizer = unsafe { &*quantizer };
    let quantized_slice = unsafe { slice::from_raw_parts(quantized, input_len) };
    let residuals_slice = unsafe { slice::from_raw_parts(residuals, input_len) };

    match quantizer.reconstruct(quantized_slice, residuals_slice) {
        Ok(reconstructed) => {
            unsafe {
                core::ptr::copy_nonoverlapping(reconstructed.as_ptr(), out_reconstructed, reconstructed.len());
            }
            QPZipError::Success.as_i32()
        }
        Err(err) => err.as_i32(),
    }
}
