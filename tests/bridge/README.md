# BRIDGE Test Suite

This directory contains tests that validate the BRIDGE documentation framework
as implemented by utf8dok.

## Structure

```
tests/bridge/
├── README.md           # This file
├── fixtures/           # Test input files (coming soon)
├── expected/           # Expected output files (coming soon)
└── integration/        # Integration tests (coming soon)
```

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run only BRIDGE-related tests
cargo test bridge
```

## Test Categories

### Unit Tests
Located in each crate's `src/` directory, testing individual components.

### Integration Tests
Located here, testing the full document processing pipeline.

### Property Tests
Using `proptest` for generative testing of parser robustness.

## Adding Tests

1. Add input fixtures to `fixtures/`
2. Add expected outputs to `expected/`
3. Write test cases in `integration/`

See the main project documentation for contribution guidelines.
