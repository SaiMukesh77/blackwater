use egui::{Color32, FontId, Rounding, Stroke, Style, Visuals};

pub struct Theme;

impl Theme {
    // Backgrounds
    pub const BG_PRIMARY: Color32 = Color32::from_rgb(17, 18, 20);       // Deep charcoal / obsidian
    pub const BG_SECONDARY: Color32 = Color32::from_rgb(24, 26, 30);     // Subtle elevated card / panel
    pub const BG_TERTIARY: Color32 = Color32::from_rgb(32, 35, 41);      // Input / hover background
    pub const BG_ACCENT_SUBTLE: Color32 = Color32::from_rgb(38, 33, 24); // Warm ember underglow

    // Brand / Frontier Accent Colors (warm amber / brass / gold)
    pub const ACCENT_GOLD: Color32 = Color32::from_rgb(212, 163, 115);    // Muted frontier gold
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(230, 149, 68);    // Rich amber
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(245, 185, 120);   // Bright hover gold

    // Status Colors
    pub const SUCCESS: Color32 = Color32::from_rgb(74, 222, 128);         // Emerald green
    pub const WARNING: Color32 = Color32::from_rgb(251, 191, 36);         // Warm amber warning
    pub const ERROR: Color32 = Color32::from_rgb(248, 113, 113);          // Crimson red
    pub const INFO: Color32 = Color32::from_rgb(96, 165, 250);            // Muted sky blue

    // Text Colors
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(241, 243, 247);   // Crisp white
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(156, 163, 175); // Slate gray
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(107, 114, 128);     // Deep muted gray

    // Borders
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(42, 45, 53);
    pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(212, 163, 115);

    pub fn apply(ctx: &egui::Context) {
        let mut visuals = Visuals::dark();

        visuals.override_text_color = Some(Self::TEXT_PRIMARY);
        visuals.panel_fill = Self::BG_PRIMARY;
        visuals.window_fill = Self::BG_SECONDARY;
        visuals.extreme_bg_color = Self::BG_PRIMARY;
        visuals.faint_bg_color = Self::BG_SECONDARY;

        // Button visuals
        visuals.widgets.inactive.bg_fill = Self::BG_SECONDARY;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Self::BORDER_SUBTLE);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_PRIMARY);
        visuals.widgets.inactive.rounding = Rounding::same(4.0);

        visuals.widgets.hovered.bg_fill = Self::BG_TERTIARY;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Self::ACCENT_GOLD);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.hovered.rounding = Rounding::same(4.0);

        visuals.widgets.active.bg_fill = Self::ACCENT_AMBER;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Self::ACCENT_HOVER);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
        visuals.widgets.active.rounding = Rounding::same(4.0);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(212, 163, 115, 60);
        visuals.selection.stroke = Stroke::new(1.0_f32, Self::ACCENT_GOLD);

        ctx.set_visuals(visuals);

        let mut style: Style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        ctx.set_style(style);
    }

    pub fn heading_font() -> FontId {
        FontId::proportional(22.0)
    }

    pub fn subhead_font() -> FontId {
        FontId::proportional(15.0)
    }

    pub fn body_font() -> FontId {
        FontId::proportional(13.0)
    }

    pub fn mono_font() -> FontId {
        FontId::monospace(12.0)
    }

    pub fn title_font() -> FontId {
        FontId::proportional(30.0)
    }
}
