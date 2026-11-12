# Excalibur

A small floating window for macOS that shows system metrics (CPU, memory, storage, temperature, network, GPU) in one place, with drop-in plugins and themes.

It has four built-in themes, including a compact one and a small round one, and an accent colour that follows macOS or is picked from the right-click menu.

## Install

On an Apple Silicon Mac, with [Homebrew](https://brew.sh):

```sh
brew install --cask gaberighter/tap/excalibur
```

The app is not notarised by Apple. The cask clears the quarantine flag so that it opens.

Excalibur has no Dock icon. Right-click its window for themes, the accent colour, the config folder and Quit.

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
scripts/build-release.sh   # build Excalibur.app and a zip of it
```
