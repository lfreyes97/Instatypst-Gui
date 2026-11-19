//! Widgets reutilizables: tarjetas, botones de icono, chips y segmentados.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, Response, RichText,
    Sense, Stroke, StrokeKind, Ui,
};

use crate::theme::*;

/// El marco lo pone el dock; esto solo mantiene el ancho completo.
pub fn card<R>(ui: &mut Ui, _fill_height: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    add(ui)
}

pub fn card_title(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).size(dims::F_TITLE).strong());
}

/// Cabecera de panel: botones a la derecha y título recortado en el hueco que
/// queda (no se solapan en paneles estrechos).
pub fn panel_header(ui: &mut Ui, title: &str, buttons: impl FnOnce(&mut Ui)) {
    let size = vec2(ui.available_width(), dims::H_CTRL);
    ui.allocate_ui_with_layout(size, egui::Layout::right_to_left(egui::Align::Center), |ui| {
        buttons(ui);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(title).size(dims::F_TITLE).strong()).truncate());
        });
    });
}

pub fn small_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(dims::F_CAPTION).color(pal().muted));
}

/// Botón cuadrado con icono (hover suave).
pub fn icon_btn(ui: &mut Ui, symbol: &str, tip: &str) -> Response {
    icon_btn_sized(ui, symbol, tip, dims::H_CTRL_LG, dims::G_LG, pal().muted, false)
}

/// Anchura de un texto en una sola línea.
pub fn text_width(ui: &Ui, text: &str, size: f32) -> f32 {
    ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), FontId::proportional(size), Color32::WHITE).size().x)
}

pub fn icon_btn_sized(
    ui: &mut Ui,
    symbol: &str,
    tip: &str,
    side: f32,
    glyph: f32,
    color: Color32,
    filled: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    let hovered = resp.hovered();
    if filled || hovered {
        let fill = if hovered { pal().hover } else { pal().inset };
        ui.painter().rect_filled(rect, CornerRadius::same(dims::R_MD), fill);
    }
    let color = if hovered { pal().text } else { color };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        symbol,
        FontId::proportional(glyph),
        color,
    );
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Lado de la muestra de color dentro de un chip.
pub const CHIP_SWATCH: f32 = 16.0;

/// Ancho de un chip que muestra `label` (con muestra de color o sin ella).
pub fn chip_width(ui: &Ui, label: &str, swatch: bool) -> f32 {
    let sw = if swatch { CHIP_SWATCH + dims::S2 } else { 0.0 };
    dims::S3 * 2.0 + sw + text_width(ui, label, dims::F_BODY)
}

/// Chip seleccionable con muestra de color opcional.
pub fn chip(ui: &mut Ui, label: &str, swatch: Option<Color32>, selected: bool, size: egui::Vec2) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = match (selected, resp.hovered()) {
        (true, _) => pal().accent_bg,
        (false, true) => pal().hover,
        _ => pal().inset,
    };
    let stroke = if selected { Stroke::new(1.0, pal().accent) } else { Stroke::new(1.0, pal().border) };
    let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    p.rect(rect, CornerRadius::same(dims::R_MD), fill, stroke, StrokeKind::Inside);
    let mut x = rect.left() + dims::S3;
    if let Some(color) = swatch {
        let sw = Rect::from_min_size(pos2(x, rect.center().y - CHIP_SWATCH / 2.0), vec2(CHIP_SWATCH, CHIP_SWATCH));
        p.rect_filled(sw, CornerRadius::same(dims::R_XS), color);
        x += CHIP_SWATCH + dims::S2;
        p.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(dims::F_BODY), pal().text);
    } else {
        p.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(dims::F_BODY), pal().text);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Control segmentado de ancho completo.
pub fn segmented(ui: &mut Ui, options: &[&str], current: &mut usize) {
    let gap = ui.spacing().item_spacing.x;
    let w = (ui.available_width() - gap * (options.len() as f32 - 1.0)) / options.len() as f32;
    ui.horizontal(|ui| {
        for (i, name) in options.iter().enumerate() {
            if chip(ui, name, None, *current == i, vec2(w, dims::H_CTRL_LG)).clicked() {
                *current = i;
            }
        }
    });
}

/// Rectángulo proporcional dibujado como icono de relación de aspecto.
pub fn ratio_btn(ui: &mut Ui, w: f32, h: f32, tip: &str, selected: bool, size: egui::Vec2) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = match (selected, resp.hovered()) {
        (true, _) => pal().accent_bg,
        (false, true) => pal().hover,
        _ => pal().inset,
    };
    let stroke = if selected { Stroke::new(1.0, pal().accent) } else { Stroke::new(1.0, pal().border) };
    ui.painter().rect(rect, CornerRadius::same(dims::R_MD), fill, stroke, StrokeKind::Inside);
    let k = 16.0 / w.max(h);
    let glyph = Rect::from_center_size(rect.center(), vec2(w * k, h * k));
    let color = if selected { pal().accent } else { pal().muted };
    ui.painter()
        .rect_stroke(glyph, CornerRadius::same(3), Stroke::new(1.5, color), StrokeKind::Inside);
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Punto de estado con texto.
pub fn status_dot(ui: &mut Ui, color: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        ui.label(RichText::new(text).size(dims::F_CAPTION).color(pal().muted));
    });
}

/// Ancho que ocupa `status_dot` con ese texto.
pub fn status_dot_width(ui: &Ui, text: &str) -> f32 {
    8.0 + ui.spacing().item_spacing.x + text_width(ui, text, dims::F_CAPTION)
}
