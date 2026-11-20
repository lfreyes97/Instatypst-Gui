//! Paleta (oscura/clara) con acento cálido y ajustes globales de egui.

use std::cell::Cell;

use egui::{Color32, CornerRadius, Stroke};

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub inset: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_bg: Color32,
    pub on_accent: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub err: Color32,
}

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub const DARK: Palette = Palette {
    dark: true,
    bg: c(0x0b, 0x0b, 0x0c),
    panel: c(0x16, 0x16, 0x19),
    inset: c(0x1d, 0x1d, 0x21),
    hover: c(0x28, 0x28, 0x2d),
    border: c(0x2a, 0x2a, 0x2f),
    text: c(0xec, 0xec, 0xee),
    muted: c(0x8b, 0x8b, 0x94),
    accent: c(0xf2, 0xa1, 0x4e),
    accent_bg: c(0x2e, 0x24, 0x1a),
    on_accent: c(0x1a, 0x12, 0x08),
    ok: c(0x8b, 0xe0, 0x9b),
    warn: c(0xe0, 0xb4, 0x5a),
    err: c(0xf0, 0x6b, 0x6b),
};

pub const LIGHT: Palette = Palette {
    dark: false,
    bg: c(0xe9, 0xe9, 0xed),
    panel: c(0xff, 0xff, 0xff),
    inset: c(0xf1, 0xf1, 0xf4),
    hover: c(0xe3, 0xe3, 0xe9),
    border: c(0xd8, 0xd8, 0xdf),
    text: c(0x1b, 0x1b, 0x1f),
    muted: c(0x6b, 0x6b, 0x76),
    accent: c(0xe8, 0x82, 0x0c),
    accent_bg: c(0xfd, 0xee, 0xdc),
    on_accent: c(0x1a, 0x12, 0x08),
    ok: c(0x1f, 0x9d, 0x55),
    warn: c(0xb7, 0x79, 0x1f),
    err: c(0xc9, 0x38, 0x38),
};

/// Escala de dimensiones de la interfaz (px lógicos, rejilla de 4 px).
///
/// Todas las medidas de la UI salen de aquí; los paneles no deberían usar
/// números sueltos salvo para el contenido de la maqueta de la página.
pub mod dims {
    // --- Espaciado (múltiplos de 4) ---
    pub const S1: f32 = 4.0;
    pub const S2: f32 = 8.0;
    pub const S3: f32 = 12.0;
    pub const S4: f32 = 16.0;

    // --- Alturas de control ---
    /// Botón dentro de un grupo segmentado (el grupo mide `H_CTRL`).
    pub const H_SEG_BTN: f32 = 28.0;
    /// Relleno interior de un grupo segmentado: `H_SEG_BTN + 2·SEG_PAD = H_CTRL`.
    pub const SEG_PAD: i8 = 2;
    /// Chip, combo, campo de texto, botón de icono en paneles.
    pub const H_CTRL: f32 = 32.0;
    /// Opciones a todo ancho (preajustes, segmentados) y botones de la barra superior.
    pub const H_CTRL_LG: f32 = 36.0;
    /// Filas de lista de una línea y botón principal.
    pub const H_ROW: f32 = 40.0;
    /// Controles de la barra superior (una fila baja, como una barra de título).
    pub const H_BAR: f32 = 28.0;
    /// Filas de lista de dos líneas (título + metadato).
    pub const H_ROW_2: f32 = 48.0;

    // --- Radios ---
    pub const R_XS: u8 = 2;
    pub const R_SM: u8 = 3;
    /// Controles: chips, botones, combos.
    pub const R_MD: u8 = 4;
    /// Filas, grupos segmentados, campos grandes.
    pub const R_LG: u8 = 6;
    /// Contenedores flotantes (menús, barra de presets).
    pub const R_XL: u8 = 8;
    /// Esquinas de los paneles del dock.
    pub const R_PANEL: u8 = 6;

    // --- Tipografía ---
    pub const F_CAPTION: f32 = 12.0;
    pub const F_BODY: f32 = 13.0;
    pub const F_LABEL: f32 = 14.0;
    pub const F_TITLE: f32 = 15.0;
    pub const F_CODE: f32 = 13.0;

    // --- Glifos (Phosphor) ---
    pub const G_SM: f32 = 14.0;
    pub const G_MD: f32 = 16.0;
    pub const G_LG: f32 = 18.0;

    // --- Marco de la ventana ---
    /// Margen exterior de la ventana y separación barra ↔ dock. `TOPBAR_H` es la
    /// altura exterior de la barra (incluye el margen superior de `GUTTER`).
    pub const GUTTER: i8 = 8;
    pub const TOPBAR_H: f32 = 52.0;
    pub const TAB_BAR_H: f32 = 32.0;
    /// Relleno interior del cuerpo de cada panel.
    pub const PANEL_PAD: i8 = 10;
    /// Hueco entre paneles del dock (= separador arrastrable).
    pub const SEPARATOR: f32 = 6.0;
    /// Un separador no puede acercarse a menos de esto del borde de su nodo.
    /// egui_dock aplica este límite en cada fotograma y reescribe la fracción:
    /// si es grande, la ventana estrecha del arranque (≈663 px en mosaico)
    /// deforma los diseños para siempre. 80 px no altera ningún preset ≥ 640 px.
    pub const SEPARATOR_LIMIT: f32 = 80.0;
}

thread_local! {
    static CURRENT: Cell<Palette> = const { Cell::new(DARK) };
}

/// Paleta activa (barata: `Palette` es `Copy`).
pub fn pal() -> Palette {
    CURRENT.with(Cell::get)
}

/// Cambia la paleta activa y reaplica el estilo de egui.
pub fn set_dark(ctx: &egui::Context, dark: bool) {
    CURRENT.with(|p| p.set(if dark { DARK } else { LIGHT }));
    apply_style(ctx);
}

/// Registra Phosphor y fija el estilo inicial.
pub fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Dark);
    apply_style(ctx);
}

fn apply_style(ctx: &egui::Context) {
    let p = pal();
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.visuals = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        let v = &mut style.visuals;
        v.panel_fill = p.bg;
        v.window_fill = p.panel;
        v.extreme_bg_color = p.inset;
        v.faint_bg_color = p.panel;
        v.override_text_color = Some(p.text);
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 70);
        v.selection.stroke = Stroke::new(1.0, p.accent);
        v.window_stroke = Stroke::new(1.0, p.border);
        let radius = CornerRadius::same(dims::R_MD);
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
        ] {
            w.corner_radius = radius;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
        v.widgets.inactive.weak_bg_fill = p.inset;
        v.widgets.inactive.bg_fill = p.inset;
        v.widgets.hovered.weak_bg_fill = p.hover;
        v.widgets.hovered.bg_fill = p.hover;
        v.widgets.active.weak_bg_fill = p.hover;
        style.spacing.item_spacing = egui::vec2(dims::S2, dims::S2);
    });
}

/// Estilo del dock a juego con la paleta (pestañas tipo Grammata).
pub fn dock_style(style: &egui::Style) -> egui_dock::Style {
    let _ = style;
    use egui::Margin;
    use egui_dock::TabInteractionStyle;
    let mut s = egui_dock::Style::from_egui(style);
    let r = CornerRadius { nw: dims::R_LG, ne: dims::R_LG, sw: 0, se: 0 };
    let tab = |bg: Color32, text: Color32| TabInteractionStyle {
        bg_fill: bg,
        text_color: text,
        corner_radius: r,
        outline_color: Color32::TRANSPARENT,
    };
    s.main_surface_border_stroke = Stroke::NONE;
    s.main_surface_border_rounding = CornerRadius::ZERO;
    s.dock_area_padding = None;

    s.tab_bar.height = dims::TAB_BAR_H;
    s.tab_bar.bg_fill = pal().bg;
    s.tab_bar.hline_color = pal().border;
    s.tab_bar.corner_radius = CornerRadius::ZERO;
    s.tab_bar.fill_tab_bar = false;

    s.tab.tab_body.bg_fill = pal().panel;
    s.tab.tab_body.stroke = Stroke::new(1.0, pal().border);
    s.tab.tab_body.corner_radius = CornerRadius { nw: 0, ne: 0, sw: dims::R_PANEL, se: dims::R_PANEL };
    s.tab.tab_body.inner_margin = Margin::same(dims::PANEL_PAD);
    s.tab.hline_below_active_tab_name = false;
    s.tab.minimum_width = None;
    s.tab.spacing = 4.0;
    s.tab.active = tab(pal().panel, pal().text);
    s.tab.focused = tab(pal().panel, pal().accent);
    s.tab.hovered = tab(pal().inset, pal().text);
    s.tab.inactive = tab(Color32::TRANSPARENT, pal().muted);
    s.tab.active_with_kb_focus = tab(pal().panel, pal().text);
    s.tab.focused_with_kb_focus = tab(pal().panel, pal().accent);
    s.tab.inactive_with_kb_focus = tab(Color32::TRANSPARENT, pal().muted);

    s.separator.width = dims::SEPARATOR;
    s.separator.extra = dims::SEPARATOR_LIMIT;
    s.separator.extra_interact_width = 4.0;
    s.separator.color_idle = pal().bg;
    s.separator.color_hovered = pal().accent;
    s.separator.color_dragged = pal().accent;

    s.buttons.close_tab_color = pal().muted;
    s.buttons.close_tab_active_color = pal().text;
    s.buttons.close_tab_bg_fill = pal().hover;
    s.overlay.selection_color = Color32::from_rgba_unmultiplied(0xf2, 0xa1, 0x4e, 60);
    s.overlay.button_color = pal().hover;
    s.overlay.button_border_stroke = Stroke::new(1.0, pal().border);
    s
}
