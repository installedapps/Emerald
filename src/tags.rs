use super::*;

pub(super) fn extract_tags(source: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix(":tags:") {
            for tag in value
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
            {
                let tag = tag.trim_start_matches('#').to_owned();
                if !tag.is_empty() && !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
    }
    tags
}

pub(super) fn render_note_tags(tags: &[String], theme: EmeraldTheme) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .children(tags.iter().map(|tag| {
            div()
                .px_2()
                .py_1()
                .rounded_full()
                .bg(rgb(theme.accent).opacity(0.14))
                .text_sm()
                .text_color(rgb(theme.accent))
                .child(format!("#{tag}"))
        }))
}
