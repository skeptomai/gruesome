# Experimental Tests

This directory contains tests for experimental features that are not yet ready for production use.

## V4/V5 Compiler Tests

V4 and V5 Z-Machine compilation support is experimental and has known issues:

- **String alignment requirements**: V4/V5 require 4-byte aligned string addresses
- **IR mapping bugs**: Complex instruction generation has regressions  
- **Test failures expected**: These tests are known to fail and are not run in CI

### Running Experimental Tests

To run experimental tests locally:

```bash
# Run all experimental tests (many will fail)
cargo test --ignored

# Run specific experimental test
cargo test test_experimental_v4_v5_compilation --ignored
```

### Current Status

- **V3 compilation**: Production ready ✅
- **V4/V5 compilation**: Experimental, disabled in CI ⚠️

V4/V5 support will be re-enabled in CI once the underlying bugs are resolved.