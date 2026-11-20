//! Paneles de un documento `.typ` libre: el editor de Typst completo y el visor de páginas.

use egui::{pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Ui};
use egui_phosphor::regular as ic;
use instatypst_engine::Severity;

use crate::layout::{LayoutAction, Tab};
use crate::theme::*;
use crate::typst_editor::TypEditor;
use crate::widgets::*;
use crate::State;

const EDITOR_ID: &str = "typ-doc-edit";

pub fn editor(app: &mut State, ui: &mut Ui) {
    use dims::*;
    let Some(i) = app.active_typ else { return };
    let ctx = ui.ctx().clone();
    let errors: Vec<String> = app
        .typ_preview
        .as_ref()
        .map(|p| p.diags.iter().filter(|d| d.severity == Severity::Error).map(|d| format!("{}  {}", d.location(), d.message)).collect())
        .unwrap_or_default();
    let bad = app.typ_preview.as_ref().map(|p| p.error_lines()).unwrap_or_default();

    ui.horizontal(|ui| {
        let doc = &app.typs[i];
        let title = format!("{}  {}{}", ic::FILE_CODE, doc.name(), if doc.is_modified() { " •" } else { "" });
        chip(ui, &title, None, true, vec2(chip_width(ui, &title, false), H_CTRL));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            match errors.first() {
                None => status_dot(ui, pal().ok, "Compila"),
                Some(e) => status_dot(ui, pal().err, &e.chars().take(56).collect::<String>()),
            };
            if icon_btn_sized(ui, ic::FLOPPY_DISK, "Guardar   Ctrl+S", H_CTRL, G_MD, pal().muted, false).clicked() {
                app.file_action = Some(crate::FileAction::Save);
            }
        });
    });
    ui.add_space(S1);

    // Saltar a una línea (clic en una página): el cursor al inicio y el foco en el editor.
    if let Some(line) = app.typ_goto_line.take() {
        let text = &app.typs[i].text;
        let idx: usize = text.split_inclusive('\n').take(line).map(|l| l.chars().count()).sum();
        let id = Id::new(EDITOR_ID);
        let mut st = egui::TextEdit::load_state(&ctx, id).unwrap_or_default();
        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(idx))));
        st.store(&ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
    }

    let h = if errors.is_empty() { 0.0 } else { 18.0 * errors.len().min(4) as f32 + S2 };
    let alto = (ui.available_height() - h).max(80.0);
    let (root, filename) = (app.typs[i].root(), app.typs[i].filename());
    let ed = app.typ_ed.get_or_insert_with(|| TypEditor::new(&ctx));
    let doc = &mut app.typs[i];
    ui.allocate_ui(vec2(ui.available_width(), alto), |ui| {
        if crate::typst_editor::show(ui, ed, &mut doc.text, EDITOR_ID, &bad, &root, &filename) {
            doc.rev += 1;
        }
    });
    for e in errors.iter().take(4) {
        ui.label(RichText::new(e).size(F_CAPTION).color(pal().err));
    }
}

/// Visor de páginas: una a una con navegación, o todas en columna con scroll.
pub fn preview(app: &mut State, ui: &mut Ui) {
    use dims::*;
    let Some(i) = app.active_typ else { return };
    let area = ui.available_rect_before_wrap();
    let area_resp = ui.allocate_rect(area, Sense::hover());
    const PILL_H: f32 = 28.0;
    let stage = Rect::from_min_max(area.min + vec2(S3, S3), area.max - vec2(S3, PILL_H + S4 + S2));
    let ctx = ui.ctx().clone();
    let ppp = ctx.pixels_per_point();
    // En scroll se compila al ancho del hueco; una a una, para que la página entre entera.
    let target = if app.typ_scroll { stage.width() * ppp } else { stage.size().max_elem().max(64.0) * ppp };
    let pv = app.typ_preview.get_or_insert_with(|| crate::typdoc::TypPreview::new(&ctx));
    pv.tick(&ctx, &app.typs[i], target);
    let n = pv.pages.len();
    app.typ_page = app.typ_page.min(n.saturating_sub(1));
    let stale = pv.stale();

    // Teclado con el ratón sobre la vista: AvPág/RePág mueven una página.
    let mut jump: Option<usize> = app.typ_jump.take();
    if area_resp.hovered() || ctx.memory(|m| m.focused().is_none()) {
        let (next, prev) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, egui::Key::PageDown), i.consume_key(egui::Modifiers::NONE, egui::Key::PageUp)));
        if next && app.typ_page + 1 < n {
            jump = Some(app.typ_page + 1);
            app.typ_page += 1;
        }
        if prev && app.typ_page > 0 {
            jump = Some(app.typ_page - 1);
            app.typ_page -= 1;
        }
    }

    let mut click: Option<(usize, f32, f32)> = None;
    let tint = if pv.failed || stale { Color32::from_white_alpha(120) } else { Color32::WHITE };
    let draw = |ui: &Ui, page: Rect, tex: &egui::TextureHandle| {
        ui.painter().rect_filled(page.translate(vec2(0.0, 6.0)).expand(2.0), CornerRadius::same(8), Color32::from_black_alpha(80));
        ui.painter().image(tex.id(), page, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    };

    if n == 0 {
        ui.painter().text(stage.center(), Align2::CENTER_CENTER, if pv.busy { "Compilando…" } else { "Sin páginas" }, FontId::proportional(F_BODY), pal().muted);
    } else if app.typ_scroll {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(stage));
        let mut visible = app.typ_page;
        egui::ScrollArea::vertical().id_salt("typ-scroll").auto_shrink([false, false]).show(&mut child, |ui| {
            ui.spacing_mut().item_spacing.y = 14.0;
            let widest = pv.pages.iter().map(|p| p.size_pt.0).fold(1.0, f32::max);
            let k = (stage.width() - 8.0) / widest;
            for (idx, p) in pv.pages.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(p.size_pt.0 * k, p.size_pt.1 * k), Sense::click());
                let r = Rect::from_center_size(pos2(stage.center().x, r.center().y), r.size());
                if ui.is_rect_visible(r) {
                    draw(ui, r, &p.tex);
                    if r.contains(stage.center()) || (r.top() <= stage.center().y && r.bottom() >= stage.center().y) {
                        visible = idx;
                    }
                }
                if jump == Some(idx) {
                    ui.scroll_to_rect(r, Some(Align::TOP));
                }
                if resp.clicked() {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        click = Some((idx, (pos.x - r.min.x) / k, (pos.y - r.min.y) / k));
                    }
                }
            }
        });
        if jump.is_none() {
            app.typ_page = visible;
        }
    } else {
        let p = &pv.pages[app.typ_page];
        let k = (stage.width() / p.size_pt.0).min(stage.height() / p.size_pt.1);
        let r = Rect::from_center_size(stage.center(), vec2(p.size_pt.0 * k, p.size_pt.1 * k));
        draw(ui, r, &p.tex);
        let resp = ui.interact(r, Id::new("typ-page-click"), Sense::click());
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                click = Some((app.typ_page, (pos.x - r.min.x) / k, (pos.y - r.min.y) / k));
            }
        }
    }
    // Clic en la página → la línea del código que la produjo.
    if let Some((page, x, y)) = click {
        if let Some(e) = instatypst_engine::line_for_pos(&pv.sync, page, x, y) {
            if e.file.is_none() {
                app.typ_goto_line = Some(e.line.saturating_sub(1));
                app.layout_actions.push(LayoutAction::Focus(Tab::Editor));
            }
        }
    }

    // Barra inferior: ‹ 2 / 5 ›, interruptor de scroll y estado.
    let errors = pv.diags.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = pv.diags.len() - errors;
    let (dot, info) = if pv.failed {
        (pal().err, format!("{errors} error(es)"))
    } else if pv.busy || stale {
        (pal().warn, "Compilando…".to_string())
    } else {
        (pal().ok, format!("{} ms{}", pv.ms, if warnings > 0 { format!(" · {warnings} aviso(s)") } else { String::new() }))
    };
    let nav = format!("{} / {}", if n == 0 { 0 } else { app.typ_page + 1 }, n);
    let w = text_width(ui, &nav, F_CAPTION) + text_width(ui, &info, F_CAPTION) + H_CTRL * 3.0 + S4 * 4.0 + 20.0;
    let pill = Rect::from_center_size(pos2(area.center().x, area.bottom() - S1 - PILL_H / 2.0), vec2(w.min(area.width()), PILL_H));
    ui.painter().rect(pill, CornerRadius::same((PILL_H / 2.0) as u8), pal().panel, Stroke::new(1.0, pal().border), StrokeKind::Inside);
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(pill.shrink2(vec2(S2, 0.0))).layout(Layout::left_to_right(Align::Center)));
    bar.spacing_mut().item_spacing.x = S1;
    let mut go: Option<usize> = None;
    if icon_btn_sized(&mut bar, ic::CARET_LEFT, "Página anterior   RePág", H_CTRL, G_SM, pal().muted, false).clicked() && app.typ_page > 0 {
        go = Some(app.typ_page - 1);
    }
    bar.label(RichText::new(nav).size(F_CAPTION).color(pal().text));
    if icon_btn_sized(&mut bar, ic::CARET_RIGHT, "Página siguiente   AvPág", H_CTRL, G_SM, pal().muted, false).clicked() && app.typ_page + 1 < n {
        go = Some(app.typ_page + 1);
    }
    let tip = if app.typ_scroll { "Una página a la vez" } else { "Todas las páginas con scroll" };
    if icon_btn_sized(&mut bar, ic::ARROW_DOWN, tip, H_CTRL, G_SM, if app.typ_scroll { pal().accent } else { pal().muted }, app.typ_scroll).clicked() {
        app.typ_scroll = !app.typ_scroll;
        app.typ_jump = Some(app.typ_page);
        ctx.request_repaint();
    }
    let (r, _) = bar.allocate_exact_size(vec2(8.0, H_CTRL), Sense::hover());
    bar.painter().circle_filled(r.center(), 4.0, dot);
    bar.label(RichText::new(info).size(F_CAPTION).color(pal().muted));
    if let Some(g) = go {
        app.typ_page = g;
        app.typ_jump = Some(g);
        ctx.request_repaint();
    }
}
