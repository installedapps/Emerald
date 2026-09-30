pub mod cursor;
mod document;
pub mod graph;
pub mod logging;
pub mod parser;
pub mod preview;
pub mod rendered;
pub mod session;
pub mod slash_commands;
mod text_buffer;
pub mod theme;
pub mod ui;
pub mod workspace;

pub use document::{EditCommand, Selection};
pub use session::WorkspaceSession;
pub type EditorState = WorkspaceSession;

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
