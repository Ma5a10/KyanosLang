# Compiler Tests (`compiler_tests`)

This crate provides integration and unit tests for the Kyanos compiler crates (`kyanos_lexer`, `kyanos_parse`, `kyanos_sema`, `kyanos_codegen_llvm`, `kyanos_driver`).

## Structure

- `src/lib.rs`: Common test utilities and helper functions (e.g. `tokenize`).
- `tests/`: Integration test suites.
  - `lexer_test.rs`: Lexer tests.
  - Future test files can be added here (e.g., `parser_test.rs`, `sema_test.rs`).

## Running Tests

Run all tests across the workspace:
```powershell
cargo test --workspace
```

Run only compiler integration tests:
```powershell
cargo test -p compiler_tests
```

Run a specific test:
```powershell
cargo test -p compiler_tests --test lexer_test
```

