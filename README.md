# Mallow

[![CI](https://github.com/minhuw/mallow/actions/workflows/ci.yml/badge.svg)](https://github.com/minhuw/mallow/actions/workflows/ci.yml)

## Requirements

- Rust nightly toolchain (for portable SIMD support)
- Cargo (Rust's package manager)
- [pre-commit](https://pre-commit.com/) (for git hooks)

## Getting Started

1. Clone the repository:
   ```bash
   git clone https://github.com/minhuw/mallow.git
   cd mallow
   ```

2. Install pre-commit hooks:
   ```bash
   pip install pre-commit
   pre-commit install
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

4. Run the example:
   ```bash
   cargo run --release
   ```

## Example Usage

The tool provides various options to benchmark memory bandwidth:

```bash
# Basic sequential benchmark with default settings
cargo run --release
```


## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 