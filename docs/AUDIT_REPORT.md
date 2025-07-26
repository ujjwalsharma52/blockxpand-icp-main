# Security Audit

## Overview

We performed a manual review and fuzz testing of the DEX adapters. The fuzz tests feed random Candid bytes into each decoder to ensure that malformed input never causes a panic. Reward claiming logic now checks for arithmetic overflow and returns an error if detected. Claim attempts fail cleanly when environment variables are missing, preventing unauthorized calls.

## Findings

- **Re-entrancy**: no cross-call cycles were found. Calls to external canisters return before updating internal state.
- **Overflow**: reward totals use `checked_add` to guard against u64 overflow.
- **Unauthorized claim**: attempts without the required environment variables produce errors.

All issues were resolved and no critical vulnerabilities remain.

## Coverage

Unit and integration tests compile successfully. Fuzz targets run as part of `cargo test`, providing coverage across decoding paths.

