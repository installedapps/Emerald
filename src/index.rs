//! A workspace snapshot shared by graph, backlink, and future search views.
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::graph::{GraphEdge, LinkGraph};
use crate::parser::{extract_references, InlineReference};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedNote {
    pub path: PathBuf,
    pub text: Arc<str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkKind {
    Xref,
    Wiki,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvalidLinkReason {
    UnsupportedFileType,
    UnsafePath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedTarget {
    ExistingNote(PathBuf),
    MissingNote(PathBuf),
    Invalid {
        target: String,
        reason: InvalidLinkReason,
    },
}

impl ResolvedTarget {
    pub fn note_path(&self) -> Option<&Path> {
        match self {
            Self::ExistingNote(path) | Self::MissingNote(path) => Some(path),
            Self::Invalid { .. } => None,
        }
    }

    pub fn exists(&self) -> bool {
        matches!(self, Self::ExistingNote(_))
    }
}

/// One source occurrence, including its UTF-8 byte range in the AsciiDoc text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoteLink {
    pub source: PathBuf,
    pub target: ResolvedTarget,
    pub raw_target: String,
    pub label: String,
    pub kind: LinkKind,
    pub source_range: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIndex {
    notes: BTreeMap<PathBuf, IndexedNote>,
    links: Vec<NoteLink>,
    graph: LinkGraph,
}

impl WorkspaceIndex {
    pub fn build(root: &Path, files: &[PathBuf], active: Option<(&Path, &str)>) -> Self {
        let known_notes = files.iter().cloned().collect::<BTreeSet<_>>();
        let mut notes = BTreeMap::new();
        let mut links = Vec::new();
        let mut nodes = known_notes.clone();
        let mut edges = Vec::new();
        let mut errors = Vec::new();

        for source in files {
            let text = active
                .filter(|(path, _)| *path == source.as_path())
                .map(|(_, text)| text.to_string())
                .or_else(|| std::fs::read_to_string(source).ok());
            let Some(text) = text else {
                errors.push(format!(
                    "Could not read {}; note has not been made",
                    source.display()
                ));
                continue;
            };

            let references = extract_references(&text);
            for reference in references {
                let target = resolve_target(root, &known_notes, &reference);
                if let Some(target_path) = target.note_path() {
                    nodes.insert(target_path.to_path_buf());
                    let edge = GraphEdge {
                        source: source.clone(),
                        target: target_path.to_path_buf(),
                    };
                    if !edges.contains(&edge) {
                        edges.push(edge);
                    }
                } else if let ResolvedTarget::Invalid { target, .. } = &target {
                    errors.push(format!(
                        "Invalid link target '{target}'; note has not been made"
                    ));
                }
                links.push(note_link(source, reference, target));
            }
            notes.insert(
                source.clone(),
                IndexedNote {
                    path: source.clone(),
                    text: Arc::from(text),
                },
            );
        }

        Self {
            notes,
            links,
            graph: LinkGraph {
                nodes: nodes.into_iter().collect(),
                edges,
                errors,
            },
        }
    }

    pub fn notes(&self) -> impl Iterator<Item = &IndexedNote> {
        self.notes.values()
    }

    pub fn note(&self, path: &Path) -> Option<&IndexedNote> {
        self.notes.get(path)
    }

    pub fn links(&self) -> &[NoteLink] {
        &self.links
    }

    pub fn backlinks_to(&self, target: &Path) -> impl Iterator<Item = &NoteLink> {
        let target = target.to_path_buf();
        self.links.iter().filter(move |link| {
            matches!(
                &link.target,
                ResolvedTarget::ExistingNote(path) if path == &target
            )
        })
    }

    pub fn forward_links_from(&self, source: &Path) -> impl Iterator<Item = &NoteLink> {
        let source = source.to_path_buf();
        self.links.iter().filter(move |link| link.source == source)
    }

    pub fn broken_links(&self) -> impl Iterator<Item = &NoteLink> {
        self.links
            .iter()
            .filter(|link| matches!(link.target, ResolvedTarget::MissingNote(_)))
    }

    pub fn graph(&self) -> &LinkGraph {
        &self.graph
    }
}

fn note_link(source: &Path, reference: InlineReference, target: ResolvedTarget) -> NoteLink {
    NoteLink {
        source: source.to_path_buf(),
        target,
        raw_target: reference.target,
        label: reference.label,
        kind: if reference.wiki {
            LinkKind::Wiki
        } else {
            LinkKind::Xref
        },
        source_range: reference.start..reference.end,
    }
}

fn resolve_target(
    root: &Path,
    known_notes: &BTreeSet<PathBuf>,
    reference: &InlineReference,
) -> ResolvedTarget {
    let target = reference.file_target();
    if !target.ends_with(".adoc") {
        return ResolvedTarget::Invalid {
            target,
            reason: InvalidLinkReason::UnsupportedFileType,
        };
    }
    if !valid_target(&target) {
        return ResolvedTarget::Invalid {
            target,
            reason: InvalidLinkReason::UnsafePath,
        };
    }
    let path = root.join(target);
    if known_notes.contains(&path) {
        ResolvedTarget::ExistingNote(path)
    } else {
        ResolvedTarget::MissingNote(path)
    }
}

fn valid_target(target: &str) -> bool {
    let path = Path::new(target);
    !path.is_absolute() && path.components().count() == 1 && !target.contains(['\\', ':'])
}
