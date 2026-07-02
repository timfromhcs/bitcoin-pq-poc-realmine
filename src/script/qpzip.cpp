// Copyright (c) 2023-present The Bitcoin Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or https://opensource.org/license/mit/.

#include "qpzip.h"
#include <rust_qp_zip/include/qpzip.h>
#include <script/interpreter.h>
#include <hash.h>
#include <util/check.h>

#include <cstring>
#include <mutex>
#include <vector>

// Global QP-ZIP context and extractor
static QPZipContext* g_qpzip_ctx = nullptr;
static Extractor* g_qpzip_extractor = nullptr;
static std::mutex g_qpzip_mutex;

static inline bool set_success(ScriptError* ret)
{
    if (ret)
        *ret = SCRIPT_ERR_OK;
    return true;
}

static inline bool set_error(ScriptError* ret, const ScriptError serror)
{
    if (ret)
        *ret = serror;
    return false;
}

bool InitQPZipLibrary()
{
    std::lock_guard<std::mutex> lock(g_qpzip_mutex);
    if (g_qpzip_ctx != nullptr) {
        return true;
    }

    g_qpzip_ctx = qp_zip_context_new();
    if (g_qpzip_ctx == nullptr) {
        return false;
    }

    // Initialize extractor with a default CRS seed for mainnet/testnet
    static const uint8_t default_crs_seed[32] = {
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20
    };

    g_qpzip_extractor = qp_zip_extractor_new(1024.0, default_crs_seed, 32);
    if (g_qpzip_extractor == nullptr) {
        qp_zip_context_free(g_qpzip_ctx);
        g_qpzip_ctx = nullptr;
        return false;
    }

    return true;
}

void ShutdownQPZipLibrary()
{
    std::lock_guard<std::mutex> lock(g_qpzip_mutex);
    if (g_qpzip_extractor != nullptr) {
        qp_zip_extractor_free(g_qpzip_extractor);
        g_qpzip_extractor = nullptr;
    }
    if (g_qpzip_ctx != nullptr) {
        qp_zip_context_free(g_qpzip_ctx);
        g_qpzip_ctx = nullptr;
    }
}

bool VerifyQPZipWitnessProgram(const CScriptWitness& witness,
                               const std::vector<unsigned char>& program,
                               const BaseSignatureChecker& checker,
                               ScriptError* serror)
{
    (void)checker;

    // 1. Ensure library is initialized
    if (!InitQPZipLibrary()) {
        return set_error(serror, SCRIPT_ERR_UNKNOWN_ERROR);
    }

    // 2. Validate witness program structure
    // A valid QP-ZIP program must have at least 32 bytes (the public key commitment)
    if (program.size() < 32) {
        return set_error(serror, SCRIPT_ERR_WITNESS_PROGRAM_WRONG_LENGTH);
    }

    // 3. Extract the public key commitment (first 32 bytes of the program)
    std::vector<unsigned char> pubkey_commitment(program.begin(), program.begin() + 32);

    // 4. The witness stack must contain exactly 1 item: the serialized compressed signature
    if (witness.stack.size() != 1) {
        return set_error(serror, SCRIPT_ERR_WITNESS_PROGRAM_MISMATCH);
    }

    const std::vector<unsigned char>& compressed_sig = witness.stack[0];
    if (compressed_sig.size() > QP_ZIP_MAX_WITNESS_SIZE) {
        return set_error(serror, SCRIPT_ERR_PUSH_SIZE);
    }

    // 5. Compute message hash from the witness program for validation
    // In a production implementation, this would be the actual transaction sighash.
    std::vector<unsigned char> message(32, 0);
    CSHA256().Write(program.data(), program.size()).Finalize(message.data());

    // 6. Call the Rust FFI to extract and validate the signature
    // All memory is pre-allocated on the C++ side to avoid FFI memory issues.
    std::vector<double> reconstructed_sig(QP_ZIP_LATTICE_DIMENSION, 0.0);
    
    std::lock_guard<std::mutex> lock(g_qpzip_mutex);
    QPZipError err = (QPZipError)qp_zip_extract_and_validate(
        g_qpzip_extractor,
        compressed_sig.data(),
        compressed_sig.size(),
        message.data(),
        message.size(),
        reconstructed_sig.data()
    );

    if (err != QPZIP_SUCCESS) {
        switch (err) {
            case QPZIP_INVALID_INPUT:
                return set_error(serror, SCRIPT_ERR_NUMEQUALVERIFY);
            case QPZIP_PROOF_VERIFICATION_FAILED:
                return set_error(serror, SCRIPT_ERR_EVAL_FALSE);
            case QPZIP_EXTRACTION_FAILED:
                return set_error(serror, SCRIPT_ERR_VERIFY);
            default:
                return set_error(serror, SCRIPT_ERR_UNKNOWN_ERROR);
        }
    }

    // 7. Verify that the reconstructed signature matches the public key commitment.
    // The public key commitment is the SHA256 hash of the reconstructed signature vector bytes.
    std::vector<unsigned char> reconstructed_bytes(reconstructed_sig.size() * sizeof(double));
    std::memcpy(reconstructed_bytes.data(), reconstructed_sig.data(), reconstructed_bytes.size());

    uint256 reconstructed_hash;
    CSHA256().Write(reconstructed_bytes.data(), reconstructed_bytes.size()).Finalize(reconstructed_hash.begin());

    if (std::memcmp(reconstructed_hash.begin(), pubkey_commitment.data(), 32) != 0) {
        return set_error(serror, SCRIPT_ERR_WITNESS_PROGRAM_MISMATCH);
    }

    return set_success(serror);
}