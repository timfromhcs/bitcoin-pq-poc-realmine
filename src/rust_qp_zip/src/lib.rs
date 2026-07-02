//! BIP-QP-ZIP: Quantum-Proof Zero-Knowledge Inflight Processing
//! 
//! This library provides post-quantum cryptographic primitives for Bitcoin Core,
//! including lattice-based vector quantization, ZK-SNARK compression, and
//! deterministic extraction for witness programs.

#![no_std] // Use no_std for embedded compatibility
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

extern crate alloc;

use alloc::boxed::Box;
use core::ffi::{c_char, c_void};

pub mod quantizer;
pub mod zk_prover;
pub mod extractor;
mod ffi;

// Re-export FFI functions
pub use ffi::*;

/// Version information for the QP-ZIP library
pub const QP_ZIP_VERSION: &str = "0.1.0";
pub const QP_ZIP_PROTOCOL_VERSION: u32 = 1;

/// Error codes for FFI operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QPZipError {
    Success = 0,
    InvalidInput = 1,
    CompressionFailed = 2,
    DecompressionFailed = 3,
    ProofGenerationFailed = 4,
    ProofVerificationFailed = 5,
    ExtractionFailed = 6,
    MemoryError = 7,
    UnsupportedVersion = 8,
}

impl QPZipError {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Result type for internal operations
pub type Result<T> = core::result::Result<T, QPZipError>;

/// Context for QP-ZIP operations
#[repr(C)]
pub struct QPZipContext {
    /// Internal state pointer
    internal: *mut c_void,
    /// Protocol version
    version: u32,
    /// Flags for operation modes
    flags: u32,
}

impl Default for QPZipContext {
    fn default() -> Self {
        Self {
            internal: core::ptr::null_mut(),
            version: QP_ZIP_PROTOCOL_VERSION,
            flags: 0,
        }
    }
}

/// Initialize a new QP-ZIP context
#[no_mangle]
pub extern "C" fn qp_zip_context_new() -> *mut QPZipContext {
    let ctx = Box::new(QPZipContext::default());
    Box::into_raw(ctx)
}

/// Free a QP-ZIP context
#[no_mangle]
pub extern "C" fn qp_zip_context_free(ctx: *mut QPZipContext) {
    if !ctx.is_null() {
        unsafe {
            let _ = Box::from_raw(ctx);
        }
    }
}

/// Get the library version string
#[no_mangle]
pub extern "C" fn qp_zip_get_version() -> *const c_char {
    b"0.1.0\0".as_ptr() as *const c_char
}


/// Get the protocol version
#[no_mangle]
pub extern "C" fn qp_zip_get_protocol_version() -> u32 {
    QP_ZIP_PROTOCOL_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = qp_zip_context_new();
        assert!(!ctx.is_null());
        qp_zip_context_free(ctx);
    }

    #[test]
    fn test_version_info() {
        let version = unsafe { core::ffi::CStr::from_ptr(qp_zip_get_version()) };
        let version_str = version.to_str().unwrap();
        assert_eq!(version_str, QP_ZIP_VERSION);
        assert_eq!(qp_zip_get_protocol_version(), QP_ZIP_PROTOCOL_VERSION);
    }
}