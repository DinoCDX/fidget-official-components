use crate::{
    color::Rgba,
    config::{CustomPalette, ThemeMode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedTheme {
    pub scheme: Scheme,
    pub primary: Rgba,
    pub on_primary: Rgba,
    pub primary_container: Rgba,
    pub on_primary_container: Rgba,
    pub surface: Rgba,
    pub surface_container: Rgba,
    pub surface_container_high: Rgba,
    pub on_surface: Rgba,
    pub on_surface_variant: Rgba,
    pub outline: Rgba,
    pub error: Rgba,
}

impl ResolvedTheme {
    const LIGHT: Self = Self {
        scheme: Scheme::Light,
        primary: Rgba::rgb(0x67, 0x50, 0xA4),
        on_primary: Rgba::rgb(0xFF, 0xFF, 0xFF),
        primary_container: Rgba::rgb(0xEA, 0xDD, 0xFF),
        on_primary_container: Rgba::rgb(0x21, 0x00, 0x5D),
        surface: Rgba::rgb(0xFE, 0xF7, 0xFF),
        surface_container: Rgba::rgb(0xF3, 0xED, 0xF7),
        surface_container_high: Rgba::rgb(0xEC, 0xE6, 0xF0),
        on_surface: Rgba::rgb(0x1D, 0x1B, 0x20),
        on_surface_variant: Rgba::rgb(0x49, 0x45, 0x4F),
        outline: Rgba::rgb(0x79, 0x74, 0x7E),
        error: Rgba::rgb(0xB3, 0x26, 0x1E),
    };

    const DARK: Self = Self {
        scheme: Scheme::Dark,
        primary: Rgba::rgb(0xD0, 0xBC, 0xFF),
        on_primary: Rgba::rgb(0x38, 0x1E, 0x72),
        primary_container: Rgba::rgb(0x4F, 0x37, 0x8B),
        on_primary_container: Rgba::rgb(0xEA, 0xDD, 0xFF),
        surface: Rgba::rgb(0x14, 0x12, 0x18),
        surface_container: Rgba::rgb(0x21, 0x1F, 0x26),
        surface_container_high: Rgba::rgb(0x2B, 0x29, 0x30),
        on_surface: Rgba::rgb(0xE6, 0xE0, 0xE9),
        on_surface_variant: Rgba::rgb(0xCA, 0xC4, 0xD0),
        outline: Rgba::rgb(0x93, 0x8F, 0x99),
        error: Rgba::rgb(0xF2, 0xB8, 0xB5),
    };

    pub fn resolve(mode: ThemeMode, custom: &CustomPalette, system_dark: bool) -> Self {
        let scheme = match mode {
            ThemeMode::Light => Scheme::Light,
            ThemeMode::Dark => Scheme::Dark,
            ThemeMode::System | ThemeMode::Custom => {
                if system_dark {
                    Scheme::Dark
                } else {
                    Scheme::Light
                }
            }
        };
        let mut theme = match scheme {
            Scheme::Light => Self::LIGHT,
            Scheme::Dark => Self::DARK,
        };
        if mode == ThemeMode::Custom {
            theme.apply(custom);
        }
        theme
    }

    fn apply(&mut self, custom: &CustomPalette) {
        macro_rules! overlay {
            ($($field:ident),+) => {
                $(
                    if let Some(color) = custom.$field {
                        self.$field = color;
                    }
                )+
            };
        }
        overlay!(primary, on_primary, surface, on_surface, surface_container, outline, error);
    }
}
