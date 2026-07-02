# Copyright (c) 2023-present The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or https://opensource.org/license/mit/.

# This module builds the rust_qp_zip static library from the Rust source.
# It uses cargo to compile the library and then imports it as a CMake target.

function(add_rust_qp_zip subdir)
  message("")
  message("Configuring rust_qp_zip Rust library...")

  # Determine the Rust target directory
  set(RUST_TARGET_DIR "${CMAKE_CURRENT_SOURCE_DIR}/${subdir}/target")
  set(RUST_LIB_NAME "librust_qp_zip.a")

  # Build the Rust library using cargo
  # We use a custom command to build the library
  add_custom_command(
    OUTPUT "${RUST_TARGET_DIR}/release/${RUST_LIB_NAME}"
    COMMAND bash -c "source ~/.cargo/env && cargo build --release --manifest-path ${CMAKE_CURRENT_SOURCE_DIR}/${subdir}/Cargo.toml"
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/${subdir}"
    COMMENT "Building rust_qp_zip static library"
    VERBATIM
  )

  # Create a custom target for the Rust library
  add_custom_target(rust_qp_zip_build ALL
    DEPENDS "${RUST_TARGET_DIR}/release/${RUST_LIB_NAME}"
  )

  # Create an imported static library target
  add_library(rust_qp_zip STATIC IMPORTED GLOBAL)
  set_target_properties(rust_qp_zip PROPERTIES
    IMPORTED_LOCATION "${RUST_TARGET_DIR}/release/${RUST_LIB_NAME}"
  )
  add_dependencies(rust_qp_zip rust_qp_zip_build)

  # Include the Rust library headers
  include_directories("${CMAKE_CURRENT_SOURCE_DIR}/${subdir}/include")
endfunction()