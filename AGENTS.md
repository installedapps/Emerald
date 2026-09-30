# Repository Guidelines

## Project Structure & Module Organization

Emerald is a Rust  desktop AsciiDoc editor built with GPUI.

- `src/main.rs` contains the application entry point and GPUI event/render wiring.
- `src/lib.rs` exposes editor state and editing commands; supporting modules cover
  parsing (`parser.rs`), rendering (`rendered.rs`), text storage (`rope.rs`,
  `sum_tree.rs`), UI layout (`ui.rs`), themes (`theme.rs`), and workspace files
  (`workspace.rs`).
- `notes/` contains architecture, roadmap, and implementation guidance. Update
  relevant notes when a design decision changes.
- `Cargo.toml` and `Cargo.lock` define the package and dependency versions.

There is no separate test or asset tree currently; unit tests live beside the
implementation they cover.

## Build, Test, and Development Commands

Run commands from the repository root:

- `cargo build` — compile the application.
- `cargo run` — launch the GPUI editor locally.
- `cargo test` — run all unit tests, including tests in `src/`.
- `cargo fmt --all -- --check` — verify Rust formatting; use `cargo fmt --all`
  to apply it.
- `cargo clippy --all-targets --all-features -- -D warnings` — run lint checks
  with warnings treated as errors.

The application creates a default `welcome.adoc` in the selected workspace when
needed. Avoid committing generated build output such as `target/`.

## Coding Style & Naming Conventions

Use standard `rustfmt` formatting, four-space indentation, and idiomatic Rust:
`snake_case` for functions/modules, `UpperCamelCase` for types, and
`SCREAMING_SNAKE_CASE` for constants. Keep editor behavior in `lib.rs` and
focused domain logic in its corresponding module. Prefer small, testable pure
functions for parsing, layout calculations, and text operations; use `Result`
(and `anyhow` at application boundaries) for recoverable I/O failures.

## Testing Guidelines

Add focused `#[test]` cases in the module being changed. Name tests by behavior,
for example `creates_default_asciidoc_file_when_workspace_is_empty`. Cover text
editing, parsing, workspace behavior, and layout calculations; run `cargo test`
before submitting changes. No explicit coverage threshold is configured.

## Commit & Pull Request Guidelines

Use short imperative commit subjects, such as `Fix selection deletion` or
`Add AsciiDoc table rendering`. Keep unrelated changes separate. Pull requests
should explain the behavior change, mention tests and validation commands, link
an issue when applicable, and include screenshots or a short recording for
visible GPUI changes.

## Security & Configuration Tips

Do not commit workspace documents containing sensitive data, credentials, or
local configuration. Keep dependency changes deliberate and commit the updated
`Cargo.lock` when dependencies change.
