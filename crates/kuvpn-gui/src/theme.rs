use iced::Color;
use serde::{Deserialize, Serialize};

/// All semantic color slots for a palette variant. All fields are `Color` which
/// is `Copy`, so `Palette` itself is `Copy` — safe to capture in `'static` closures.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub border: Color,
    pub border_subtle: Color,

    pub accent: Color,
    pub accent_hover: Color,

    pub success: Color,
    pub warning: Color,
    pub danger: Color,

    pub text: Color,
    pub text_muted: Color,
    pub text_disabled: Color,

    pub overlay: Color,
}

/// Named color families — ten aesthetic moods, each with a dark and light variant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum PaletteFamily {
    /// KU Burgundy — the current default palette.
    #[default]
    Crimson,
    /// Cool steel-blue (Breeze / KDE-inspired).
    Frost,
    /// Neutral gray (Adwaita / GNOME-inspired).
    Pebble,
    /// Blue-gray (Windows Fluent-inspired).
    Slate,
    /// Warm gray (macOS-inspired).
    Sand,
    /// Dark green with vivid emerald accent.
    Forest,
    /// Dark teal with bright cyan accent.
    Ocean,
    /// Dark purple with vivid violet accent.
    Violet,
    /// Dark amber with bright warm accent.
    Ember,
    /// Dark pink with vivid rose accent.
    Rose,
}

impl PaletteFamily {
    pub const ALL: [PaletteFamily; 10] = [
        PaletteFamily::Crimson,
        PaletteFamily::Frost,
        PaletteFamily::Pebble,
        PaletteFamily::Slate,
        PaletteFamily::Sand,
        PaletteFamily::Forest,
        PaletteFamily::Ocean,
        PaletteFamily::Violet,
        PaletteFamily::Ember,
        PaletteFamily::Rose,
    ];

    pub fn palette(self, dark: bool) -> Palette {
        match (self, dark) {
            (PaletteFamily::Crimson, true) => crimson_dark(),
            (PaletteFamily::Crimson, false) => crimson_light(),
            (PaletteFamily::Frost, true) => frost_dark(),
            (PaletteFamily::Frost, false) => frost_light(),
            (PaletteFamily::Pebble, true) => pebble_dark(),
            (PaletteFamily::Pebble, false) => pebble_light(),
            (PaletteFamily::Slate, true) => slate_dark(),
            (PaletteFamily::Slate, false) => slate_light(),
            (PaletteFamily::Sand, true) => sand_dark(),
            (PaletteFamily::Sand, false) => sand_light(),
            (PaletteFamily::Forest, true) => forest_dark(),
            (PaletteFamily::Forest, false) => forest_light(),
            (PaletteFamily::Ocean, true) => ocean_dark(),
            (PaletteFamily::Ocean, false) => ocean_light(),
            (PaletteFamily::Violet, true) => violet_dark(),
            (PaletteFamily::Violet, false) => violet_light(),
            (PaletteFamily::Ember, true) => ember_dark(),
            (PaletteFamily::Ember, false) => ember_light(),
            (PaletteFamily::Rose, true) => rose_dark(),
            (PaletteFamily::Rose, false) => rose_light(),
        }
    }

    pub fn default_rounding(self) -> Rounding {
        match self {
            PaletteFamily::Crimson => Rounding::Rounded,
            PaletteFamily::Frost => Rounding::Rounded,
            PaletteFamily::Pebble => Rounding::Smooth,
            PaletteFamily::Slate => Rounding::Sharp,
            PaletteFamily::Sand => Rounding::Rounded,
            PaletteFamily::Forest => Rounding::Smooth,
            PaletteFamily::Ocean => Rounding::Rounded,
            PaletteFamily::Violet => Rounding::Smooth,
            PaletteFamily::Ember => Rounding::Rounded,
            PaletteFamily::Rose => Rounding::Pill,
        }
    }

    pub fn default_shadow(self) -> ShadowDepth {
        match self {
            PaletteFamily::Crimson => ShadowDepth::Medium,
            PaletteFamily::Frost => ShadowDepth::Subtle,
            PaletteFamily::Pebble => ShadowDepth::Medium,
            PaletteFamily::Slate => ShadowDepth::Subtle,
            PaletteFamily::Sand => ShadowDepth::Subtle,
            PaletteFamily::Forest => ShadowDepth::Medium,
            PaletteFamily::Ocean => ShadowDepth::Medium,
            PaletteFamily::Violet => ShadowDepth::Elevated,
            PaletteFamily::Ember => ShadowDepth::Medium,
            PaletteFamily::Rose => ShadowDepth::Subtle,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PaletteFamily::Crimson => "Crimson",
            PaletteFamily::Frost => "Frost",
            PaletteFamily::Pebble => "Pebble",
            PaletteFamily::Slate => "Slate",
            PaletteFamily::Sand => "Sand",
            PaletteFamily::Forest => "Forest",
            PaletteFamily::Ocean => "Ocean",
            PaletteFamily::Violet => "Violet",
            PaletteFamily::Ember => "Ember",
            PaletteFamily::Rose => "Rose",
        }
    }
}

// ── Palette definitions ───────────────────────────────────────────────────────

fn crimson_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.07, 0.07, 0.07),
        surface: Color::from_rgb(0.12, 0.12, 0.12),
        surface_raised: Color::from_rgb(0.16, 0.16, 0.16),
        border: Color::from_rgb(0.25, 0.25, 0.25),
        border_subtle: Color::from_rgb(0.18, 0.18, 0.18),
        accent: Color::from_rgb(0.50, 0.0, 0.125),
        accent_hover: Color::from_rgb(0.60, 0.06, 0.19),
        // warm sage green + rich golden amber — warm tones echo the burgundy
        success: Color::from_rgb(0.38, 0.70, 0.32),
        warning: Color::from_rgb(0.88, 0.68, 0.16),
        danger: Color::from_rgb(0.80, 0.20, 0.20),
        text: Color::from_rgb(0.85, 0.85, 0.85),
        text_muted: Color::from_rgb(0.50, 0.50, 0.50),
        text_disabled: Color::from_rgb(0.35, 0.35, 0.35),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.85),
    }
}

fn crimson_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.95, 0.95, 0.95),
        surface: Color::from_rgb(1.0, 1.0, 1.0),
        surface_raised: Color::from_rgb(0.97, 0.97, 0.97),
        border: Color::from_rgb(0.70, 0.70, 0.70),
        border_subtle: Color::from_rgb(0.82, 0.82, 0.82),
        accent: Color::from_rgb(0.50, 0.0, 0.125),
        accent_hover: Color::from_rgb(0.60, 0.06, 0.19),
        success: Color::from_rgb(0.22, 0.52, 0.18),
        warning: Color::from_rgb(0.70, 0.50, 0.06),
        danger: Color::from_rgb(0.72, 0.12, 0.12),
        text: Color::from_rgb(0.10, 0.10, 0.10),
        text_muted: Color::from_rgb(0.45, 0.45, 0.45),
        text_disabled: Color::from_rgb(0.65, 0.65, 0.65),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn frost_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.06, 0.08, 0.10),
        surface: Color::from_rgb(0.10, 0.13, 0.17),
        surface_raised: Color::from_rgb(0.13, 0.17, 0.22),
        border: Color::from_rgb(0.22, 0.30, 0.38),
        border_subtle: Color::from_rgb(0.16, 0.22, 0.28),
        accent: Color::from_rgb(0.18, 0.48, 0.72),
        accent_hover: Color::from_rgb(0.24, 0.55, 0.82),
        // cool icy mint green + cool chartreuse-yellow — icy like the steel-blue accent
        success: Color::from_rgb(0.25, 0.75, 0.50),
        warning: Color::from_rgb(0.72, 0.78, 0.18),
        danger: Color::from_rgb(0.80, 0.20, 0.20),
        text: Color::from_rgb(0.88, 0.90, 0.92),
        text_muted: Color::from_rgb(0.50, 0.58, 0.65),
        text_disabled: Color::from_rgb(0.35, 0.40, 0.45),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.85),
    }
}

fn frost_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.92, 0.94, 0.97),
        surface: Color::from_rgb(1.0, 1.0, 1.0),
        surface_raised: Color::from_rgb(0.95, 0.97, 1.0),
        border: Color::from_rgb(0.62, 0.72, 0.82),
        border_subtle: Color::from_rgb(0.78, 0.85, 0.92),
        accent: Color::from_rgb(0.18, 0.48, 0.72),
        accent_hover: Color::from_rgb(0.12, 0.38, 0.60),
        success: Color::from_rgb(0.12, 0.52, 0.34),
        warning: Color::from_rgb(0.55, 0.58, 0.06),
        danger: Color::from_rgb(0.72, 0.12, 0.12),
        text: Color::from_rgb(0.08, 0.12, 0.18),
        text_muted: Color::from_rgb(0.38, 0.48, 0.58),
        text_disabled: Color::from_rgb(0.60, 0.68, 0.75),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn pebble_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.08, 0.08, 0.08),
        surface: Color::from_rgb(0.13, 0.13, 0.13),
        surface_raised: Color::from_rgb(0.17, 0.17, 0.17),
        border: Color::from_rgb(0.28, 0.28, 0.28),
        border_subtle: Color::from_rgb(0.20, 0.20, 0.20),
        accent: Color::from_rgb(0.40, 0.60, 0.85),
        accent_hover: Color::from_rgb(0.48, 0.68, 0.95),
        // clean vivid green + warm amber — neutral grey palette lets these pop clearly
        success: Color::from_rgb(0.32, 0.72, 0.40),
        warning: Color::from_rgb(0.85, 0.68, 0.18),
        danger: Color::from_rgb(0.80, 0.22, 0.22),
        text: Color::from_rgb(0.88, 0.88, 0.88),
        text_muted: Color::from_rgb(0.55, 0.55, 0.55),
        text_disabled: Color::from_rgb(0.38, 0.38, 0.38),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.85),
    }
}

fn pebble_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.94, 0.94, 0.94),
        surface: Color::from_rgb(1.0, 1.0, 1.0),
        surface_raised: Color::from_rgb(0.97, 0.97, 0.97),
        border: Color::from_rgb(0.70, 0.70, 0.70),
        border_subtle: Color::from_rgb(0.82, 0.82, 0.82),
        accent: Color::from_rgb(0.25, 0.45, 0.75),
        accent_hover: Color::from_rgb(0.18, 0.35, 0.62),
        success: Color::from_rgb(0.18, 0.54, 0.24),
        warning: Color::from_rgb(0.66, 0.50, 0.06),
        danger: Color::from_rgb(0.72, 0.15, 0.15),
        text: Color::from_rgb(0.10, 0.10, 0.10),
        text_muted: Color::from_rgb(0.45, 0.45, 0.45),
        text_disabled: Color::from_rgb(0.65, 0.65, 0.65),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn slate_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.07, 0.09, 0.11),
        surface: Color::from_rgb(0.11, 0.14, 0.17),
        surface_raised: Color::from_rgb(0.14, 0.18, 0.22),
        border: Color::from_rgb(0.25, 0.32, 0.38),
        border_subtle: Color::from_rgb(0.18, 0.24, 0.30),
        accent: Color::from_rgb(0.25, 0.48, 0.72),
        accent_hover: Color::from_rgb(0.32, 0.56, 0.82),
        // blue-green teal + golden-steel yellow — both echo the slate-blue character
        success: Color::from_rgb(0.22, 0.68, 0.52),
        warning: Color::from_rgb(0.75, 0.72, 0.20),
        danger: Color::from_rgb(0.78, 0.22, 0.22),
        text: Color::from_rgb(0.88, 0.90, 0.92),
        text_muted: Color::from_rgb(0.52, 0.58, 0.65),
        text_disabled: Color::from_rgb(0.38, 0.42, 0.48),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.85),
    }
}

fn slate_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.90, 0.92, 0.95),
        surface: Color::from_rgb(0.98, 0.99, 1.0),
        surface_raised: Color::from_rgb(0.94, 0.96, 0.99),
        border: Color::from_rgb(0.62, 0.70, 0.78),
        border_subtle: Color::from_rgb(0.78, 0.84, 0.90),
        accent: Color::from_rgb(0.18, 0.42, 0.68),
        accent_hover: Color::from_rgb(0.12, 0.32, 0.55),
        success: Color::from_rgb(0.10, 0.50, 0.36),
        warning: Color::from_rgb(0.58, 0.54, 0.06),
        danger: Color::from_rgb(0.70, 0.12, 0.12),
        text: Color::from_rgb(0.08, 0.10, 0.14),
        text_muted: Color::from_rgb(0.38, 0.45, 0.52),
        text_disabled: Color::from_rgb(0.60, 0.66, 0.72),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn sand_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.09, 0.08, 0.07),
        surface: Color::from_rgb(0.14, 0.13, 0.11),
        surface_raised: Color::from_rgb(0.18, 0.17, 0.15),
        border: Color::from_rgb(0.30, 0.27, 0.24),
        border_subtle: Color::from_rgb(0.22, 0.20, 0.18),
        accent: Color::from_rgb(0.45, 0.35, 0.22),
        accent_hover: Color::from_rgb(0.55, 0.42, 0.28),
        // olive-warm green + deep golden amber — earthy tones complement warm sand/tan
        success: Color::from_rgb(0.44, 0.64, 0.24),
        warning: Color::from_rgb(0.92, 0.70, 0.12),
        danger: Color::from_rgb(0.80, 0.22, 0.22),
        text: Color::from_rgb(0.90, 0.88, 0.85),
        text_muted: Color::from_rgb(0.55, 0.52, 0.48),
        text_disabled: Color::from_rgb(0.40, 0.38, 0.35),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.85),
    }
}

fn sand_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.96, 0.94, 0.92),
        surface: Color::from_rgb(1.0, 0.99, 0.97),
        surface_raised: Color::from_rgb(0.98, 0.96, 0.93),
        border: Color::from_rgb(0.72, 0.68, 0.62),
        border_subtle: Color::from_rgb(0.84, 0.80, 0.75),
        accent: Color::from_rgb(0.48, 0.35, 0.20),
        accent_hover: Color::from_rgb(0.38, 0.26, 0.14),
        success: Color::from_rgb(0.28, 0.50, 0.13),
        warning: Color::from_rgb(0.72, 0.52, 0.04),
        danger: Color::from_rgb(0.72, 0.14, 0.14),
        text: Color::from_rgb(0.12, 0.10, 0.08),
        text_muted: Color::from_rgb(0.45, 0.42, 0.38),
        text_disabled: Color::from_rgb(0.65, 0.62, 0.58),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn forest_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.05, 0.08, 0.06),
        surface: Color::from_rgb(0.08, 0.12, 0.09),
        surface_raised: Color::from_rgb(0.11, 0.17, 0.13),
        border: Color::from_rgb(0.20, 0.32, 0.24),
        border_subtle: Color::from_rgb(0.14, 0.22, 0.16),
        accent: Color::from_rgb(0.15, 0.68, 0.38),
        accent_hover: Color::from_rgb(0.22, 0.78, 0.46),
        // vivid emerald (near-accent) + bright lime-yellow — forest energy, natural vitality
        success: Color::from_rgb(0.16, 0.82, 0.40),
        warning: Color::from_rgb(0.80, 0.80, 0.08),
        danger: Color::from_rgb(0.80, 0.22, 0.22),
        text: Color::from_rgb(0.85, 0.92, 0.87),
        text_muted: Color::from_rgb(0.48, 0.60, 0.52),
        text_disabled: Color::from_rgb(0.32, 0.42, 0.36),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.85),
    }
}

fn forest_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.91, 0.95, 0.92),
        surface: Color::from_rgb(0.98, 1.0, 0.98),
        surface_raised: Color::from_rgb(0.93, 0.97, 0.94),
        border: Color::from_rgb(0.55, 0.72, 0.60),
        border_subtle: Color::from_rgb(0.75, 0.87, 0.78),
        accent: Color::from_rgb(0.10, 0.52, 0.28),
        accent_hover: Color::from_rgb(0.07, 0.40, 0.20),
        success: Color::from_rgb(0.06, 0.56, 0.22),
        warning: Color::from_rgb(0.60, 0.58, 0.03),
        danger: Color::from_rgb(0.70, 0.12, 0.12),
        text: Color::from_rgb(0.07, 0.14, 0.09),
        text_muted: Color::from_rgb(0.32, 0.48, 0.36),
        text_disabled: Color::from_rgb(0.58, 0.70, 0.62),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn ocean_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.04, 0.08, 0.10),
        surface: Color::from_rgb(0.07, 0.12, 0.16),
        surface_raised: Color::from_rgb(0.10, 0.17, 0.22),
        border: Color::from_rgb(0.18, 0.32, 0.40),
        border_subtle: Color::from_rgb(0.12, 0.22, 0.30),
        accent: Color::from_rgb(0.10, 0.68, 0.75),
        accent_hover: Color::from_rgb(0.18, 0.78, 0.86),
        // oceanic teal-green + golden-aqua yellow — warm gold contrasts the cool cyan
        success: Color::from_rgb(0.18, 0.76, 0.58),
        warning: Color::from_rgb(0.68, 0.80, 0.14),
        danger: Color::from_rgb(0.80, 0.22, 0.28),
        text: Color::from_rgb(0.83, 0.93, 0.96),
        text_muted: Color::from_rgb(0.42, 0.62, 0.70),
        text_disabled: Color::from_rgb(0.28, 0.42, 0.50),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.85),
    }
}

fn ocean_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.90, 0.95, 0.97),
        surface: Color::from_rgb(0.97, 1.0, 1.0),
        surface_raised: Color::from_rgb(0.92, 0.97, 0.99),
        border: Color::from_rgb(0.50, 0.72, 0.80),
        border_subtle: Color::from_rgb(0.72, 0.87, 0.92),
        accent: Color::from_rgb(0.05, 0.52, 0.60),
        accent_hover: Color::from_rgb(0.03, 0.40, 0.48),
        success: Color::from_rgb(0.06, 0.52, 0.40),
        warning: Color::from_rgb(0.50, 0.58, 0.05),
        danger: Color::from_rgb(0.68, 0.12, 0.16),
        text: Color::from_rgb(0.05, 0.12, 0.18),
        text_muted: Color::from_rgb(0.28, 0.48, 0.58),
        text_disabled: Color::from_rgb(0.55, 0.70, 0.78),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn violet_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.07, 0.05, 0.10),
        surface: Color::from_rgb(0.11, 0.08, 0.16),
        surface_raised: Color::from_rgb(0.15, 0.11, 0.22),
        border: Color::from_rgb(0.28, 0.20, 0.40),
        border_subtle: Color::from_rgb(0.20, 0.14, 0.28),
        accent: Color::from_rgb(0.65, 0.35, 0.95),
        accent_hover: Color::from_rgb(0.72, 0.44, 1.0),
        // vivid electric green + vivid gold — both pop sharply against deep purple
        success: Color::from_rgb(0.28, 0.85, 0.45),
        warning: Color::from_rgb(0.92, 0.78, 0.08),
        danger: Color::from_rgb(0.80, 0.22, 0.30),
        text: Color::from_rgb(0.90, 0.86, 0.96),
        text_muted: Color::from_rgb(0.55, 0.45, 0.70),
        text_disabled: Color::from_rgb(0.38, 0.30, 0.50),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.88),
    }
}

fn violet_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.94, 0.92, 0.98),
        surface: Color::from_rgb(1.0, 0.99, 1.0),
        surface_raised: Color::from_rgb(0.96, 0.94, 0.99),
        border: Color::from_rgb(0.65, 0.55, 0.82),
        border_subtle: Color::from_rgb(0.82, 0.76, 0.92),
        accent: Color::from_rgb(0.52, 0.22, 0.80),
        accent_hover: Color::from_rgb(0.40, 0.15, 0.65),
        success: Color::from_rgb(0.12, 0.58, 0.26),
        warning: Color::from_rgb(0.72, 0.56, 0.04),
        danger: Color::from_rgb(0.70, 0.12, 0.18),
        text: Color::from_rgb(0.10, 0.06, 0.18),
        text_muted: Color::from_rgb(0.42, 0.32, 0.58),
        text_disabled: Color::from_rgb(0.65, 0.58, 0.75),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn ember_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.09, 0.07, 0.04),
        surface: Color::from_rgb(0.14, 0.11, 0.06),
        surface_raised: Color::from_rgb(0.19, 0.15, 0.08),
        border: Color::from_rgb(0.35, 0.26, 0.12),
        border_subtle: Color::from_rgb(0.24, 0.18, 0.08),
        accent: Color::from_rgb(0.92, 0.55, 0.08),
        accent_hover: Color::from_rgb(1.0, 0.64, 0.15),
        // earthy warm green + intense fiery gold — both feel like living embers
        success: Color::from_rgb(0.34, 0.65, 0.26),
        warning: Color::from_rgb(0.96, 0.74, 0.08),
        danger: Color::from_rgb(0.82, 0.22, 0.18),
        text: Color::from_rgb(0.96, 0.90, 0.80),
        text_muted: Color::from_rgb(0.60, 0.50, 0.34),
        text_disabled: Color::from_rgb(0.42, 0.34, 0.22),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.88),
    }
}

fn ember_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.98, 0.95, 0.90),
        surface: Color::from_rgb(1.0, 0.99, 0.96),
        surface_raised: Color::from_rgb(0.99, 0.96, 0.91),
        border: Color::from_rgb(0.78, 0.62, 0.38),
        border_subtle: Color::from_rgb(0.90, 0.80, 0.62),
        accent: Color::from_rgb(0.72, 0.38, 0.02),
        accent_hover: Color::from_rgb(0.58, 0.28, 0.01),
        success: Color::from_rgb(0.20, 0.48, 0.14),
        warning: Color::from_rgb(0.74, 0.52, 0.03),
        danger: Color::from_rgb(0.70, 0.14, 0.10),
        text: Color::from_rgb(0.15, 0.10, 0.04),
        text_muted: Color::from_rgb(0.48, 0.36, 0.18),
        text_disabled: Color::from_rgb(0.68, 0.58, 0.42),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

fn rose_dark() -> Palette {
    Palette {
        bg: Color::from_rgb(0.09, 0.05, 0.07),
        surface: Color::from_rgb(0.14, 0.08, 0.11),
        surface_raised: Color::from_rgb(0.19, 0.11, 0.15),
        border: Color::from_rgb(0.35, 0.18, 0.26),
        border_subtle: Color::from_rgb(0.24, 0.12, 0.18),
        accent: Color::from_rgb(0.90, 0.30, 0.55),
        accent_hover: Color::from_rgb(1.0, 0.40, 0.64),
        // fresh vivid green + warm peachy-amber — both contrast the rose/pink character
        success: Color::from_rgb(0.26, 0.76, 0.42),
        warning: Color::from_rgb(0.90, 0.70, 0.15),
        danger: Color::from_rgb(0.82, 0.18, 0.22),
        text: Color::from_rgb(0.96, 0.88, 0.92),
        text_muted: Color::from_rgb(0.60, 0.40, 0.52),
        text_disabled: Color::from_rgb(0.42, 0.26, 0.34),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.88),
    }
}

fn rose_light() -> Palette {
    Palette {
        bg: Color::from_rgb(0.98, 0.93, 0.95),
        surface: Color::from_rgb(1.0, 0.98, 0.99),
        surface_raised: Color::from_rgb(0.99, 0.95, 0.97),
        border: Color::from_rgb(0.80, 0.55, 0.68),
        border_subtle: Color::from_rgb(0.90, 0.76, 0.83),
        accent: Color::from_rgb(0.75, 0.18, 0.40),
        accent_hover: Color::from_rgb(0.60, 0.12, 0.30),
        success: Color::from_rgb(0.12, 0.52, 0.26),
        warning: Color::from_rgb(0.70, 0.50, 0.07),
        danger: Color::from_rgb(0.70, 0.10, 0.14),
        text: Color::from_rgb(0.16, 0.06, 0.10),
        text_muted: Color::from_rgb(0.48, 0.28, 0.38),
        text_disabled: Color::from_rgb(0.68, 0.54, 0.60),
        overlay: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
    }
}

impl std::fmt::Display for PaletteFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

// ── Rounding ──────────────────────────────────────────────────────────────────

/// Corner radius style. `Copy` — safe to move into `'static` closures.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Rounding {
    /// 0 px — completely square corners.
    Square,
    /// 4 px — crisp, nearly square (Windows-flavoured).
    Sharp,
    /// 10 px — clearly rounded (Breeze / KDE).
    #[default]
    Rounded,
    /// 16 px — soft, card-like (Adwaita / GNOME).
    Smooth,
    /// 24 px — pill-shaped buttons, large cards.
    Pill,
}

impl Rounding {
    /// Main corner radius for cards and buttons.
    pub fn radius(self) -> f32 {
        match self {
            Rounding::Square => 0.0,
            Rounding::Sharp => 4.0,
            Rounding::Rounded => 10.0,
            Rounding::Smooth => 16.0,
            Rounding::Pill => 24.0,
        }
    }

    /// Smaller radius for inputs, badges, segment buttons.
    pub fn small_radius(self) -> f32 {
        match self {
            Rounding::Square => 0.0,
            Rounding::Sharp => 2.0,
            Rounding::Rounded => 6.0,
            Rounding::Smooth => 10.0,
            Rounding::Pill => 14.0,
        }
    }

    pub fn border_width(self) -> f32 {
        match self {
            Rounding::Square => 1.5,
            Rounding::Sharp => 1.5,
            Rounding::Rounded => 1.0,
            Rounding::Smooth => 1.0,
            Rounding::Pill => 0.5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rounding::Square => "Square",
            Rounding::Sharp => "Sharp",
            Rounding::Rounded => "Rounded",
            Rounding::Smooth => "Smooth",
            Rounding::Pill => "Pill",
        }
    }
}

// ── ShadowDepth ───────────────────────────────────────────────────────────────

/// Drop-shadow intensity. `Copy` — safe to move into `'static` closures.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum ShadowDepth {
    /// No shadow at all.
    None,
    /// Faint lift — 6 px blur.
    #[default]
    Subtle,
    /// Visible card elevation — 16 px blur.
    Medium,
    /// Strong floating shadow — 28 px blur.
    Elevated,
}

impl ShadowDepth {
    pub fn blur(self) -> f32 {
        match self {
            ShadowDepth::None => 0.0,
            ShadowDepth::Subtle => 6.0,
            ShadowDepth::Medium => 16.0,
            ShadowDepth::Elevated => 28.0,
        }
    }

    pub fn offset(self) -> f32 {
        match self {
            ShadowDepth::None => 0.0,
            ShadowDepth::Subtle => 1.0,
            ShadowDepth::Medium => 4.0,
            ShadowDepth::Elevated => 8.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ShadowDepth::None => "None",
            ShadowDepth::Subtle => "Subtle",
            ShadowDepth::Medium => "Medium",
            ShadowDepth::Elevated => "Elevated",
        }
    }
}

// ── ThemeConfig / AppTheme ────────────────────────────────────────────────────

/// Theme configuration persisted in `GuiSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub family: PaletteFamily,
    pub dark: bool,
    /// `None` → use `family.default_rounding()`; `Some` → explicit user override.
    #[serde(default)]
    pub rounding: Option<Rounding>,
    /// `None` → use `family.default_shadow()`; `Some` → explicit user override.
    #[serde(default)]
    pub shadow: Option<ShadowDepth>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            family: PaletteFamily::Crimson,
            dark: true,
            rounding: None,
            shadow: None,
        }
    }
}

impl ThemeConfig {
    /// Resolve into a concrete `AppTheme` ready for rendering.
    pub fn resolve(self) -> AppTheme {
        let palette = self.family.palette(self.dark);
        let rounding = self
            .rounding
            .unwrap_or_else(|| self.family.default_rounding());
        let shadow = self
            .shadow
            .unwrap_or_else(|| self.family.default_shadow());
        AppTheme {
            palette,
            rounding,
            shadow,
        }
    }
}

/// Fully-resolved theme, computed at render time from `ThemeConfig`.
#[derive(Debug, Clone, Copy)]
pub struct AppTheme {
    pub palette: Palette,
    pub rounding: Rounding,
    pub shadow: ShadowDepth,
}
