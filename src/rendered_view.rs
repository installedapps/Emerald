use super::*;
use emerald::preview::PreparedPreview;
use std::path::Path;

#[path = "rendered_view/admonition.rs"]
mod admonition;
#[path = "rendered_view/code.rs"]
mod code;
#[path = "rendered_view/diagram.rs"]
mod diagram;
#[path = "rendered_view/heading.rs"]
mod heading;
#[path = "rendered_view/image.rs"]
mod image;
#[path = "rendered_view/ordered_list.rs"]
mod ordered_list;
#[path = "rendered_view/paragraph.rs"]
mod paragraph;
#[path = "rendered_view/quote.rs"]
mod quote;
#[path = "rendered_view/table.rs"]
mod table;
#[path = "rendered_view/table_of_contents.rs"]
mod table_of_contents;
#[path = "rendered_view/thematic_break.rs"]
mod thematic_break;
#[path = "rendered_view/unordered_list.rs"]
mod unordered_list;
#[cfg(test)]
use self::admonition::admonition_colors;

struct RenderContext<'a> {
    index: usize,
    style: RenderStyle,
    theme: emerald::theme::EmeraldTheme,
    prepared: &'a PreparedPreview,
    document_path: &'a Path,
    entity: gpui::Entity<Emerald>,
    slice: &'a emerald::preview::PreviewRow,
    text_color: u32,
    muted_text: u32,
    accent: u32,
    border: u32,
    panel: u32,
    heading: u32,
    code_background: u32,
    table_background: u32,
    quote: u32,
}

impl super::Emerald {
    pub(super) fn render_preview_row(
        index: usize,
        row: &emerald::preview::PreviewRow,
        blocks: &[RenderBlock],
        style: RenderStyle,
        theme: emerald::theme::EmeraldTheme,
        prepared: &PreparedPreview,
        document_path: &Path,
        entity: gpui::Entity<Emerald>,
    ) -> AnyElement {
        Self::render_block_contents(
            index,
            &blocks[row.block_index],
            style,
            theme,
            prepared,
            document_path,
            entity,
            row,
        )
    }

    pub(super) fn render_block(
        index: usize,
        block: &RenderBlock,
        style: RenderStyle,
        theme: emerald::theme::EmeraldTheme,
        prepared: &PreparedPreview,
        document_path: &Path,
        entity: gpui::Entity<Emerald>,
    ) -> AnyElement {
        Self::render_block_contents(
            index,
            block,
            style,
            theme,
            prepared,
            document_path,
            entity,
            &emerald::preview::PreviewRow {
                block_index: index,
                range: 0..0,
                first: true,
                last: true,
            },
        )
    }

    fn render_block_contents(
        index: usize,
        block: &RenderBlock,
        style: RenderStyle,
        theme: emerald::theme::EmeraldTheme,
        prepared: &PreparedPreview,
        document_path: &Path,
        entity: gpui::Entity<Emerald>,
        slice: &emerald::preview::PreviewRow,
    ) -> AnyElement {
        let text_color = style.text.unwrap_or(theme.text);
        let muted_text = style.muted_text.unwrap_or(theme.muted_text);
        let accent = style.accent.unwrap_or(theme.accent);
        let border = style.border.unwrap_or(theme.panel_border);
        let panel = style.panel.unwrap_or(theme.sidebar);
        let heading = style.heading.unwrap_or(text_color);
        let code_background = style.code_background.unwrap_or(theme.code_background);
        let table_background = style.table_background.unwrap_or(theme.panel);
        let quote = style.quote.unwrap_or(muted_text);

        let context = RenderContext {
            index,
            style,
            theme,
            prepared,
            document_path,
            entity: entity.clone(),
            slice,
            text_color,
            muted_text,
            accent,
            border,
            panel,
            heading,
            code_background,
            table_background,
            quote,
        };
        match block {
            RenderBlock::TableOfContents(_) => table_of_contents::render(block, &context),
            RenderBlock::Heading { .. } => heading::render(block, &context),
            RenderBlock::Paragraph(_) => paragraph::render(block, &context),
            RenderBlock::Image { .. } => image::render(block, &context),
            RenderBlock::UnorderedList(_) => unordered_list::render(block, &context),
            RenderBlock::OrderedList(_) => ordered_list::render(block, &context),
            RenderBlock::Quote(_) => quote::render(block, &context),
            RenderBlock::ThematicBreak => thematic_break::render(block, &context),
            RenderBlock::Admonition { .. } => admonition::render(block, &context),
            RenderBlock::Code { .. } => code::render(block, &context),
            RenderBlock::Diagram { .. } => diagram::render(block, &context),
            RenderBlock::Table(_) => table::render(block, &context),
        }
    }
}

#[cfg(test)]
#[path = "tests/rendered_view.rs"]
mod tests;
