#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmeraldTheme {
    pub background: u32,
    pub sidebar: u32,
    pub panel: u32,
    pub panel_border: u32,
    pub text: u32,
    pub muted_text: u32,
    pub accent: u32,
    pub selection: u32,
    pub active_file: u32,
    pub active_file_text: u32,
}

pub const EVERFOREST_DARK: EmeraldTheme = EmeraldTheme {
    background: 0x1f2a2e,
    sidebar: 0x162023,
    panel: 0x263237,
    panel_border: 0x46535a,
    text: 0xe4dfcf,
    muted_text: 0xa8b2aa,
    accent: 0x8fbf9f,
    selection: 0x43545a,
    active_file: 0xd6b678,
    active_file_text: 0x172124,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everforest_palette_exposes_the_required_editor_regions() {
        assert_ne!(EVERFOREST_DARK.sidebar, EVERFOREST_DARK.background);
        assert_ne!(EVERFOREST_DARK.panel, EVERFOREST_DARK.sidebar);
        assert_ne!(EVERFOREST_DARK.accent, EVERFOREST_DARK.text);
        assert_ne!(EVERFOREST_DARK.active_file, EVERFOREST_DARK.sidebar);
    }
}
