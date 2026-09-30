use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::document::DocumentEditor;
use crate::graph::{self, LinkGraph};
use crate::parser::{ParseReport, ParsedDocument};
use crate::workspace::Workspace;

/// Cohesive application boundary for the active document and its workspace.
///
/// This type owns every transition that can invalidate the active document,
/// file list, or link graph. The UI only observes and commands this boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSession {
    workspace: Workspace,
    active_file: PathBuf,
    files: Vec<PathBuf>,
    document: DocumentEditor,
    graph_cache: Option<(u64, LinkGraph)>,
}

impl WorkspaceSession {
    /// Create a lightweight state for the UI while the workspace is loaded in
    /// the background. The full file list and active document are populated by
    /// `open_or_create` once that work completes.
    pub fn loading_placeholder(workspace_root: impl AsRef<Path>) -> Result<Self> {
        let workspace = Workspace::open(workspace_root.as_ref())?;
        let active_file = workspace.root().join("welcome.adoc");
        let document = DocumentEditor::from_text("");

        Ok(Self {
            workspace,
            active_file,
            files: Vec::new(),
            document,
            graph_cache: None,
        })
    }

    #[tracing::instrument(level = "info", skip(workspace_root), fields(root = %workspace_root.as_ref().display()), err)]
    pub fn open_or_create(workspace_root: impl AsRef<Path>) -> Result<Self> {
        let workspace = Workspace::open(workspace_root.as_ref())?;
        let files = workspace.list_asciidoc_files()?;
        let active_file = files
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("workspace has no AsciiDoc files"))?;
        let document = DocumentEditor::from_text(&workspace.read_file(&active_file)?);

        Ok(Self {
            workspace,
            active_file,
            files,
            document,
            graph_cache: None,
        })
    }

    pub fn workspace_root(&self) -> &Path {
        self.workspace.root()
    }
    pub fn active_file(&self) -> &Path {
        &self.active_file
    }
    pub fn text(&self) -> &str {
        self.document.text()
    }
    pub fn line_count(&self) -> usize {
        self.document.line_count()
    }
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }
    pub fn parse_report(&self) -> &ParseReport {
        self.document.parse_report()
    }
    pub fn parsed_document(&self) -> &ParsedDocument {
        self.document.parsed_document()
    }
    pub fn parse_is_dirty(&self) -> bool {
        self.document.parse_is_dirty()
    }
    pub fn parse_revision(&self) -> u64 {
        self.document.parse_revision()
    }
    pub fn text_revision(&self) -> u64 {
        self.document.text_revision()
    }
    pub fn refresh_parse_report_if_dirty(&mut self) -> bool {
        self.document.refresh_parse_report_if_dirty()
    }
    pub fn apply_parse_result(&mut self, text_revision: u64, parsed: ParsedDocument) -> bool {
        self.document.apply_parse_result(text_revision, parsed)
    }
    pub fn is_dirty(&self) -> bool {
        self.document.is_dirty()
    }
    pub fn cursor(&self) -> usize {
        self.document.cursor()
    }
    pub fn selection(&self) -> Option<crate::Selection> {
        self.document.selection()
    }
    pub fn selected_text(&self) -> Option<&str> {
        self.document.selected_text()
    }

    pub fn set_cursor(&mut self, cursor: usize, selecting: bool) {
        self.document.set_cursor(cursor, selecting)
    }
    pub fn select_all(&mut self) {
        self.document.select_all()
    }
    pub fn cursor_line_column(&self) -> (usize, usize) {
        self.document.cursor_line_column()
    }
    pub fn byte_offset_for_line_column(&self, line: usize, column: usize) -> usize {
        self.document.byte_offset_for_line_column(line, column)
    }
    pub fn begin_mouse_selection(&mut self, cursor: usize) {
        self.document.begin_mouse_selection(cursor)
    }
    pub fn extend_mouse_selection(&mut self, cursor: usize) {
        self.document.extend_mouse_selection(cursor)
    }
    pub fn update_mouse_selection(&mut self, cursor: usize) {
        self.document.update_mouse_selection(cursor)
    }
    pub fn end_mouse_selection(&mut self) {
        self.document.end_mouse_selection()
    }

    pub fn insert_text(&mut self, text: &str) {
        self.document.insert_text(text);
        self.invalidate_graph();
    }
    pub fn backspace(&mut self) {
        self.document.backspace();
        self.invalidate_graph();
    }
    pub fn delete(&mut self) {
        self.document.delete();
        self.invalidate_graph();
    }
    pub fn apply_edit_command(&mut self, command: crate::EditCommand) {
        self.document.apply_edit_command(command);
        self.invalidate_graph();
    }

    pub fn save(&mut self) -> Result<()> {
        tracing::info!(file = %self.active_file.display(), "saving document");
        self.workspace
            .write_file(&self.active_file, self.document.text())?;
        self.document.mark_clean();
        self.invalidate_graph();
        Ok(())
    }

    pub fn select_file(&mut self, file: impl AsRef<Path>) -> Result<()> {
        tracing::info!(file = %file.as_ref().display(), "selecting document");
        let active_file = self.workspace.resolve_existing(file)?;
        let text = self.workspace.read_file(&active_file)?;
        self.commit_loaded_file(active_file, text, self.is_dirty())
    }

    pub fn apply_loaded_file(
        &mut self,
        active_file: PathBuf,
        text: String,
        parsed: ParsedDocument,
    ) {
        self.document.replace_loaded_text(&text, parsed);
        self.active_file = active_file;
        self.document.mark_clean();
        self.invalidate_graph();
    }

    pub fn create_file(&mut self, name: &str) -> Result<()> {
        tracing::info!(name, "creating document");
        self.save_if_dirty()?;
        let active_file = self.workspace.create_file(name)?;
        self.active_file = active_file;
        self.document = DocumentEditor::from_text("");
        self.refresh_files()?;
        self.invalidate_graph();
        Ok(())
    }

    pub fn delete_active_file(&mut self) -> Result<()> {
        tracing::info!(file = %self.active_file.display(), "deleting document");
        if self.files.len() <= 1 {
            bail!("cannot delete the last file in the workspace");
        }
        self.save_if_dirty()?;
        self.workspace.remove_file(&self.active_file)?;
        self.refresh_files()?;
        let next_file = self
            .files
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("workspace has no remaining files"))?;
        let text = self.workspace.read_file(&next_file)?;
        self.document = DocumentEditor::from_text(&text);
        self.active_file = next_file;
        self.invalidate_graph();
        Ok(())
    }

    pub fn rename_active_file(&mut self, name: &str) -> Result<()> {
        tracing::info!(file = %self.active_file.display(), name, "renaming document");
        self.save_if_dirty()?;
        let destination = self.workspace.rename_file(&self.active_file, name)?;
        self.active_file = destination;
        self.refresh_files()?;
        self.invalidate_graph();
        Ok(())
    }

    pub fn link_graph(&mut self) -> &LinkGraph {
        let revision = self.text_revision();
        if self
            .graph_cache
            .as_ref()
            .is_none_or(|(cached, _)| *cached != revision)
        {
            let graph = LinkGraph::build(
                self.workspace.root(),
                &self.files,
                Some((&self.active_file, self.text())),
            );
            tracing::debug!(
                nodes = graph.nodes.len(),
                edges = graph.edges.len(),
                errors = graph.errors.len(),
                "rebuilt link graph"
            );
            self.graph_cache = Some((revision, graph));
        }
        &self.graph_cache.as_ref().expect("graph cache populated").1
    }

    pub fn ensure_linked_file(&mut self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = path.as_ref();
        let relative = path
            .strip_prefix(self.workspace.root())
            .or_else(|_| {
                if path.is_relative() {
                    Ok(path)
                } else {
                    Err(path)
                }
            })
            .map_err(|_| {
                anyhow::anyhow!(
                    "Invalid note target '{}'; note has not been made",
                    path.display()
                )
            })?;
        let name = graph::target_file_name(relative).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid note target '{}'; note has not been made",
                path.display()
            )
        })?;
        if relative.components().count() != 1 {
            bail!(
                "Invalid note target '{}'; note has not been made",
                path.display()
            );
        }
        let destination = self.workspace.root().join(name);
        if !destination.exists() {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)?;
        }
        self.refresh_files()?;
        self.invalidate_graph();
        Ok(destination)
    }

    fn commit_loaded_file(
        &mut self,
        active_file: PathBuf,
        text: String,
        dirty: bool,
    ) -> Result<()> {
        if dirty {
            self.save()?;
        }
        let parsed = ParsedDocument::from_source(&text);
        self.apply_loaded_file(active_file, text, parsed);
        Ok(())
    }

    fn save_if_dirty(&mut self) -> Result<()> {
        if self.is_dirty() {
            self.save()?;
        }
        Ok(())
    }

    fn refresh_files(&mut self) -> Result<()> {
        self.files = self.workspace.list_asciidoc_files()?;
        Ok(())
    }

    fn invalidate_graph(&mut self) {
        self.graph_cache = None;
    }
}
