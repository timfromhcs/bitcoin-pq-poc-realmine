// Copyright (c) 2023-present The Bitcoin Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or https://opensource.org/license/mit/.
//
// BIP-QP-ZIP: Quantum-Proof Zero-Knowledge Inflight Processing
// Soft-fork witness validation integration.

#ifndef BITCOIN_SCRIPT_QPZIP_H
#define BITCOIN_SCRIPT_QPZIP_H

#include <script/script_error.h>
#include <primitives/transaction.h>
#include <script/interpreter.h>
#include <script/script.h>
#include <span.h>

#include <cstddef>
#include <cstdint>
#include <vector>

/**
 * QP-ZIP witness version constant.
 * Witness version 2 is used for post-quantum signature validation.
 */
static constexpr int QP_ZIP_WITNESS_VERSION = 2;

/**
 * Check if a witness program is a QP-ZIP program.
 * QP-ZIP programs have witness version 2 and a program length between 32 and 4096 bytes.
 *
 * @param witversion The witness program version
 * @param program The witness program data
 * @return true if this is a QP-ZIP program
 */
inline bool IsQPZipProgram(int witversion, const std::vector<unsigned char>& program)
{
    return witversion == QP_ZIP_WITNESS_VERSION &&
           program.size() >= 32 &&
           program.size() <= 4096;
}

/**
 * Verify a QP-ZIP witness program.
 * This function is called from VerifyWitnessProgram when a witness version 2 program is detected.
 *
 * The QP-ZIP witness program contains:
 *   - 32 bytes: Public key commitment (SHA256 of the public key)
 *   - Remaining bytes: Compressed ZK proof, quantized lattice vector, and residuals
 *
 * Legacy (non-upgraded) nodes will treat this as ANYONECANSPEND (always valid),
 * providing soft-fork backwards compatibility.
 *
 * Upgraded nodes will extract and validate the post-quantum signature via the
 * Rust FFI library (rust_qp_zip).
 *
 * @param witness The witness stack from the transaction
 * @param program The witness program (from scriptPubKey)
 * @param checker The transaction signature checker for message extraction
 * @param serror Output parameter for script error details
 * @return true if the witness is valid
 */
bool VerifyQPZipWitnessProgram(const CScriptWitness& witness,
                               const std::vector<unsigned char>& program,
                               const BaseSignatureChecker& checker,
                               ScriptError* serror);

/**
 * Initialize the QP-ZIP library.
 * Should be called once during node startup.
 * Returns true if the library was loaded and initialized successfully.
 */
bool InitQPZipLibrary();

/**
 * Shutdown the QP-ZIP library and free all resources.
 */
void ShutdownQPZipLibrary();

#endif // BITCOIN_SCRIPT_QPZIP_H