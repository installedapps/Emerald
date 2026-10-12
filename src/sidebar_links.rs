use super::*;
use gpui_component::scroll::ScrollableElement;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreviewPart {
    text: String,
    highlighted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Mention {
    parts: Vec<PreviewPart>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MentionGroup {
    source: PathBuf,
    mentions: Vec<Mention>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LinkPanelDirection {
    Backlinks,
    ForwardLinks,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LinksSidebarMode {
    Links,
    Tags,
}

impl LinkPanelDirection {
    pub(super) fn toggled(self) -> Self {
        match self {
            Self::Backlinks => Self::ForwardLinks,
            Self::ForwardLinks => Self::Backlinks,
        }
    }
}

impl super::Emerald {
    pub(super) fn render_links_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let active = self.state.active_file().to_path_buf();
        let direction = self.interaction.link_panel_direction;
        let mode = self.interaction.links_sidebar_mode;
        let header = self.links_sidebar_header(theme, direction, mode, cx);
        let index = self.state.workspace_index();
        if mode == LinksSidebarMode::Tags {
            let mut tags = std::collections::BTreeMap::<String, Vec<PathBuf>>::new();
            for note in index.notes() {
                for tag in super::tags::extract_tags(&note.text) {
                    tags.entry(tag).or_default().push(note.path.clone());
                }
            }
            let mut sidebar = div()
                .flex()
                .flex_col()
                .gap_2()
                .w(px(self.interaction.links_sidebar_width))
                .h_full()
                .min_h(px(0.0))
                .overflow_y_scrollbar()
                .p_3()
                .bg(sidebar_background(theme))
                .border_l_1()
                .border_color(rgb(theme.panel_border))
                .child(header);
            sidebar = sidebar.child(section_heading("Tags", tags.len(), theme));
            if tags.is_empty() {
                sidebar = sidebar.child(empty_section("No tags yet", theme));
            }
            let selected_tag = self.interaction.selected_sidebar_tag.clone();
            for (tag, notes) in tags {
                let selected = selected_tag.as_deref() == Some(tag.as_str());
                sidebar = sidebar.child(
                    div()
                        .id(gpui::SharedString::from(format!("tag-row-{tag}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(rgb(if selected { theme.accent } else { theme.text }))
                        .when(selected, |row| row.bg(rgb(theme.selection)))
                        .hover(|row| row.bg(rgb(theme.hover)))
                        .child(format!("#{tag}"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme.faint_text))
                                .child(notes.len().to_string()),
                        )
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.interaction.selected_sidebar_tag =
                                if selected { None } else { Some(tag.clone()) };
                            cx.notify();
                        })),
                );
                if selected {
                    sidebar = sidebar.child(div().flex().flex_col().gap_1().pl_3().children(
                        notes.into_iter().map(|path| {
                            let label = note_label(&path);
                            div()
                                .id(gpui::SharedString::from(format!(
                                    "tag-note-{}",
                                    path.display()
                                )))
                                .w_full()
                                .truncate()
                                .px_2()
                                .py_1()
                                .rounded_sm()
                                .text_xs()
                                .text_color(rgb(if path == active {
                                    theme.accent
                                } else {
                                    theme.muted_text
                                }))
                                .cursor_pointer()
                                .hover(|row| row.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
                                .child(label)
                                .on_click(cx.listener(move |view, _, window, cx| {
                                    view.open_tagged_note(path.clone(), window, cx);
                                }))
                        }),
                    ));
                }
            }
            return self.sidebar_frame(true, sidebar, cx);
        }
        let groups = match direction {
            LinkPanelDirection::Backlinks => linked_mentions(index, &active),
            LinkPanelDirection::ForwardLinks => forward_link_groups(index, &active),
        };
        let count = groups
            .iter()
            .map(|group| group.mentions.len())
            .sum::<usize>();

        let width = self.interaction.links_sidebar_width;
        let mut sidebar = div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(width))
            .h_full()
            .min_h(px(0.0))
            .overflow_y_scrollbar()
            .p_3()
            .bg(sidebar_background(theme))
            .border_l_1()
            .border_color(rgb(theme.panel_border))
            .child(header);

        let section = match direction {
            LinkPanelDirection::Backlinks => "Linked mentions",
            LinkPanelDirection::ForwardLinks => "Outgoing links",
        };
        let empty = match direction {
            LinkPanelDirection::Backlinks => "No linked mentions",
            LinkPanelDirection::ForwardLinks => "No outgoing links",
        };
        sidebar = sidebar.child(section_heading(section, count, theme));
        if groups.is_empty() {
            sidebar = sidebar.child(empty_section(empty, theme));
        } else {
            for group in groups {
                sidebar = sidebar.child(render_mention_group(
                    group,
                    direction,
                    false,
                    theme,
                    &self.interaction.collapsed_backlinks,
                    cx,
                ));
            }
        }

        if direction == LinkPanelDirection::Backlinks {
            let unlinked = unlinked_mentions(index, &active);
            let unlinked_count = unlinked
                .iter()
                .map(|group| group.mentions.len())
                .sum::<usize>();
            sidebar = sidebar.child(section_heading("Unlinked mentions", unlinked_count, theme));
            if unlinked.is_empty() {
                sidebar = sidebar.child(empty_section("No unlinked mentions", theme));
            } else {
                for group in unlinked {
                    sidebar = sidebar.child(render_mention_group(
                        group,
                        direction,
                        true,
                        theme,
                        &self.interaction.collapsed_backlinks,
                        cx,
                    ));
                }
            }
        }

        self.sidebar_frame(true, sidebar, cx)
    }

    fn links_sidebar_header(
        &self,
        theme: emerald::theme::EmeraldTheme,
        direction: LinkPanelDirection,
        mode: LinksSidebarMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(theme.accent))
                    .child(match mode {
                        LinksSidebarMode::Links => match direction {
                            LinkPanelDirection::Backlinks => "BACKLINKS",
                            LinkPanelDirection::ForwardLinks => "FORWARD LINKS",
                        },
                        LinksSidebarMode::Tags => "TAGS",
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(mode == LinksSidebarMode::Links, |controls| {
                        controls.child(
                            div()
                                .id("toggle-link-direction")
                                .role(gpui::Role::Button)
                                .aria_label(match direction {
                                    LinkPanelDirection::Backlinks => "Show forward links",
                                    LinkPanelDirection::ForwardLinks => "Show backlinks",
                                })
                                .tab_stop(true)
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_sm()
                                .text_color(rgb(theme.muted_text))
                                .cursor_pointer()
                                .hover(|button| {
                                    button.bg(rgb(theme.hover)).text_color(rgb(theme.accent))
                                })
                                .child("⇄")
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.interaction.link_panel_direction =
                                        view.interaction.link_panel_direction.toggled();
                                    cx.notify();
                                })),
                        )
                    })
                    .when(mode == LinksSidebarMode::Tags, |controls| {
                        controls.child(
                            div()
                                .id("show-links-panel")
                                .role(gpui::Role::Button)
                                .aria_label("Show links")
                                .tab_stop(true)
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_sm()
                                .text_color(rgb(theme.muted_text))
                                .cursor_pointer()
                                .hover(|button| {
                                    button.bg(rgb(theme.hover)).text_color(rgb(theme.accent))
                                })
                                .child("⇄")
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.interaction.links_sidebar_mode = LinksSidebarMode::Links;
                                    view.interaction.selected_sidebar_tag = None;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        div()
                            .id("toggle-tags-panel")
                            .role(gpui::Role::Button)
                            .aria_label(if mode == LinksSidebarMode::Tags {
                                "Show links"
                            } else {
                                "Show tags"
                            })
                            .tab_stop(true)
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|button| {
                                button.bg(rgb(theme.hover)).text_color(rgb(theme.accent))
                            })
                            .child(super::sidebar::sidebar_icon(
                                super::sidebar::SidebarIcon::Tag,
                                theme,
                                mode == LinksSidebarMode::Tags,
                                16.0,
                            ))
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.interaction.links_sidebar_mode = if view
                                    .interaction
                                    .links_sidebar_mode
                                    == LinksSidebarMode::Tags
                                {
                                    LinksSidebarMode::Links
                                } else {
                                    LinksSidebarMode::Tags
                                };
                                view.interaction.selected_sidebar_tag = None;
                                cx.notify();
                            })),
                    ),
            )
    }
}

fn section_heading(
    label: &'static str,
    count: usize,
    theme: emerald::theme::EmeraldTheme,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .pt_2()
        .text_xs()
        .text_color(rgb(theme.muted_text))
        .child(label)
        .child(count.to_string())
}

fn empty_section(label: &'static str, theme: emerald::theme::EmeraldTheme) -> impl IntoElement {
    div()
        .py_1()
        .text_xs()
        .text_color(rgb(theme.faint_text))
        .child(label)
}

fn render_mention_group(
    group: MentionGroup,
    direction: LinkPanelDirection,
    unlinked: bool,
    theme: emerald::theme::EmeraldTheme,
    collapsed: &std::collections::HashSet<PathBuf>,
    cx: &mut Context<Emerald>,
) -> impl IntoElement {
    let source = group.source;
    let label = note_label(&source);
    let target = note_target(&source).unwrap_or_default();
    let kind = if unlinked {
        "unlinked"
    } else {
        match direction {
            LinkPanelDirection::Backlinks => "backlink",
            LinkPanelDirection::ForwardLinks => "forward-link",
        }
    };
    let is_collapsed = collapsed.contains(&source);
    let source_for_toggle = source.clone();
    let header = div()
        .id(gpui::SharedString::from(format!("{kind}-group-{target}",)))
        .role(gpui::Role::Button)
        .aria_label(format!("{} mentions in {label}", group.mentions.len()))
        .tab_stop(true)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .px_1()
        .py_1()
        .rounded_sm()
        .text_sm()
        .text_color(rgb(theme.text))
        .cursor_pointer()
        .hover(|row| row.bg(rgb(theme.hover)))
        .child(if is_collapsed { "›" } else { "⌄" })
        .child(label)
        .child(
            div()
                .ml_auto()
                .text_xs()
                .text_color(rgb(theme.muted_text))
                .child(group.mentions.len().to_string()),
        )
        .on_click(cx.listener(move |view, _, _, cx| {
            if !view
                .interaction
                .collapsed_backlinks
                .remove(&source_for_toggle)
            {
                view.interaction
                    .collapsed_backlinks
                    .insert(source_for_toggle.clone());
            }
            cx.notify();
        }));

    let mut row = div().flex().flex_col().gap_1().child(header);
    if !is_collapsed {
        for (index, mention) in group.mentions.into_iter().enumerate() {
            let target = target.clone();
            let label = note_label(&source);
            let mut preview = div()
                .id(gpui::SharedString::from(format!(
                    "{kind}-preview-{target}-{index}",
                )))
                .role(gpui::Role::Button)
                .aria_label(format!("Open mention in {label}"))
                .tab_stop(true)
                .flex()
                .flex_wrap()
                .w_full()
                .min_w(px(0.0))
                .max_h(px(112.0))
                .overflow_hidden()
                .px_2()
                .py_2()
                .rounded_md()
                .bg(rgb(theme.panel))
                .text_xs()
                .text_color(rgb(theme.muted_text))
                .cursor_pointer()
                .hover(|card| card.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
                .children(mention.parts.into_iter().map(|part| {
                    let piece = div()
                        .min_w(px(0.0))
                        .whitespace_normal()
                        .child(gpui::SharedString::new(part.text));
                    if part.highlighted {
                        piece
                            .px_1()
                            .rounded_sm()
                            .bg(rgb(theme.active_file))
                            .text_color(rgb(theme.active_file_text))
                            .into_any_element()
                    } else {
                        piece.into_any_element()
                    }
                }));
            let source_name = target.clone();
            preview = preview.on_click(cx.listener(move |view, _, window, cx| {
                view.open_link(source_name.clone(), window, cx);
            }));
            row = row.child(preview);
        }
    }
    row
}

fn linked_mentions(
    index: &emerald::index::WorkspaceIndex,
    active: &std::path::Path,
) -> Vec<MentionGroup> {
    let mut groups = std::collections::BTreeMap::<PathBuf, Vec<Mention>>::new();
    for backlink in index.backlinks_to(active) {
        let Some(note) = index.note(&backlink.source) else {
            continue;
        };
        let source_links = index
            .links()
            .iter()
            .filter(|link| link.source == backlink.source)
            .collect::<Vec<_>>();
        groups
            .entry(backlink.source.clone())
            .or_default()
            .push(reference_preview(&note.text, &source_links, backlink));
    }
    groups
        .into_iter()
        .map(|(source, mentions)| MentionGroup { source, mentions })
        .collect()
}

fn forward_link_groups(
    index: &emerald::index::WorkspaceIndex,
    active: &std::path::Path,
) -> Vec<MentionGroup> {
    let Some(note) = index.note(active) else {
        return Vec::new();
    };
    let source_links = index.forward_links_from(active).collect::<Vec<_>>();
    let mut groups = std::collections::BTreeMap::<PathBuf, Vec<Mention>>::new();
    for link in &source_links {
        let Some(target) = link.target.note_path() else {
            continue;
        };
        groups
            .entry(target.to_path_buf())
            .or_default()
            .push(reference_preview(&note.text, &source_links, link));
    }
    groups
        .into_iter()
        .map(|(source, mentions)| MentionGroup { source, mentions })
        .collect()
}

fn unlinked_mentions(
    index: &emerald::index::WorkspaceIndex,
    active: &std::path::Path,
) -> Vec<MentionGroup> {
    let Some(stem) = active.file_stem().and_then(|stem| stem.to_str()) else {
        return Vec::new();
    };
    let label = stem.replace(['_', '-'], " ");
    let mut variants = [stem, label.as_str()]
        .into_iter()
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    variants.sort_unstable();
    variants.dedup();
    let mut groups = Vec::new();
    for note in index.notes().filter(|note| note.path != active) {
        let source = &note.path;
        let text = note.text.as_ref();
        let references = index
            .links()
            .iter()
            .filter(|link| link.source == *source)
            .collect::<Vec<_>>();
        let mut ranges = Vec::new();
        for variant in &variants {
            for (start, end) in find_text_occurrences(text, variant) {
                if references.iter().any(|reference| {
                    start < reference.source_range.end && end > reference.source_range.start
                }) {
                    continue;
                }
                ranges.push((start, end));
            }
        }
        ranges.sort_unstable();
        ranges.dedup();
        let mentions = ranges
            .into_iter()
            .map(|(start, end)| plain_reference_preview(text, start, end))
            .collect::<Vec<_>>();
        if !mentions.is_empty() {
            groups.push(MentionGroup {
                source: source.to_path_buf(),
                mentions,
            });
        }
    }
    groups
}

fn reference_preview(
    source: &str,
    references: &[&emerald::index::NoteLink],
    selected: &emerald::index::NoteLink,
) -> Mention {
    let (start, end) = paragraph_bounds(
        source,
        selected.source_range.start,
        selected.source_range.end,
    );
    let mut parts = Vec::new();
    let mut offset = start;
    for reference in references.iter().filter(|reference| {
        reference.source_range.start >= start && reference.source_range.end <= end
    }) {
        push_part(
            &mut parts,
            source[offset..reference.source_range.start].to_string(),
            false,
        );
        push_part(
            &mut parts,
            reference.label.clone(),
            reference.source_range == selected.source_range,
        );
        offset = reference.source_range.end;
    }
    push_part(&mut parts, source[offset..end].to_string(), false);
    Mention { parts }
}

fn plain_reference_preview(source: &str, focus_start: usize, focus_end: usize) -> Mention {
    let (start, end) = paragraph_bounds(source, focus_start, focus_end);
    Mention {
        parts: vec![
            PreviewPart {
                text: source[start..focus_start].to_string(),
                highlighted: false,
            },
            PreviewPart {
                text: source[focus_start..focus_end].to_string(),
                highlighted: true,
            },
            PreviewPart {
                text: source[focus_end..end].to_string(),
                highlighted: false,
            },
        ],
    }
}

fn paragraph_bounds(source: &str, focus_start: usize, focus_end: usize) -> (usize, usize) {
    let start = source[..focus_start]
        .rfind("\n\n")
        .map_or(0, |index| index + 2);
    let end = source[focus_end..]
        .find("\n\n")
        .map_or(source.len(), |index| focus_end + index);
    let paragraph = &source[start..end];
    let left = paragraph.len() - paragraph.trim_start().len();
    let right = paragraph.trim_end().len();
    (start + left, start + right)
}

fn find_text_occurrences(source: &str, needle: &str) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    for (start, _) in source.char_indices() {
        let Some(candidate) = source.get(start..start + needle.len()) else {
            continue;
        };
        if candidate.eq_ignore_ascii_case(needle) {
            matches.push((start, start + needle.len()));
        }
    }
    matches
}

fn push_part(parts: &mut Vec<PreviewPart>, text: String, highlighted: bool) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = parts
        .last_mut()
        .filter(|part| part.highlighted == highlighted)
    {
        last.text.push_str(&text);
    } else {
        parts.push(PreviewPart { text, highlighted });
    }
}

#[cfg(test)]
pub(super) fn outgoing_targets(
    graph: &emerald::graph::LinkGraph,
    active: &std::path::Path,
) -> Vec<std::path::PathBuf> {
    let mut targets = Vec::new();
    for edge in &graph.edges {
        if edge.source == active && !targets.contains(&edge.target) {
            targets.push(edge.target.clone());
        }
    }
    targets
}

pub(super) fn note_target(path: &std::path::Path) -> Option<String> {
    path.file_name()?
        .to_str()
        .filter(|name| name.ends_with(".adoc"))
        .map(str::to_owned)
}

pub(super) fn note_label(path: &std::path::Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('_', " ")
}
