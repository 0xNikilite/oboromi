# Contributing to oboromi

First off, thanks for taking the time to contribute!

The following is a set of guidelines for contributing to `oboromi`. These are mostly guidelines, not rules. Use your best judgment, and feel free to propose changes to this document in a pull request.

## Getting Started

### Prerequisites

To build and run `oboromi`, you will need:

*   **Rust**: Latest stable version. [Install Rust](https://www.rust-lang.org/tools/install).
*   **CMake**: Version 3.16 or higher.
*   **Ninja**: Build system.
*   **C++ Compiler**: MSVC (Windows) or Clang (macOS/Linux).

### Building the Project

1.  Clone the repository:
    ```bash
    git clone https://github.com/0xNikilite/oboromi
    cd oboromi
    ```

2.  Build and run:
    ```bash
    cargo run
    ```

### Running Tests (no tests yet)

We prioritize correctness. Please ensure all tests pass before submitting a PR.

```bash
cargo test
```

## Code Style

We follow standard Rust community guidelines.

*   **Formatting**: Run `cargo fmt` to ensure your code is formatted correctly.
*   **Linting**: Run `cargo clippy` to catch common mistakes and improve code quality. (rn it's normal if it fails)

## Submitting a Pull Request

1.  Fork the repository and create your branch from `main`.
2.  If you've added code that should be tested, add tests.
3.  Ensure the test suite passes.
4.  Make sure your code lints.
5.  Issue that pull request!

## Reporting Bugs

Bugs are tracked as GitHub issues. When filing an issue, please explain the problem and include additional details to help maintainers reproduce the problem:

*   Use a clear and descriptive title for the issue to identify the problem.
*   Describe the exact steps which reproduce the problem.
*   Describe the behavior you observed after following the steps.

## License

By contributing, you agree that your contributions will be licensed under its GNU General Public License v3.
