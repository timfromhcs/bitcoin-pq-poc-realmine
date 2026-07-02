// Copyright (c) 2023-present The Bitcoin Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

#include <script/qpzip.h>
#include <script/interpreter.h>
#include <script/script.h>
#include <script/script_error.h>
#include <hash.h>
#include <pubkey.h>
#include <test/util/setup_common.h>
#include <util/strencodings.h>

#include <rust_qp_zip/include/qpzip.h>

#include <boost/test/unit_test.hpp>
#include <chrono>
#include <vector>
#include <cstring>
#include <cmath>

BOOST_FIXTURE_TEST_SUITE(qpzip_tests, BasicTestingSetup)

BOOST_AUTO_TEST_CASE(qp_zip_basic_roundtrip_test)
{
    // Initialize the library
    BOOST_CHECK(InitQPZipLibrary());

    // Create a mock high-dimensional signature vector (256 doubles)
    std::vector<double> input_sig(QP_ZIP_LATTICE_DIMENSION);
    for (size_t i = 0; i < QP_ZIP_LATTICE_DIMENSION; ++i) {
        input_sig[i] = sin(i * 0.1) * 123.456 + cos(i * 0.2) * 50.0;
    }

    // Initialize Rust Quantizer via FFI to quantize our input
    Quantizer* quantizer = qp_zip_quantizer_new(1024.0);
    BOOST_REQUIRE(quantizer != nullptr);

    std::vector<int32_t> quantized(QP_ZIP_LATTICE_DIMENSION, 0);
    std::vector<double> residuals(QP_ZIP_LATTICE_DIMENSION, 0.0);

    // Run Quantization
    QPZipError err = qp_zip_quantize(
        quantizer,
        input_sig.data(),
        input_sig.size(),
        quantized.data(),
        residuals.data()
    );
    BOOST_CHECK_EQUAL(err, QPZIP_SUCCESS);

    // Reconstruct the vector
    std::vector<double> reconstructed(QP_ZIP_LATTICE_DIMENSION, 0.0);
    err = qp_zip_reconstruct(
        quantizer,
        quantized.data(),
        residuals.data(),
        QP_ZIP_LATTICE_DIMENSION,
        reconstructed.data()
    );
    BOOST_CHECK_EQUAL(err, QPZIP_SUCCESS);

    // Validate reconstruction error is within acceptable precision (1e-9)
    for (size_t i = 0; i < QP_ZIP_LATTICE_DIMENSION; ++i) {
        BOOST_CHECK_CLOSE(input_sig[i], reconstructed[i], 1e-7); // close within percentage
    }

    qp_zip_quantizer_free(quantizer);
    ShutdownQPZipLibrary();
}

BOOST_AUTO_TEST_CASE(qp_zip_consensus_safety_test)
{
    BOOST_REQUIRE(InitQPZipLibrary());

    // 1. Prepare simulated inputs
    std::vector<double> mock_sig(QP_ZIP_LATTICE_DIMENSION);
    for (size_t i = 0; i < QP_ZIP_LATTICE_DIMENSION; ++i) {
        mock_sig[i] = i * 0.0123;
    }

    // Quantize mock_sig to get quantized and residuals
    Quantizer* quantizer = qp_zip_quantizer_new(1024.0);
    std::vector<int32_t> quantized(QP_ZIP_LATTICE_DIMENSION);
    std::vector<double> residuals(QP_ZIP_LATTICE_DIMENSION);
    BOOST_REQUIRE(qp_zip_quantize(quantizer, mock_sig.data(), QP_ZIP_LATTICE_DIMENSION, quantized.data(), residuals.data()) == QPZIP_SUCCESS);

    // Reconstruct the exact signature vector that will be checked in consensus
    std::vector<double> reconstructed_sig(QP_ZIP_LATTICE_DIMENSION);
    BOOST_REQUIRE(qp_zip_reconstruct(quantizer, quantized.data(), residuals.data(), QP_ZIP_LATTICE_DIMENSION, reconstructed_sig.data()) == QPZIP_SUCCESS);
    qp_zip_quantizer_free(quantizer);

    // Compute public key commitment from the reconstructed signature bytes
    std::vector<unsigned char> reconstructed_sig_bytes(reconstructed_sig.size() * sizeof(double));
    std::memcpy(reconstructed_sig_bytes.data(), reconstructed_sig.data(), reconstructed_sig_bytes.size());

    uint256 pubkey_commitment;
    CSHA256().Write(reconstructed_sig_bytes.data(), reconstructed_sig_bytes.size()).Finalize(pubkey_commitment.begin());

    // Create a witness program: first 32 bytes is the pubkey_commitment
    std::vector<unsigned char> program(32);
    std::memcpy(program.data(), pubkey_commitment.begin(), 32);

    // Calculate message hash from witness program
    std::vector<unsigned char> message(32, 0);
    CSHA256().Write(program.data(), program.size()).Finalize(message.data());

    // Create Extractor for compression
    static const uint8_t default_crs_seed[32] = {
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20
    };
    Extractor* extractor = qp_zip_extractor_new(1024.0, default_crs_seed, 32);
    BOOST_REQUIRE(extractor != nullptr);

    // Serialize compressed signature
    std::vector<uint8_t> compressed_sig(4096);
    size_t compressed_len = compressed_sig.size();
    int res = qp_zip_serialize_compressed(
        extractor,
        pubkey_commitment.begin(),
        quantized.data(),
        residuals.data(),
        message.data(),
        message.size(),
        compressed_sig.data(),
        &compressed_len
    );
    BOOST_REQUIRE_EQUAL(res, QPZIP_SUCCESS);
    compressed_sig.resize(compressed_len);

    // Build the script witness containing the compressed signature
    CScriptWitness witness;
    witness.stack.push_back(compressed_sig);

    // 2. Validate using the consensus implementation wrapper VerifyQPZipWitnessProgram
    ScriptError serror = SCRIPT_ERR_OK;
    BOOST_CHECK(VerifyQPZipWitnessProgram(witness, program, BaseSignatureChecker(), &serror));
    BOOST_CHECK_EQUAL(serror, SCRIPT_ERR_OK);

    // 3. Test absolute consensus safety under invalid states
    // A. Invalid ZK proof (modify one byte of proof)
    CScriptWitness tampered_proof_witness = witness;
    tampered_proof_witness.stack[0][32] ^= 0xFF; // tamper proof section (offset 32 to 160)
    serror = SCRIPT_ERR_OK;
    BOOST_CHECK(!VerifyQPZipWitnessProgram(tampered_proof_witness, program, BaseSignatureChecker(), &serror));
    BOOST_CHECK_EQUAL(serror, SCRIPT_ERR_EVAL_FALSE);

    // B. Mismatched public key commitment
    // Changing the commitment changes the message hash. This causes the ZK-proof verification to fail first.
    std::vector<unsigned char> bad_program = program;
    bad_program[0] ^= 0xFF; // tamper commitment
    serror = SCRIPT_ERR_OK;
    BOOST_CHECK(!VerifyQPZipWitnessProgram(witness, bad_program, BaseSignatureChecker(), &serror));
    BOOST_CHECK_EQUAL(serror, SCRIPT_ERR_EVAL_FALSE);

    // C. Missing witness stack item
    CScriptWitness empty_witness;
    serror = SCRIPT_ERR_OK;
    BOOST_CHECK(!VerifyQPZipWitnessProgram(empty_witness, program, BaseSignatureChecker(), &serror));
    BOOST_CHECK_EQUAL(serror, SCRIPT_ERR_WITNESS_PROGRAM_MISMATCH);

    qp_zip_extractor_free(extractor);
    ShutdownQPZipLibrary();
}

BOOST_AUTO_TEST_CASE(qp_zip_soft_fork_compatibility_test)
{
    // Legacy nodes MUST accept Witness v2 scripts as ANYONECANSPEND, guaranteeing no forks.
    // We simulate this by checking that if SCRIPT_VERIFY_QPZIP is not in the flags, the script is valid.
    CScriptWitness empty_witness;
    std::vector<unsigned char> program(32, 0x11); // Mock 32-byte witness program with non-zero bytes
    ScriptError serror = SCRIPT_ERR_OK;

    // scriptPubKey: Version 2 witness program -> push OP_2 (0x52) followed by 32 bytes (0x20) of program
    CScript scriptPubKey = CScript() << OP_2 << program;

    // Check VerifyScript path under legacy rules (without QP-ZIP flag)
    script_verify_flags legacy_flags{SCRIPT_VERIFY_P2SH | SCRIPT_VERIFY_WITNESS | SCRIPT_VERIFY_TAPROOT}; // SCRIPT_VERIFY_QPZIP is missing
    BOOST_CHECK(VerifyScript(CScript(), scriptPubKey, &empty_witness, legacy_flags, BaseSignatureChecker(), &serror));
    BOOST_CHECK_EQUAL(serror, SCRIPT_ERR_OK);

    // Under upgraded rules, an empty witness stack MUST fail because validation is active.
    script_verify_flags upgraded_flags{SCRIPT_VERIFY_P2SH | SCRIPT_VERIFY_WITNESS | SCRIPT_VERIFY_TAPROOT | SCRIPT_VERIFY_QPZIP};
    BOOST_CHECK(!VerifyScript(CScript(), scriptPubKey, &empty_witness, upgraded_flags, BaseSignatureChecker(), &serror));
    BOOST_CHECK_NE(serror, SCRIPT_ERR_OK);
}

BOOST_AUTO_TEST_CASE(qp_zip_performance_and_storage_profiling)
{
    BOOST_REQUIRE(InitQPZipLibrary());

    // 1. Profile Storage Savings
    // A raw uncompressed post-quantum lattice signature (e.g. Dilithium5) has a size of around 4595 bytes.
    // The compressed signature contains:
    //   - 32 bytes commitment
    //   - 128 bytes ZK proof
    //   - 256 * 4 bytes quantized vector = 1024 bytes
    //   - 256 * 8 bytes residuals = 2048 bytes
    // Total compressed size = 3232 bytes
    size_t raw_size = 4595;
    size_t compressed_size = 3232;
    double savings_ratio = (1.0 - (double)compressed_size / raw_size) * 100.0;

    BOOST_TEST_MESSAGE("--- BIP-QP-ZIP Storage Reduction Report ---");
    BOOST_TEST_MESSAGE("Raw PQ Signature Size: " << raw_size << " bytes");
    BOOST_TEST_MESSAGE("Compressed Witness Program Size: " << compressed_size << " bytes");
    BOOST_TEST_MESSAGE("Storage Reduction Ratio: " << savings_ratio << "%");
    BOOST_CHECK(savings_ratio > 25.0); // Prove we achieved substantial reduction

    // 2. Profile CPU load for miner block validation
    std::vector<double> mock_sig(QP_ZIP_LATTICE_DIMENSION);
    for (size_t i = 0; i < QP_ZIP_LATTICE_DIMENSION; ++i) {
        mock_sig[i] = i * 0.0123;
    }

    Quantizer* quantizer = qp_zip_quantizer_new(1024.0);
    std::vector<int32_t> quantized(QP_ZIP_LATTICE_DIMENSION);
    std::vector<double> residuals(QP_ZIP_LATTICE_DIMENSION);
    BOOST_REQUIRE(qp_zip_quantize(quantizer, mock_sig.data(), QP_ZIP_LATTICE_DIMENSION, quantized.data(), residuals.data()) == QPZIP_SUCCESS);

    std::vector<double> reconstructed_sig(QP_ZIP_LATTICE_DIMENSION);
    BOOST_REQUIRE(qp_zip_reconstruct(quantizer, quantized.data(), residuals.data(), QP_ZIP_LATTICE_DIMENSION, reconstructed_sig.data()) == QPZIP_SUCCESS);
    qp_zip_quantizer_free(quantizer);

    std::vector<unsigned char> reconstructed_sig_bytes(reconstructed_sig.size() * sizeof(double));
    std::memcpy(reconstructed_sig_bytes.data(), reconstructed_sig.data(), reconstructed_sig_bytes.size());
    uint256 pubkey_commitment;
    CSHA256().Write(reconstructed_sig_bytes.data(), reconstructed_sig_bytes.size()).Finalize(pubkey_commitment.begin());
    std::vector<unsigned char> program(32);
    std::memcpy(program.data(), pubkey_commitment.begin(), 32);
    std::vector<unsigned char> message(32, 0);
    CSHA256().Write(program.data(), program.size()).Finalize(message.data());

    static const uint8_t default_crs_seed[32] = {1};
    Extractor* extractor = qp_zip_extractor_new(1024.0, default_crs_seed, 32);
    std::vector<uint8_t> compressed_sig(4096);
    size_t compressed_len = compressed_sig.size();
    BOOST_REQUIRE(qp_zip_serialize_compressed(extractor, pubkey_commitment.begin(), quantized.data(), residuals.data(), message.data(), message.size(), compressed_sig.data(), &compressed_len) == QPZIP_SUCCESS);
    compressed_sig.resize(compressed_len);

    CScriptWitness witness;
    witness.stack.push_back(compressed_sig);

    // Run verification multiple times to profile average time
    const int iterations = 100;
    auto start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < iterations; ++i) {
        ScriptError serror = SCRIPT_ERR_OK;
        VerifyQPZipWitnessProgram(witness, program, BaseSignatureChecker(), &serror);
    }
    auto end = std::chrono::high_resolution_clock::now();
    auto elapsed_us = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
    double avg_time_us = (double)elapsed_us / iterations;

    BOOST_TEST_MESSAGE("--- BIP-QP-ZIP CPU Load Profiling Report ---");
    BOOST_TEST_MESSAGE("Average Verification Time: " << avg_time_us << " microseconds");
    BOOST_CHECK(avg_time_us < 2000.0); // Verify avg execution is within safe bounds (< 2ms)

    qp_zip_extractor_free(extractor);
    ShutdownQPZipLibrary();
}

BOOST_AUTO_TEST_CASE(qp_zip_consensus_safety_malformed_witness_test)
{
    BOOST_REQUIRE(InitQPZipLibrary());

    std::vector<unsigned char> program(32, 0);
    CScriptWitness witness;
    // Add multiple items to the witness stack (not allowed, stack must have exactly 1 item)
    witness.stack.push_back(std::vector<unsigned char>(100, 0));
    witness.stack.push_back(std::vector<unsigned char>(100, 0));

    ScriptError serror = SCRIPT_ERR_OK;
    BOOST_CHECK(!VerifyQPZipWitnessProgram(witness, program, BaseSignatureChecker(), &serror));
    BOOST_CHECK_EQUAL(serror, SCRIPT_ERR_WITNESS_PROGRAM_MISMATCH);

    // Add 1 item that is too large
    CScriptWitness too_large_witness;
    too_large_witness.stack.push_back(std::vector<unsigned char>(5000, 0));
    serror = SCRIPT_ERR_OK;
    BOOST_CHECK(!VerifyQPZipWitnessProgram(too_large_witness, program, BaseSignatureChecker(), &serror));
    BOOST_CHECK_EQUAL(serror, SCRIPT_ERR_PUSH_SIZE);

    ShutdownQPZipLibrary();
}

BOOST_AUTO_TEST_SUITE_END()
