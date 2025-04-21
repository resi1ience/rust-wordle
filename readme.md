# Wordle Rust Implementation

A command-line and TUI Wordle game clone implemented in Rust as a course assignment.

## Versions

This project includes two versions:

1.  **Basic Version:**
    *   Pure command-line interface.
    *   File: `src/main.rs`
    *   Binary: `wordle`
2.  **Enhanced Version:**
    *   Adds a Text User Interface (TUI) and a solver/hint mode.
    *   File: `src/main+tui+hint.rs`
    *   Binary: `extra_version` (configured in `Cargo.toml`)

## Features

*   Standard Wordle gameplay rules.
*   **Game Modes:** Specify answer (`--word`), random answer (`--random`), hard mode (`--difficulty hard`).
*   **Options:** Display game statistics (`--stats`), use custom word lists (`--final-set`, `--acceptable-set`), save/load game state (`--state`, `--save`), configuration file (`--config`).
*   **Enhanced Version Only:**
    *   Text User Interface (TUI) built with `tui-rs` and `crossterm`.
    *   Wordle Solver/Hint mode (`--hint` or `-h`) to suggest potential answers.

## How to Run

Replace `[arguments]` with desired command-line options (e.g., `--random`, `--stats`, `--hint`).

*   **Basic Version:**
    ```bash
    cargo run --bin wordle -- [arguments]
    ```

*   **Enhanced Version (TUI + Hints):**
    ```bash
    cargo run --bin extra_version -- [arguments]
    ```

## Testing (Basic Version)

Run automated tests using:

```bash
cargo test [--release] -- --test-threads=1