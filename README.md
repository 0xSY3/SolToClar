# Sol2Clarity: Solidity to Clarity Smart Contract Transpiler

A Rust-based command-line tool that transpiles Solidity smart contracts to Clarity language. This tool helps developers migrate their Ethereum smart contracts to the Stacks blockchain by automatically converting Solidity code to equivalent Clarity code.

## Features

- Parses Solidity smart contracts using a custom PEG parser
- Supports multiple contracts in a single file
- Handles complex Solidity features:
  - State variables and mappings
  - Nested mappings with tuple keys
  - Public/private functions
  - msg.sender conversion to tx-sender
  - Basic arithmetic operations
  - Events (converted to prints)
  - Multiple contracts in a single file

## Prerequisites

- Rust toolchain (2021 edition or later)
- Cargo package manager

## Quick Start

1. Clone the repository:
```bash
git clone https://github.com/0xSY3R/SolToClar.git
cd SoltoClar
```

2. Build the project:
```bash
cargo build --release
```

3. Run the transpiler:
```bash
sol2clarity <input_file.sol> [-o output_directory]
```

The transpiler will create separate Clarity files (`.clar`) for each contract in the input Solidity file.

## Detailed Usage Guide

### Command Line Options

```bash
USAGE:
    sol2clarity [OPTIONS] <INPUT>

ARGS:
    <INPUT>    Input Solidity file

OPTIONS:
    -o, --output <DIR>    Output directory for Clarity files (default: current directory)
    -h, --help           Prints help information
    -V, --version        Prints version information
```

### Input/Output Example

Input (Solidity):
```solidity
// token.sol
contract TokenManager {
    mapping(address => uint256) public balances;
    mapping(address => mapping(uint256 => bool)) public approvals;

    function transfer(address to, uint256 amount) public {
        balances[msg.sender] = balances[msg.sender] - amount;
        balances[to] = balances[to] + amount;
    }
}
```

Output (Clarity):
```clarity
;; Contract: TokenManager
;; Auto-generated Clarity contract from Solidity source

;; @desc Map storing balances values
(define-map balances principal uint)

;; @desc Getter for map balances
(define-read-only (get-balances (key principal))
  (ok (map-get? balances key)))

;; Function: transfer
(define-public (transfer (to principal) (amount uint))
  (begin
    (map-set balances tx-sender (- (map-get? balances tx-sender) amount))
    (ok (map-set balances to (+ (map-get? balances to) amount)))))
```

## Project Architecture

### Directory Structure
```
src/
├── main.rs           # Entry point and CLI handling
├── parser/          
│   ├── mod.rs       # Parser implementation
│   └── solidity.pest # PEG grammar for Solidity
├── transpiler/
│   ├── mod.rs       # Main transpiler module
│   ├── ast.rs       # AST definitions
│   └── converter.rs # Solidity to Clarity conversion
├── generator/
│   └── mod.rs       # Clarity code generation
└── tests/
    └── mod.rs       # Integration tests
```

### Component Overview

#### 1. Parser (`parser/`)
- Uses Pest parser generator with custom PEG grammar
- Handles Solidity syntax including:
  - Contract declarations
  - State variables
  - Functions and modifiers
  - Mappings (including nested)
  - Member access (msg.sender)
  - Events

#### 2. AST (`transpiler/ast.rs`)
The Abstract Syntax Tree represents Solidity constructs:
- Contracts and functions
- State variables
- Mappings
- Expressions and statements
- Events

#### 3. Converter (`transpiler/converter.rs`)
Handles conversion of Solidity concepts to Clarity:
- Type conversions:
  - `address` → `principal`
  - `uint256` → `uint`
  - `mapping` → `define-map`
- Special handling:
  - Nested mappings → Tuple keys
  - msg.sender → tx-sender
  - Public variables → Getter functions

#### 4. Generator (`generator/mod.rs`)
Produces final Clarity code with:
- Proper formatting and indentation
- Documentation comments
- Map definitions and getters
- Function implementations
- Error handling

### Data Flow

1. **Input Processing**
   - Read Solidity file
   - Parse using PEG grammar
   - Generate Solidity AST

2. **Conversion Process**
   - Transform Solidity AST to Clarity AST
   - Apply type conversions
   - Handle special cases (msg.sender, nested mappings)

3. **Code Generation**
   - Generate Clarity code from AST
   - Create separate files for each contract
   - Add documentation and type information

## Testing

### Running Tests

1. Run all tests:
```bash
cargo test
```

2. Run specific test categories:
```bash
cargo test test_parse     # Parser tests
cargo test test_convert  # Converter tests
cargo test test_generate # Generator tests
```

### Test Coverage

- Basic contract parsing
- State variable declarations
- Function definitions with visibility
- Complex mappings and nested structures
- Event declarations
- Binary expressions
- Member access (msg.sender)
- Multiple contracts in single file

## Advanced Features

### 1. Nested Mappings
Solidity nested mappings are converted to Clarity maps with tuple keys:

```solidity
mapping(address => mapping(uint256 => bool)) public approvals;
```

Becomes:

```clarity
(define-map approvals {owner: principal, token-id: uint} bool)
```

### 2. Public State Variables
Public state variables automatically generate getter functions:

```solidity
uint256 public totalSupply;
```

Becomes:

```clarity
(define-data-var total-supply uint u0)
(define-read-only (get-total-supply) (ok (var-get total-supply)))
```

### 3. Multiple Contracts
Processing multiple contracts in a single file:

```solidity
// contracts.sol
contract TokenA { ... }
contract TokenB { ... }
```

Generates:
- `tokena.clar`
- `tokenb.clar`

## Limitations and Future Work

Current limitations:
- Limited support for complex Solidity features
- No support for inheritance
- Basic type system mapping
- Limited standard library support

Planned improvements:
- Support for more Solidity features
- Better error handling and recovery
- Contract inheritance
- Standard library mappings
- Gas optimization
- Source maps for debugging

## Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

Please include tests for new features and ensure all tests pass before submitting PRs.

## Troubleshooting

### Common Issues

1. **Parsing Errors**
   - Check Solidity syntax
   - Verify file encoding (UTF-8)
   - Look for unsupported features

2. **Type Conversion Errors**
   - Check for unsupported Solidity types
   - Verify mapping key/value types
   - Check for complex nested structures

3. **Output Issues**
   - Verify output directory permissions
   - Check for file naming conflicts
   - Ensure valid Clarity syntax

### Debug Logging

Enable debug logging by setting the environment variable:
```bash
RUST_LOG=debug sol2clarity input.sol
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- [Pest Parser](https://pest.rs/) - Used for parsing Solidity
- [Clarity Documentation](https://docs.stacks.co/clarity/overview) - Reference for Clarity language