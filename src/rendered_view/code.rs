use super::*;
use gpui_component::scroll::ScrollableElement;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Code { language, code } = block else {
        unreachable!("renderer and block variant diverged")
    };
    let index = cx.index;
    let slice = cx.slice;
    let text_color = cx.text_color;
    let accent = cx.accent;
    let border = cx.border;
    let code_background = cx.code_background;
    let tokens = cx.prepared.code_tokens(language.as_deref(), code);
    let source_offset = if slice.first && slice.last {
        0
    } else {
        slice.range.start
    };
    let theme = cx.theme;
    div()
        .id(("code-block", index))
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .min_w(px(0.0))
        .px_3()
        .bg(rgb(code_background))
        .border_x_1()
        .when(slice.first, |el| el.pt_3().border_t_1().rounded_t_md())
        .when(slice.last, |el| el.pb_3().border_b_1().rounded_b_md())
        .border_color(rgb(border))
        .when(slice.first, |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(rgb(accent))
                    .child(language.clone().unwrap_or_else(|| "text".to_string())),
            )
        })
        .child(
            div()
                .min_w(px(0.0))
                .overflow_x_scrollbar()
                .whitespace_nowrap()
                .font_family("monospace")
                .text_color(rgb(text_color))
                .line_height(px(SOURCE_LINE_HEIGHT - 4.0))
                .children(
                    slice
                        .text(code)
                        .split_inclusive('\n')
                        .scan(source_offset, |offset, line| {
                            let line_start = *offset;
                            *offset += line.len();
                            Some((line_start, line.strip_suffix('\n').unwrap_or(line)))
                        })
                        .map(|(line_start, line)| {
                            let (rendered, runs) =
                                highlighted_line(line, line_start, tokens, theme);
                            div()
                                .min_h(px(SOURCE_LINE_HEIGHT - 4.0))
                                .whitespace_nowrap()
                                .child(gpui::StyledText::new(rendered).with_runs(runs))
                        }),
                ),
        )
        .into_any_element()
}

fn highlighted_line(
    line: &str,
    line_start: usize,
    tokens: &[emerald::preview::CodeToken],
    theme: emerald::theme::EmeraldTheme,
) -> (String, Vec<gpui::TextRun>) {
    use emerald::preview::CodeTokenKind;

    let line_end = line_start + line.len();
    let mut rendered = String::with_capacity(line.len());
    let mut runs = Vec::new();
    let mut column = 0;
    let mut cursor = line_start;
    let mut token_index = tokens.partition_point(|token| token.range.end <= line_start);
    while cursor < line_end {
        let token = tokens
            .get(token_index)
            .filter(|token| token.range.start < line_end);
        let (end, kind) = token.map_or((line_end, CodeTokenKind::Plain), |token| {
            (token.range.end.min(line_end), token.kind)
        });
        let segment_start = cursor.max(token.map_or(cursor, |token| token.range.start));
        if segment_start > cursor {
            append_run(
                &line[cursor - line_start..segment_start - line_start],
                theme.text,
                &mut column,
                &mut rendered,
                &mut runs,
            );
        }
        if end <= segment_start {
            token_index += 1;
            continue;
        }
        append_run(
            &line[segment_start - line_start..end - line_start],
            code_color(kind, theme),
            &mut column,
            &mut rendered,
            &mut runs,
        );
        cursor = end;
        if token.is_some_and(|token| token.range.end <= cursor) {
            token_index += 1;
        }
    }
    (rendered, runs)
}

fn append_run(
    text: &str,
    color: u32,
    column: &mut usize,
    rendered: &mut String,
    runs: &mut Vec<gpui::TextRun>,
) {
    let start = rendered.len();
    for character in text.chars() {
        match character {
            ' ' => {
                rendered.push('\u{00a0}');
                *column += 1;
            }
            '\t' => {
                let spaces = 4 - *column % 4;
                rendered.extend(std::iter::repeat_n('\u{00a0}', spaces));
                *column += spaces;
            }
            character => {
                rendered.push(character);
                *column += 1;
            }
        }
    }
    let len = rendered.len() - start;
    if len > 0 {
        runs.push(gpui::TextRun {
            len,
            font: gpui::font("monospace"),
            color: rgb(color).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }
}

fn code_color(kind: emerald::preview::CodeTokenKind, theme: emerald::theme::EmeraldTheme) -> u32 {
    use emerald::preview::CodeTokenKind::*;
    match kind {
        Plain => theme.text,
        Keyword => theme.code_keyword,
        String => theme.code_string,
        Comment => theme.code_comment,
        Number => theme.code_number,
        Function => theme.code_function,
        Type => theme.code_type,
        Constant => theme.code_constant,
        Property => theme.code_property,
    }
}
