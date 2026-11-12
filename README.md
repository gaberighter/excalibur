# Excalibur

A small floating window for macOS that shows system metrics (CPU, memory, storage, temperature, network, GPU) in one place, with drop-in plugins and themes.

The project is at an early stage: the window shows live metrics with sparklines and two themes, but it is not yet borderless or packaged for installation.

## Documents

- [project_scope.md](project_scope.md): the problem and the solution.
- [decisions.md](decisions.md): design decisions and their reasons.
- [stack.md](stack.md): what it is built with.
- [implementation_plan.md](implementation_plan.md): the build order.

## Development

Prerequisites: a current stable Rust toolchain, Xcode Command Line Tools, and the Tauri CLI (`cargo install tauri-cli --version "^2" --locked`).

```sh
cargo tauri dev            # run the app
cargo run -p stats-probe -- cpu memory   # run the helper
cargo test --workspace     # run all tests
```
