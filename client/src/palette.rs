use gloo_storage::Storage;

#[derive(Clone, PartialEq)]
pub(crate) struct Palette {
    pub shadow_stone_colors: [&'static str; 4],
    pub shadow_border_colors: [&'static str; 4],
    pub stone_colors: [&'static str; 4],
    pub stone_colors_hidden: [&'static str; 4],
    pub border_colors: [&'static str; 4],
    pub dead_mark_color: [&'static str; 4],
    pub background: &'static str,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum PaletteOption {
    Normal,
    Colorblind,
}

impl std::fmt::Display for PaletteOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PaletteOption {
    pub(crate) fn get() -> PaletteOption {
        let val = gloo_storage::LocalStorage::get::<String>("palette").ok();
        match val.as_deref() {
            Some("Normal") => PaletteOption::Normal,
            Some("Colorblind") => PaletteOption::Colorblind,
            _ => PaletteOption::Normal,
        }
    }

    pub(crate) fn save(&self) {
        gloo_storage::LocalStorage::set("palette", &format!("{:?}", self)).unwrap();
    }

    pub(crate) fn to_palette(&self) -> Palette {
        match self {
            PaletteOption::Normal => Palette {
                shadow_stone_colors: ["#000000a0", "#eeeeeea0", "#5074bca0", "#e0658fa0"],
                shadow_border_colors: ["#bbbbbb", "#555555", "#555555", "#555555"],
                stone_colors: ["#000000", "#eeeeee", "#5074bc", "#e0658f"],
                stone_colors_hidden: ["#00000080", "#eeeeee80", "#5074bc80", "#e0658f80"],
                border_colors: ["#555555", "#000000", "#000000", "#000000"],
                dead_mark_color: ["#eeeeee", "#000000", "#000000", "#000000"],
                background: "#e0bb6c",
            },
            PaletteOption::Colorblind => Palette {
                shadow_stone_colors: ["#000000a0", "#eeeeeea0", "#56b3e9a0", "#d52e00a0"],
                shadow_border_colors: ["#bbbbbb", "#555555", "#555555", "#555555"],
                stone_colors: ["#000000", "#eeeeee", "#56b3e9", "#d52e00"],
                stone_colors_hidden: ["#00000080", "#eeeeee80", "#56b3e980", "#d52e0080"],
                border_colors: ["#555555", "#000000", "#000000", "#000000"],
                dead_mark_color: ["#eeeeee", "#000000", "#000000", "#000000"],
                background: "#e0bb6c",
            },
        }
    }
}
