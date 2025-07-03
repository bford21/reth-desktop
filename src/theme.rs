use eframe::egui;

pub struct RethTheme;

impl RethTheme {
    // Reth brand colors - modern dark theme with blue accents
    pub const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(13, 17, 23);       // Dark blue-gray
    pub const SURFACE: egui::Color32 = egui::Color32::from_rgb(22, 27, 34);          // Lighter surface
    pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(35, 134, 54);          // Green accent
    pub const PRIMARY: egui::Color32 = egui::Color32::from_rgb(88, 166, 255);        // Blue primary
    pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(230, 237, 243);  // Light text
    pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(139, 148, 158); // Muted text
    pub const SUCCESS: egui::Color32 = egui::Color32::from_rgb(35, 134, 54);         // Success green
    pub const WARNING: egui::Color32 = egui::Color32::from_rgb(255, 159, 0);         // Warning orange
    pub const ERROR: egui::Color32 = egui::Color32::from_rgb(248, 81, 73);           // Error red
    pub const BORDER: egui::Color32 = egui::Color32::from_rgb(48, 54, 61);           // Border color

    pub fn apply(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        // Set dark theme as base
        style.visuals = egui::Visuals::dark();
        
        // Custom colors
        style.visuals.widgets.noninteractive.bg_fill = Self::SURFACE;
        style.visuals.widgets.noninteractive.weak_bg_fill = Self::BACKGROUND;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Self::BORDER);
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, Self::TEXT_SECONDARY);
        
        // Interactive widgets
        style.visuals.widgets.inactive.bg_fill = Self::SURFACE;
        style.visuals.widgets.inactive.weak_bg_fill = Self::BACKGROUND;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Self::BORDER);
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, Self::TEXT_PRIMARY);
        
        // Hovered widgets
        style.visuals.widgets.hovered.bg_fill = Self::PRIMARY.gamma_multiply(0.8);
        style.visuals.widgets.hovered.weak_bg_fill = Self::SURFACE;
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Self::PRIMARY);
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, Self::TEXT_PRIMARY);
        
        // Active/pressed widgets
        style.visuals.widgets.active.bg_fill = Self::PRIMARY;
        style.visuals.widgets.active.weak_bg_fill = Self::SURFACE;
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, Self::PRIMARY);
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, Self::TEXT_PRIMARY);
        
        // Background colors
        style.visuals.window_fill = Self::BACKGROUND;
        style.visuals.panel_fill = Self::BACKGROUND;
        style.visuals.faint_bg_color = Self::SURFACE;
        
        // Text colors - these are method-based now, so we skip direct assignment
        
        // Spacing and sizing for modern look
        style.spacing.item_spacing = egui::vec2(12.0, 8.0);
        style.spacing.button_padding = egui::vec2(16.0, 8.0);
        style.spacing.indent = 20.0;
        style.spacing.window_margin = egui::style::Margin::same(16.0);
        
        // Rounded corners for modern look
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
        style.visuals.window_rounding = egui::Rounding::same(12.0);
        
        ctx.set_style(style);
    }
    
    pub fn heading_text(text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(24.0)
            .color(Self::TEXT_PRIMARY)
            .strong()
    }
    
    pub fn subheading_text(text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(18.0)
            .color(Self::TEXT_PRIMARY)
            .strong()
    }
    
    pub fn body_text(text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .color(Self::TEXT_PRIMARY)
    }
    
    pub fn muted_text(text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(13.0)
            .color(Self::TEXT_SECONDARY)
    }
    
    pub fn success_text(text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .color(Self::SUCCESS)
            .strong()
    }
    
    pub fn warning_text(text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .color(Self::WARNING)
            .strong()
    }
    
    pub fn error_text(text: &str) -> egui::RichText {
        egui::RichText::new(text)
            .size(14.0)
            .color(Self::ERROR)
            .strong()
    }
}