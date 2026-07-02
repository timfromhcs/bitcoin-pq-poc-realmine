// Copyright (c) 2023-present The Bitcoin Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or https://opensource.org/license/mit/.

#ifndef BITCOIN_QPZIP_H
#define BITCOIN_QPZIP_H

#include <cstdint>
#include <cstddef>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * BIP-QP-ZIP: Quantum-Proof Zero-Knowledge Inflight Processing
 *
 * This header defines the C interface to the rust_qp_zip library,
 * which provides post-quantum cryptographic primitives for Bitcoin Core.
 */

/// Error codes for QP-ZIP operations
typedef enum {
    QPZIP_SUCCESS = 0,
    QPZIP_INVALID_INPUT = 1,
    QPZIP_COMPRESSION_FAILED = 2,
    QPZIP_DECOMPRESSION_FAILED = 3,
    QPZIP_PROOF_GENERATION_FAILED = 4,
    QPZIP_PROOF_VERIFICATION_FAILED = 5,
    QPZIP_EXTRACTION_FAILED = 6,
    QPZIP_MEMORY_ERROR = 7,
    QPZIP_UNSUPPORTED_VERSION = 8,
} QPZipError;

/// Opaque context for QP-ZIP operations
typedef struct QPZipContext QPZipContext;

struct Extractor;
typedef struct Extractor Extractor;

/**
 * Initialize a new QP-ZIP context.
 * @return A new context pointer, or NULL on failure.
 */
QPZipContext* qp_zip_context_new(void);

/**
 * Free a QP-ZIP context.
 * @param ctx The context to free.
 */
void qp_zip_context_free(QPZipContext* ctx);

/**
 * Create a new Extractor instance.
 * @param scale The scaling factor for quantization.
 * @param crs_seed The Common Reference String seed.
 * @param crs_seed_len The length of the seed.
 * @return A pointer to a new Extractor, or NULL on failure.
 */
Extractor* qp_zip_extractor_new(double scale, const uint8_t* crs_seed, size_t crs_seed_len);

/**
 * Free an Extractor instance.
 * @param extractor The extractor to free.
 */
void qp_zip_extractor_free(Extractor* extractor);

/**
 * Get the library version string.
 * @return A pointer to a null-terminated version string.
 */
const char* qp_zip_get_version(void);

/**
 * Get the protocol version number.
 * @return The protocol version.
 */
uint32_t qp_zip_get_protocol_version(void);

/**
 * Compress a lattice-based signature vector.
 *
 * @param ctx The QP-ZIP context.
 * @param input The input vector (array of doubles).
 * @param input_len The length of the input vector.
 * @param output The output buffer for the compressed data.
 * @param output_len On input, the size of the output buffer.
 *                 On output, the number of bytes written.
 * @return QPZIP_SUCCESS on success, or an error code.
 */
QPZipError qp_zip_compress(
    QPZipContext* ctx,
    const double* input,
    size_t input_len,
    uint8_t* output,
    size_t* output_len
);

/**
 * Decompress a lattice-based signature vector.
 *
 * @param ctx The QP-ZIP context.
 * @param input The compressed data.
 * @param input_len The length of the compressed data.
 * @param output The output buffer for the decompressed vector.
 * @param output_len On input, the capacity of the output buffer in doubles.
 *                 On output, the number of doubles written.
 * @return QPZIP_SUCCESS on success, or an error code.
 */
QPZipError qp_zip_decompress(
    QPZipContext* ctx,
    const uint8_t* input,
    size_t input_len,
    double* output,
    size_t* output_len
);

/**
 * Generate a ZK proof for a quantized signature vector.
 *
 * @param ctx The QP-ZIP context.
 * @param quantized The quantized signature vector.
 * @param quantized_len The length of the quantized vector.
 * @param message The message to sign.
 * @param message_len The length of the message.
 * @param proof The output buffer for the proof.
 * @param proof_len On input, the size of the proof buffer.
 *                 On output, the number of bytes written.
 * @return QPZIP_SUCCESS on success, or an error code.
 */
QPZipError qp_zip_generate_proof(
    QPZipContext* ctx,
    const int32_t* quantized,
    size_t quantized_len,
    const uint8_t* message,
    size_t message_len,
    uint8_t* proof,
    size_t* proof_len
);

/**
 * Verify a ZK proof for a quantized signature vector.
 *
 * @param ctx The QP-ZIP context.
 * @param proof The proof data.
 * @param proof_len The length of the proof.
 * @param quantized The quantized signature vector.
 * @param quantized_len The length of the quantized vector.
 * @param message The message that was signed.
 * @param message_len The length of the message.
 * @param is_valid Output: 1 if the proof is valid, 0 otherwise.
 * @return QPZIP_SUCCESS on success, or an error code.
 */
QPZipError qp_zip_verify_proof(
    QPZipContext* ctx,
    const uint8_t* proof,
    size_t proof_len,
    const int32_t* quantized,
    size_t quantized_len,
    const uint8_t* message,
    size_t message_len,
    int* is_valid
);

/**
 * Extract and validate a QP-ZIP witness program.
 *
 * @param extractor The Extractor instance.
 * @param witness_program The witness program data.
 * @param witness_len The length of the witness program.
 * @param message The transaction data/message.
 * @param message_len The length of the message.
 * @param reconstructed_sig Output buffer for the reconstructed signature.
 * @return QPZIP_SUCCESS on success, or an error code.
 */
int qp_zip_extract_and_validate(
    Extractor* extractor,
    const uint8_t* witness_program,
    size_t witness_len,
    const uint8_t* message,
    size_t message_len,
    double* reconstructed_sig
);

/**
 * Serialize a compressed signature.
 * @return QPZIP_SUCCESS on success, or an error code.
 */
int qp_zip_serialize_compressed(
    Extractor* extractor,
    const uint8_t* pubkey_commitment,
    const int32_t* quantized,
    const double* residuals,
    const uint8_t* message,
    size_t message_len,
    uint8_t* out_program,
    size_t* out_program_len
);

/**
 * Create a new Quantizer instance for lattice-based vector quantization.
 *
 * @param scale The scaling factor for quantization.
 * @return A pointer to a new Quantizer, or NULL on failure.
 */
struct Quantizer* qp_zip_quantizer_new(double scale);

/**
 * Free a Quantizer instance.
 *
 * @param quantizer The Quantizer to free.
 */
void qp_zip_quantizer_free(struct Quantizer* quantizer);

/**
 * Quantize a high-dimensional vector into discrete lattice points.
 *
 * @param quantizer The Quantizer instance.
 * @param input The input vector (array of doubles).
 * @param input_len The length of the input vector.
 * @param out_quantized Output buffer for quantized values (pre-allocated with input_len int32_t elements).
 * @param out_residuals Output buffer for residual values (pre-allocated with input_len double elements).
 * @return QPZIP_SUCCESS on success, or an error code.
 */
QPZipError qp_zip_quantize(
    struct Quantizer* quantizer,
    const double* input,
    size_t input_len,
    int32_t* out_quantized,
    double* out_residuals
);

/**
 * Reconstruct the original vector from quantized points and residuals.
 *
 * @param quantizer The Quantizer instance.
 * @param quantized The quantized values (array of int32_t).
 * @param residuals The residual values (array of double).
 * @param input_len The length of the vectors.
 * @param out_reconstructed Output buffer for reconstructed values (pre-allocated with input_len double elements).
 * @return QPZIP_SUCCESS on success, or an error code.
 */
QPZipError qp_zip_reconstruct(
    struct Quantizer* quantizer,
    const int32_t* quantized,
    const double* residuals,
    size_t input_len,
    double* out_reconstructed
);

/**
 * Lattice dimension constant for QP-ZIP signatures.
 */
#define QP_ZIP_LATTICE_DIMENSION 256

/**
 * Witness version for QP-ZIP programs (Witness Version 2).
 */
#define QP_ZIP_WITNESS_VERSION 2

/**
 * Maximum size of a QP-ZIP witness program in bytes.
 */
#define QP_ZIP_MAX_WITNESS_SIZE 4096

#ifdef __cplusplus
}
#endif

#endif // BITCOIN_QPZIP_H