use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmeraldTheme {
    pub id: ThemeId,
    pub name: &'static str,
    pub window_background: u32,
    pub sidebar: u32,
    pub panel: u32,
    pub menu: u32,
    pub panel_border: u32,
    pub text: u32,
    pub muted_text: u32,
    pub faint_text: u32,
    pub accent: u32,
    pub selection: u32,
    pub hover: u32,
    pub active_file: u32,
    pub active_file_text: u32,
    pub destructive: u32,
    pub code_background: u32,
    pub graph_background: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ThemeId {
    Everforest,
    TokyoNight,
    Gruvbox,
    Dracula,
}

impl ThemeId {
    pub const ALL: [Self; 4] = [
        Self::Everforest,
        Self::TokyoNight,
        Self::Gruvbox,
        Self::Dracula,
    ];

    pub const fn name(self) -> &'static str {
        self.theme().name
    }

    pub const fn theme(self) -> EmeraldTheme {
        match self {
            Self::Everforest => EVERFOREST_DARK,
            Self::TokyoNight => TOKYO_NIGHT,
            Self::Gruvbox => GRUVBOX,
            Self::Dracula => DRACULA,
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::Everforest => "everforest",
            Self::TokyoNight => "tokyo-night",
            Self::Gruvbox => "gruvbox",
            Self::Dracula => "dracula",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.id() == id)
    }
}

pub const EVERFOREST_DARK: EmeraldTheme = EmeraldTheme {
    id: ThemeId::Everforest,
    name: "Everforest",
    window_background: 0x1f2a2e,
    sidebar: 0x162023,
    panel: 0x263237,
    menu: 0x202b30,
    panel_border: 0x46535a,
    text: 0xe4dfcf,
    muted_text: 0xa8b2aa,
    faint_text: 0x84918c,
    accent: 0x8fbf9f,
    selection: 0x43545a,
    hover: 0x2d3a3f,
    active_file: 0xd6b678,
    active_file_text: 0x172124,
    destructive: 0xe67e80,
    code_background: 0x202b30,
    graph_background: 0x1b262a,
};

pub const TOKYO_NIGHT: EmeraldTheme = EmeraldTheme {
    id: ThemeId::TokyoNight,
    name: "Tokyo Night",
    window_background: 0x1a1b26,
    sidebar: 0x16161e,
    panel: 0x1f2335,
    menu: 0x202436,
    panel_border: 0x343a52,
    text: 0xc0caf5,
    muted_text: 0xa9b1d6,
    faint_text: 0x737aa2,
    accent: 0x7aa2f7,
    selection: 0x364a82,
    hover: 0x292e42,
    active_file: 0x2d3e68,
    active_file_text: 0xc0caf5,
    destructive: 0xf7768e,
    code_background: 0x16161e,
    graph_background: 0x171821,
};

pub const GRUVBOX: EmeraldTheme = EmeraldTheme {
    id: ThemeId::Gruvbox,
    name: "Gruvbox Dark",
    window_background: 0x282828,
    sidebar: 0x1d2021,
    panel: 0x32302f,
    menu: 0x282828,
    panel_border: 0x504945,
    text: 0xebdbb2,
    muted_text: 0xbdae93,
    faint_text: 0x928374,
    accent: 0xd79921,
    selection: 0x665c54,
    hover: 0x3c3836,
    active_file: 0x504126,
    active_file_text: 0xfbf1c7,
    destructive: 0xfb4934,
    code_background: 0x1d2021,
    graph_background: 0x222222,
};

pub const DRACULA: EmeraldTheme = EmeraldTheme {
    id: ThemeId::Dracula,
    name: "Dracula",
    window_background: 0x282a36,
    sidebar: 0x21222c,
    panel: 0x2e303e,
    menu: 0x343746,
    panel_border: 0x44475a,
    text: 0xf8f8f2,
    muted_text: 0xc8c8d0,
    faint_text: 0x9294a3,
    accent: 0xbd93f9,
    selection: 0x44475a,
    hover: 0x383a4a,
    active_file: 0x4b4260,
    active_file_text: 0xf8f8f2,
    destructive: 0xff5555,
    code_background: 0x21222c,
    graph_background: 0x242530,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemePreferences {
    pub selected: ThemeId,
    pub favorites: Vec<ThemeId>,
}

impl Default for ThemePreferences {
    fn default() -> Self {
        Self {
            selected: ThemeId::Everforest,
            favorites: vec![ThemeId::Everforest],
        }
    }
}

impl ThemePreferences {
    pub fn load() -> Self {
        let Ok(contents) = std::fs::read_to_string(preferences_path()) else {
            return Self::default();
        };
        let mut preferences = Self::default();
        for line in contents.lines() {
            if let Some(id) = line.strip_prefix("selected=") {
                preferences.selected = ThemeId::from_id(id).unwrap_or(ThemeId::Everforest);
            } else if let Some(ids) = line.strip_prefix("favorites=") {
                preferences.favorites = ids.split(',').filter_map(ThemeId::from_id).collect();
            }
        }
        preferences
    }

    pub fn save(&self) {
        let path = preferences_path();
        let Some(parent) = path.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let favorites = self
            .favorites
            .iter()
            .map(|theme| theme.id())
            .collect::<Vec<_>>()
            .join(",");
        if let Err(error) = std::fs::write(
            path,
            format!("selected={}\nfavorites={favorites}\n", self.selected.id()),
        ) {
            tracing::warn!(%error, "could not save theme preferences");
        }
    }
}

fn preferences_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("emerald/ui-preferences")
}

#[cfg(test)]
#[path = "tests/theme.rs"]
mod tests;
