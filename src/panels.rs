//! Contenido de cada zona de la ventana.

use egui::text::LayoutJob;
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect,
    RichText, Sense, Stroke, StrokeKind, Ui,
};
use egui_phosphor::regular as ic;

use crate::theme::*;
use crate::widgets::*;
use crate::layout::{LayoutAction, Preset, Tab};
use crate::{highlight, FileAction, State};

/// Línea vertical fina entre grupos de la barra, con aire a cada lado.
fn bar_sep(ui: &mut Ui) {
    ui.add_space(dims::S2);
    let (r, _) = ui.allocate_exact_size(vec2(1.0, dims::H_BAR - 8.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::ZERO, pal().border);
    ui.add_space(dims::S2);
}

/// Barra superior: una sola fila baja, tipo barra de título de editor.
/// Izquierda: documento y pieza activa. Derecha: diagnóstico, deshacer,
/// disposiciones (solo iconos), paneles, tema y Exportar.
pub fn top_bar(app: &mut State, ui: &mut Ui) {
    use dims::*;
    ui.spacing_mut().item_spacing.x = S2;
    ui.horizontal_centered(|ui| {
        // --- Izquierda: documento ---
        let file = ui.add(
            egui::Button::new(RichText::new(format!("{}  {}", ic::FILE_TEXT, app.active_typ.map_or_else(|| app.project.file_name(), |i| app.typs[i].name()))).size(F_BODY).color(pal().text))
                .fill(Color32::TRANSPARENT)
                .min_size(vec2(0.0, H_BAR)),
        );
        egui::Popup::menu(&file).show(|ui| {
            ui.set_min_width(180.0);
            for (sym, label, action) in [
                (ic::FOLDER_OPEN, "Abrir…   Ctrl+O", FileAction::Open),
                (ic::FILE_PLUS, "Nuevo Typst", FileAction::NewTyp),
                (ic::FLOPPY_DISK, "Guardar   Ctrl+S", FileAction::Save),
                (ic::COPY, "Guardar como…", FileAction::SaveAs),
            ] {
                if ui.button(format!("{sym}  {label}")).clicked() {
                    app.file_action = Some(action);
                    ui.close();
                }
            }
        });
        if app.project.is_modified() {
            let (r, resp) = ui.allocate_exact_size(vec2(8.0, H_BAR), Sense::hover());
            ui.painter().circle_filled(r.center(), 3.0, pal().accent);
            resp.on_hover_text("Cambios sin guardar");
        }

        ui.add_space(S2);
        bar_sep(ui);
        // Pieza activa: el orden de las piezas es la serie.
        if icon_btn_sized(ui, ic::CARET_LEFT, "Pieza anterior", H_BAR, G_SM, pal().muted, false).clicked() && app.piece > 0 {
            app.piece -= 1;
        }
        ui.label(RichText::new(if app.project.doc.posts.is_empty() { "Sin piezas".to_string() } else { format!("Pieza {} de {}", app.piece + 1, app.project.doc.posts.len()) }).size(F_BODY).color(pal().muted));
        if icon_btn_sized(ui, ic::CARET_RIGHT, "Pieza siguiente", H_BAR, G_SM, pal().muted, false).clicked() && app.piece + 1 < app.project.doc.posts.len() {
            app.piece += 1;
        }

        // --- Derecha (de derecha a izquierda) ---
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = S2;
            // Exportar: la única acción con color de acento.
            let w = text_width(ui, "Exportar", F_BODY) + G_MD + S2 + S4 * 3.0;
            let (r, resp) = ui.allocate_exact_size(vec2(w, H_BAR), Sense::click());
            let fill = if resp.hovered() { Color32::from_rgb(0xff, 0xb4, 0x6a) } else { pal().accent };
            ui.painter().rect_filled(r, CornerRadius::same(R_MD), fill);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, format!("{}  Exportar", ic::EXPORT), FontId::proportional(F_BODY), pal().on_accent);
            resp.on_hover_cursor(egui::CursorIcon::PointingHand);

            bar_sep(ui);
            let (sym, tip) = if app.dark { (ic::SUN, "Modo claro") } else { (ic::MOON, "Modo oscuro") };
            if icon_btn_sized(ui, sym, tip, H_BAR, G_MD, pal().muted, false).clicked() {
                app.dark = !app.dark;
                crate::theme::set_dark(ui.ctx(), app.dark);
            }
            let menu = icon_btn_sized(ui, ic::LAYOUT, "Paneles", H_BAR, G_MD, pal().muted, false);
            if menu.clicked() {
                app.panels_menu = !app.panels_menu;
            }
            if app.panels_menu {
                panels_menu(app, ui.ctx(), &menu);
            }

            // Disposiciones: solo iconos, la descripción va en el tooltip.
            Frame::new()
                .fill(pal().panel)
                .stroke(Stroke::new(1.0, pal().border))
                .corner_radius(CornerRadius::same(R_LG))
                .inner_margin(Margin::same(2))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for preset in Preset::ALL.into_iter().rev() {
                        let (r, resp) = ui.allocate_exact_size(vec2(H_BAR + 2.0, H_BAR - 4.0), Sense::click());
                        let active = app.layout_preset == preset;
                        if active || resp.hovered() {
                            let fill = if active { pal().hover } else { pal().inset };
                            ui.painter().rect_filled(r, CornerRadius::same(R_MD), fill);
                        }
                        let col = if active { pal().accent } else { pal().muted };
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, preset.icon(), FontId::proportional(G_MD), col);
                        let resp = resp
                            .on_hover_text(format!("{} — {}", preset.label(), preset.description()))
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            app.layout_actions.push(LayoutAction::Preset(preset));
                        }
                    }
                });

            bar_sep(ui);
            icon_btn_sized(ui, ic::ARROW_U_UP_RIGHT, "Rehacer", H_BAR, G_MD, pal().muted, false);
            icon_btn_sized(ui, ic::ARROW_U_UP_LEFT, "Deshacer", H_BAR, G_MD, pal().muted, false);

            // Diagnóstico: punto con recuento; abrirá el panel de incidencias.
            bar_sep(ui);
            let errors = app.issues().iter().filter(|i| i.error).count();
            let warnings = app.issues().len() - errors;
            let (color, text) = match (errors, warnings) {
                (0, 0) => (pal().ok, "0".to_owned()),
                (0, w) => (pal().warn, w.to_string()),
                (e, w) => (pal().err, format!("{e} · {w}")),
            };
            let w = 8.0 + S1 + text_width(ui, &text, F_CAPTION) + S2 * 2.0;
            let (r, resp) = ui.allocate_exact_size(vec2(w, H_BAR), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, CornerRadius::same(R_MD), pal().hover);
            }
            ui.painter().circle_filled(pos2(r.left() + S2 + 4.0, r.center().y), 4.0, color);
            ui.painter().text(pos2(r.left() + S2 + 8.0 + S1, r.center().y), Align2::LEFT_CENTER, &text, FontId::proportional(F_CAPTION), pal().muted);
            let resp = resp.on_hover_text(format!("{errors} errores · {warnings} avisos")).on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                app.layout_actions.push(LayoutAction::Focus(Tab::Issues));
            }
        });
    });
}

/// Menú desplegable con un interruptor por panel.
fn panels_menu(app: &mut State, ctx: &egui::Context, anchor: &egui::Response) {
    const MENU_W: f32 = 232.0;
    let pos = pos2(anchor.rect.right() - MENU_W - dims::S2 * 2.0, anchor.rect.bottom() + dims::S2);
    let area = egui::Area::new(egui::Id::new("panels-menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            Frame::new()
                .fill(pal().panel)
                .stroke(Stroke::new(1.0, pal().border))
                .corner_radius(CornerRadius::same(dims::R_XL))
                .inner_margin(Margin::same(dims::S2 as i8))
                .show(ui, |ui| {
                    ui.set_width(MENU_W);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_space(dims::S1);
                    ui.label(RichText::new("  Paneles").size(dims::F_CAPTION).color(pal().muted));
                    ui.add_space(dims::S1);
                    for tab in Tab::ALL {
                        let open = app.open.contains(&tab);
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), dims::H_CTRL_LG), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(dims::R_MD), pal().inset);
                        }
                        let col = if open { pal().accent } else { pal().muted };
                        ui.painter().text(pos2(r.left() + dims::S4, r.center().y), Align2::CENTER_CENTER, tab.icon(), FontId::proportional(dims::G_MD), col);
                        ui.painter().text(pos2(r.left() + dims::S4 * 2.0, r.center().y), Align2::LEFT_CENTER, tab.name(), FontId::proportional(dims::F_BODY), pal().text);
                        if open {
                            ui.painter().text(pos2(r.right() - dims::S4, r.center().y), Align2::CENTER_CENTER, ic::CHECK, FontId::proportional(dims::G_SM), pal().accent);
                        }
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            app.layout_actions.push(LayoutAction::Toggle(tab));
                        }
                    }
                });
        });
    if area.response.clicked_elsewhere() && !anchor.clicked() {
        app.panels_menu = false;
    }
}

pub fn source(app: &mut State, ui: &mut Ui) {
    card(ui, true, |ui| {
        let n = app.project.doc.posts.len();
        let has = n > 0;
        let mut add = false;
        let mut dup = false;
        let mut mv = 0isize;
        let mut del = false;
        panel_header(ui, "Piezas", |ui| {
            let b = |ui: &mut Ui, sym: &str, tip: &str, on: bool| {
                ui.add_enabled_ui(on, |ui| icon_btn_sized(ui, sym, tip, dims::H_CTRL, dims::G_MD, pal().muted, true)).inner.clicked()
            };
            add = b(ui, ic::PLUS, "Nueva pieza", true);
            dup = b(ui, ic::COPY, "Duplicar", has);
            if b(ui, ic::ARROW_DOWN, "Bajar (el orden es la serie)", has && app.piece + 1 < n) {
                mv = 1;
            }
            if b(ui, ic::ARROW_UP, "Subir", has && app.piece > 0) {
                mv = -1;
            }
            del = b(ui, ic::TRASH, "Borrar", has);
            if b(ui, ic::MAGNIFYING_GLASS, "Buscar", true) {
                app.search_open = !app.search_open;
            }
        });
        if add {
            app.piece = app.project.add_piece();
        } else if dup {
            app.piece = app.project.duplicate(app.piece);
        } else if mv != 0 {
            app.piece = app.project.move_piece(app.piece, mv);
        } else if del {
            app.project.delete(app.piece);
            app.piece = app.piece.saturating_sub(1);
        }
        if app.search_open {
            ui.add(
                egui::TextEdit::singleline(&mut app.search)
                    .hint_text("Buscar piezas…")
                    .desired_width(f32::INFINITY)
                    .min_size(vec2(0.0, dims::H_CTRL))
                    .margin(Margin::symmetric(10, 8)),
            );
        }
        ui.add_space(dims::S1);

        if app.project.doc.posts.is_empty() {
            ui.add_space(dims::S4);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Este documento no tiene piezas").size(dims::F_BODY).color(pal().muted));
                ui.label(RichText::new("Crea una con +, o abre otro documento (Ctrl+O)").size(dims::F_CAPTION).color(pal().muted));
            });
            return;
        }
        let query = app.search.to_lowercase();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = dims::S1;
            for i in 0..app.project.doc.posts.len() {
                let title = app.project.label(i);
                if !query.is_empty() && !title.to_lowercase().contains(&query) {
                    continue;
                }
                let meta = app.project.meta(i);
                let num = format!("{}", i + 1);
                if block_row(ui, &num, &title, &meta, app.piece == i).clicked() {
                    app.piece = i;
                }
            }
        });
    });
}

fn truncated(ui: &Ui, text: &str, font: FontId, color: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::simple_singleline(text.into(), font, color);
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    ui.fonts_mut(|f| f.layout_job(job))
}

/// Columnas compartidas por cabeceras de sección y bloques del panel Contenido:
/// el icono del bloque queda bajo la miniatura de la sección y los textos alineados.
const ROW_INDENT: f32 = 8.0;
const ROW_ICON_X: f32 = 36.0;
const ROW_TEXT_X: f32 = 60.0;

pub(crate) fn block_row(ui: &mut Ui, symbol: &str, title: &str, meta: &str, selected: bool) -> egui::Response {
    let width = ui.available_width() - ROW_INDENT;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, dims::H_ROW_2), Sense::click());
    let rect = rect.translate(vec2(ROW_INDENT, 0.0));
    let p = ui.painter();
    if selected {
        p.rect(rect, CornerRadius::same(dims::R_LG), pal().accent_bg, Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xf2, 0xa1, 0x4e, 60)), StrokeKind::Inside);
    } else if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(dims::R_LG), pal().inset);
    }
    let dot = pos2(rect.left() + ROW_ICON_X - ROW_INDENT, rect.center().y);
    p.circle_filled(dot, 14.0, if selected { pal().accent } else { pal().hover });
    p.text(dot, Align2::CENTER_CENTER, symbol, FontId::proportional(dims::G_SM), if selected { pal().on_accent } else { pal().muted });
    let text_x = rect.left() + ROW_TEXT_X - ROW_INDENT;
    // Solo la fila seleccionada reserva sitio para el ojo de la derecha.
    let trailing = if selected { dims::S4 * 2.0 } else { dims::S2 };
    let text_w = (rect.right() - trailing - text_x).max(0.0);
    let g = truncated(ui, title, FontId::proportional(dims::F_BODY), pal().text, text_w);
    let m = truncated(ui, meta, FontId::proportional(dims::F_CAPTION), pal().muted, text_w);
    let p = ui.painter();
    p.galley(pos2(text_x, rect.top() + 7.0), g, pal().text);
    p.galley(pos2(text_x, rect.top() + 26.0), m, pal().muted);
    if selected {
        p.text(pos2(rect.right() - dims::S4, rect.center().y), Align2::CENTER_CENTER, ic::EYE, FontId::proportional(dims::G_MD), pal().muted);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn preview(app: &mut State, ui: &mut Ui) {
    if app.active_typ.is_some() {
        return crate::typpanels::preview(app, ui);
    }
    let area = ui.available_rect_before_wrap();
    let area_resp = ui.allocate_rect(area, Sense::click());
    if app.project.doc.posts.is_empty() {
        ui.add_space(dims::S4);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Sin pieza que mostrar").size(dims::F_BODY).color(pal().muted));
        });
        return;
    }
    const PILL_H: f32 = 24.0;
    let stage = Rect::from_min_max(area.min + vec2(dims::S3, dims::S3), area.max - vec2(dims::S3, PILL_H + dims::S4 + dims::S2));
    // Se compila al tamaño en que se va a ver (píxeles físicos del lado mayor).
    let target = stage.size().max_elem().max(64.0) * ui.ctx().pixels_per_point();
    let ctx = ui.ctx().clone();
    {
        let pv = app.preview.get_or_insert_with(|| crate::preview::PreviewState::new(&ctx));
        pv.tick(&ctx, &app.project, app.piece, target);
    }
    let mut pick_at: Option<(f32, f32)> = None;
    let pv = app.preview.as_ref().expect("vista previa");

    // La imagen, ajustada al hueco, con la proporción de la página compilada.
    let stale = pv.stale();
    if let Some(tex) = &pv.texture {
        let (w, h) = pv.size_pt;
        let k = (stage.width() / w).min(stage.height() / h);
        let page = Rect::from_center_size(stage.center(), vec2(w * k, h * k));
        let painter = ui.painter();
        painter.rect_filled(page.translate(vec2(0.0, 8.0)).expand(2.0), CornerRadius::same(10), Color32::from_black_alpha(90));
        let tint = if pv.error.is_some() || stale { Color32::from_white_alpha(120) } else { Color32::WHITE };
        painter.image(tex.id(), page, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    }
    // Modo debug: cajas, márgenes e inspector sobre la lámina.
    if app.debug {
        {
            if pv.texture.is_some() {
                let (w, h) = pv.size_pt;
                let k = (stage.width() / w).min(stage.height() / h);
                let page = Rect::from_center_size(stage.center(), vec2(w * k, h * k));
                let ptr = ui.input(|i| i.pointer.hover_pos()).filter(|p| page.contains(*p) && area_resp.hovered());
                let to_pt = |p: egui::Pos2| ((p.x - page.min.x) / k, (p.y - page.min.y) / k);
                let hovered = ptr.and_then(|p| { let (x, y) = to_pt(p); crate::debug::hit(&pv.boxes, x, y) });
                let screen = |r: [f32; 4]| Rect::from_min_size(page.min + vec2(r[0], r[1]) * k, vec2(r[2], r[3]) * k);
                let painter = ui.painter().with_clip_rect(page.expand(2.0));
                for (i, b) in pv.boxes.iter().enumerate() {
                    let color = match b.kind {
                        instatypst_engine::BoxKind::Text => pal().accent,
                        instatypst_engine::BoxKind::Shape => Color32::from_rgb(0x4d, 0xc9, 0xe0),
                        instatypst_engine::BoxKind::Image => Color32::from_rgb(0xe0, 0x4d, 0xc9),
                    };
                    let r = screen(b.rect);
                    if Some(i) == hovered {
                        painter.rect_filled(r, CornerRadius::ZERO, color.gamma_multiply(0.25));
                    }
                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(if Some(i) == hovered { 2.0 } else { 1.0 }, color.gamma_multiply(0.9)), StrokeKind::Inside);
                }
                // Márgenes: la caja del contenido y su distancia a los bordes de la página.
                if let Some((caja, (l, t, r, b))) = crate::debug::content_margins(&pv.boxes, pv.size_pt) {
                    let c = screen(caja);
                    let guia = Stroke::new(1.0, Color32::from_rgb(0xff, 0x4d, 0x6d));
                    painter.rect_stroke(c, CornerRadius::ZERO, guia, StrokeKind::Outside);
                    let etiqueta = |pos: egui::Pos2, txt: String, al: Align2| {
                        painter.text(pos, al, txt, FontId::monospace(dims::F_CAPTION), Color32::from_rgb(0xff, 0x4d, 0x6d));
                    };
                    etiqueta(pos2(page.min.x + 3.0, c.center().y), format!("{l:.0}"), Align2::LEFT_CENTER);
                    etiqueta(pos2(c.center().x, page.min.y + 2.0), format!("{t:.0}"), Align2::CENTER_TOP);
                    etiqueta(pos2(page.max.x - 3.0, c.center().y), format!("{r:.0}"), Align2::RIGHT_CENTER);
                    etiqueta(pos2(c.center().x, page.max.y - 2.0), format!("{b:.0}"), Align2::CENTER_BOTTOM);
                }
                // Inspector: lo que es la caja bajo el ratón.
                if let (Some(i), Some(p)) = (hovered, ptr) {
                    let b = &pv.boxes[i];
                    let spec = app.project.doc.merged(app.piece);
                    let mut lineas: Vec<String> = Vec::new();
                    match b.kind {
                        instatypst_engine::BoxKind::Text => {
                            let t = b.text.clone().unwrap_or_default();
                            lineas.push(format!("“{}”", t.chars().take(38).collect::<String>()));
                            lineas.push(format!("{} · {:.0} pt", b.font.clone().unwrap_or_default(), b.size.unwrap_or(0.0)));
                            if let Some(f) = b.fill {
                                lineas.push(format!("color #{:02x}{:02x}{:02x}", f[0], f[1], f[2]));
                            }
                            if let Some(c) = crate::debug::text_contrast(&pv.boxes, i, pv.bg) {
                                let (min, mark) = if b.size.unwrap_or(0.0) >= 24.0 { (3.0, "3:1") } else { (4.5, "4,5:1") };
                                lineas.push(format!("contraste {c:.1}:1 {} (mín. {mark})", if c >= min { "✓" } else { "⚠" }));
                            }
                            match crate::debug::block_for_text(&crate::debug::blocks_of(&spec), &t) {
                                Some(n) => lineas.push(format!("bloque: content[{n}]")),
                                None => lineas.push("bloque: ninguno (lo genera el preset)".into()),
                            }
                        }
                        instatypst_engine::BoxKind::Shape => lineas.push("forma".into()),
                        instatypst_engine::BoxKind::Image => lineas.push("imagen".into()),
                    }
                    lineas.push(format!("x {:.0}  y {:.0}  {:.0}×{:.0} pt", b.rect[0], b.rect[1], b.rect[2], b.rect[3]));
                    let txt = lineas.join("\n");
                    let g = ui.painter().layout(txt, FontId::proportional(dims::F_CAPTION), pal().text, 260.0);
                    let size = g.size() + vec2(dims::S3, dims::S2);
                    let mut pos = p + vec2(14.0, 14.0);
                    if pos.x + size.x > area.right() { pos.x = p.x - size.x - 14.0; }
                    if pos.y + size.y > area.bottom() { pos.y = p.y - size.y - 14.0; }
                    let r = Rect::from_min_size(pos, size);
                    let top = ui.painter().clone();
                    top.rect(r, CornerRadius::same(dims::R_LG), pal().panel, Stroke::new(1.0, pal().border), StrokeKind::Inside);
                    top.galley(r.min + vec2(dims::S2, dims::S1 + 2.0), g, pal().text);
                }
                // Clic: de la caja al bloque del contrato.
                if area_resp.clicked() {
                    if let Some(p) = ptr {
                        pick_at = Some(to_pt(p));
                    }
                }
            }
        }
    }
    // Un error: el mensaje sobre la lámina anterior, que queda atenuada.
    if let Some(e) = &pv.error {
        let msg = format!("{}  {e}", ic::WARNING_CIRCLE);
        let g = ui.painter().layout(msg, FontId::proportional(dims::F_BODY), pal().err, (stage.width() - dims::S4 * 2.0).max(40.0));
        let r = Rect::from_min_size(stage.left_top() + vec2(dims::S2, dims::S2), g.size() + vec2(dims::S4, dims::S3));
        ui.painter().rect(r, CornerRadius::same(dims::R_LG), pal().panel, Stroke::new(1.0, pal().err), StrokeKind::Inside);
        ui.painter().galley(r.min + vec2(dims::S2, dims::S2), g, pal().err);
    }

    // Interruptor del modo debug.
    let btn = Rect::from_min_size(pos2(area.right() - dims::H_CTRL - dims::S2, area.top() + dims::S2), vec2(dims::H_CTRL, dims::H_CTRL));
    let mut ui_btn = ui.new_child(egui::UiBuilder::new().max_rect(btn));
    if icon_btn_sized(&mut ui_btn, ic::BOUNDING_BOX, if app.debug { "Salir del modo debug" } else { "Modo debug: cajas, medidas e inspector" }, dims::H_CTRL, dims::G_MD, if app.debug { pal().accent } else { pal().muted }, app.debug).clicked() {
        app.debug = !app.debug;
        app.block_sel = None;
    }

    // Estado: compilando, error, desactualizada o lista (con el tiempo).
    let (dot, info) = if let Some(e) = &pv.error {
        (pal().err, format!("Error · {}", e.lines().next().unwrap_or("").chars().take(60).collect::<String>()))
    } else if pv.busy || stale {
        (pal().warn, "Compilando…".to_string())
    } else {
        let avisos = if pv.warnings.is_empty() { String::new() } else { format!(" · {} aviso(s)", pv.warnings.len()) };
        let bloque = match (app.debug, app.block_sel) {
            (true, Some(b)) => format!(" · bloque content[{b}]"),
            _ => String::new(),
        };
        (pal().ok, format!("{:.0}×{:.0} px · {} ms{avisos}{bloque}", pv.size_pt.0, pv.size_pt.1, pv.ms))
    };
    let pill_w = (text_width(ui, &info, dims::F_CAPTION) + dims::S4 * 2.0 + 14.0).min(area.width());
    let pill = Rect::from_center_size(pos2(area.center().x, area.bottom() - dims::S1 - PILL_H / 2.0), vec2(pill_w, PILL_H));
    let painter = ui.painter();
    painter.rect(pill, CornerRadius::same((PILL_H / 2.0) as u8), pal().panel, Stroke::new(1.0, pal().border), StrokeKind::Inside);
    painter.circle_filled(pos2(pill.left() + dims::S3, pill.center().y), 4.0, dot);
    painter.with_clip_rect(pill).text(pos2(pill.left() + dims::S3 + 10.0, pill.center().y), Align2::LEFT_CENTER, info, FontId::proportional(dims::F_CAPTION), pal().muted);
    if let Some((x, y)) = pick_at {
        if app.debug_pick(x, y).is_some() {
            app.layout_actions.push(LayoutAction::Focus(Tab::Editor));
        }
    }
}

pub fn editor(app: &mut State, ui: &mut Ui) {
    use dims::*;
    if app.active_typ.is_some() {
        return crate::typpanels::editor(app, ui);
    }
    if app.project.doc.posts.is_empty() {
        ui.add_space(S4);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No hay ninguna pieza que editar").size(F_BODY).color(pal().muted));
            ui.label(RichText::new("Abre un documento (Ctrl+O) o crea una pieza en el panel Piezas").size(F_CAPTION).color(pal().muted));
        });
        return;
    }
    // Recarga los textos cuando cambia la pieza activa o las piezas se tocaron por fuera.
    let key = (app.piece, app.project.generation);
    if app.editor_key != Some(key) {
        app.editor_text = app.project.piece_text(app.piece);
        app.editor_key = Some(key);
        app.editor_error = None;
    }
    if app.typst_key != Some(key) {
        match crate::blocks::typst_of(&app.project.doc.posts[app.piece]) {
            Ok(t) => {
                app.typst_text = t;
                app.typst_issues.clear();
            }
            Err(e) => {
                app.typst_text.clear();
                app.typst_issues = vec![instatypst_piece::model::Issue { path: String::new(), code: "typst_no_cabe", message: e }];
            }
        }
        app.typst_key = Some(key);
    }
    // Un clic en la lámina fija el bloque; el salto de línea solo vale en el modo «Pieza».
    let goto = app.editor_goto.take();

    // Cabecera: la pieza activa, el modo y si lo escrito es válido.
    ui.horizontal(|ui| {
        let title = format!("{}  {}", ic::FILE_CODE, app.project.label(app.piece));
        chip(ui, &title, None, true, vec2(chip_width(ui, &title, false), H_CTRL));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| match app.editor_mode {
            0 => match &app.blocks_note {
                Some(_) => status_dot(ui, pal().warn, "Pasó a bloques"),
                None => status_dot(ui, pal().ok, "Bloques"),
            },
            1 => match app.typst_issues.first() {
                None => status_dot(ui, pal().ok, "Válido"),
                Some(i) => status_dot(ui, pal().err, &format!("{} {}", i.path.trim_start_matches('@'), i.message.lines().next().unwrap_or("")).chars().take(48).collect::<String>()),
            },
            _ => match &app.editor_error {
                None => status_dot(ui, pal().ok, "Válido"),
                Some(e) => status_dot(ui, pal().err, e.lines().next().unwrap_or("error")),
            },
        });
    });
    ui.add_space(S1);
    let mut mode = app.editor_mode;
    segmented(ui, &["Bloques", "Typst", "Pieza"], &mut mode);
    if mode != app.editor_mode {
        app.editor_mode = mode;
        app.blocks_note = None;
    }
    ui.add_space(S1);

    match app.editor_mode {
        0 => crate::blocks::ui(app, ui),
        1 => {
            let no_cabe = app.typst_issues.first().is_some_and(|i| i.code == "typst_no_cabe");
            if !no_cabe {
                ui.horizontal(|ui| {
                    if ui.button(format!("{}  Separar a .typ", ic::FILE_CODE)).on_hover_text("Copia este Typst a un documento libre: ahí puedes salirte del contrato").clicked() {
                        app.new_typ(app.typst_text.clone());
                    }
                });
            }
            if no_cabe {
                ui.add_space(S2);
                ui.label(RichText::new(format!("{}  {}", ic::WARNING, app.typst_issues[0].message)).size(F_BODY).color(pal().warn));
                ui.label(RichText::new("Esta pieza solo se puede editar como bloques o como tabla.").size(F_CAPTION).color(pal().muted));
            } else {
                // Los errores del Typst, con su línea y columna, bajo el editor.
                let h = if app.typst_issues.is_empty() { 0.0 } else { 18.0 * app.typst_issues.len().min(4) as f32 + S2 };
                let alto = (ui.available_height() - h).max(80.0);
                ui.allocate_ui(vec2(ui.available_width(), alto), |ui| {
                    let root = app.project.path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf()).unwrap_or_else(std::env::temp_dir);
                    let ed = app.typ_ed.get_or_insert_with(|| crate::typst_editor::TypEditor::new(ui.ctx()));
                    if crate::typst_editor::show(ui, ed, &mut app.typst_text, "typst-text-edit", &Default::default(), &root, "preview.typ") {
                        app.typst_issues = crate::blocks::apply_typst(&mut app.project, app.piece, &app.typst_text).err().unwrap_or_default();
                    }
                });
                for i in app.typst_issues.iter().take(4) {
                    ui.label(RichText::new(format!("{}  {}", i.path.trim_start_matches('@'), i.message)).size(F_CAPTION).color(pal().err));
                }
            }
        }
        _ => {
            // Líneas del texto a las que apuntan las incidencias de esta pieza.
            let bad: std::collections::HashSet<usize> = app
                .issues()
                .iter()
                .filter(|i| i.error && !i.path.is_empty() && i.piece == Some(app.piece))
                .map(|i| crate::project::line_of(&app.editor_text, &i.path))
                .collect();
            let sel_line = app.block_sel.map(|b| crate::project::line_of(&app.editor_text, &format!("post[{}].content[{}]", app.piece, b)));
            if code_editor(ui, &mut app.editor_text, &bad, sel_line, EDITOR_ID, false) {
                app.editor_error = app.project.set_piece_text(app.piece, &app.editor_text).err();
            }
            // Saltar a una línea (clic en la lámina): el cursor al inicio de la línea y el foco en el editor.
            if let Some(line) = goto {
                let idx: usize = app.editor_text.split_inclusive('\n').take(line).map(|l| l.chars().count()).sum();
                let ctx = ui.ctx().clone();
                let id = egui::Id::new(EDITOR_ID);
                let mut st = egui::TextEdit::load_state(&ctx, id).unwrap_or_default();
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(idx))));
                st.store(&ctx, id);
                ctx.memory_mut(|m| m.request_focus(id));
            }
        }
    }
}

const EDITOR_ID: &str = "piece-text-edit";

/// Incidencias del contrato: clic en una lleva a su pieza y al editor.
pub fn issues(app: &mut State, ui: &mut Ui) {
    panel_header(ui, "Incidencias", |_| {});
    let list: Vec<crate::checker::Finding> = app.issues().to_vec();
    let al_dia = app.checker.as_ref().is_some_and(|c| c.up_to_date(&app.project));
    if list.is_empty() {
        ui.add_space(dims::S4);
        ui.vertical_centered(|ui| {
            let (txt, col) = if al_dia { (format!("{}  Sin incidencias", ic::CHECK_CIRCLE), pal().ok) } else { ("Comprobando…".to_string(), pal().muted) };
            ui.label(RichText::new(txt).size(dims::F_BODY).color(col));
            ui.label(RichText::new("Contrato, firmas de los estilos y avisos de contraste").size(dims::F_CAPTION).color(pal().muted));
        });
        return;
    }
    let mut goto: Option<usize> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = dims::S1;
        for (n, i) in list.iter().enumerate() {
            let donde = match i.piece {
                Some(p) if p < app.project.doc.posts.len() => {
                    if i.path.is_empty() { app.project.label(p) } else { format!("{} · {}", app.project.label(p), i.path) }
                }
                _ => if i.path.is_empty() { "documento".to_string() } else { i.path.clone() },
            };
            let title = format!("{}  {}", i.code, i.message);
            let icon = if i.error { ic::WARNING_CIRCLE } else { ic::WARNING };
            if block_row(ui, icon, &title, &donde, app.issue_sel == Some(n)).clicked() {
                app.issue_sel = Some(n);
                goto = Some(n);
            }
        }
    });
    if let Some(n) = goto {
        if let Some(p) = list[n].piece.filter(|p| *p < app.project.doc.posts.len()) {
            app.piece = p;
            app.layout_actions.push(LayoutAction::Focus(Tab::Editor));
        }
    }
}

/// Panel de archivos: por ahora, el documento abierto.
pub fn files(app: &mut State, ui: &mut Ui) {
    panel_header(ui, "Archivos", |ui| {
        if icon_btn_sized(ui, ic::FOLDER_OPEN, "Abrir documento", dims::H_CTRL, dims::G_MD, pal().muted, true).clicked() {
            app.file_action = Some(FileAction::Open);
        }
        if icon_btn_sized(ui, ic::FILE_PLUS, "Nuevo documento Typst", dims::H_CTRL, dims::G_MD, pal().muted, true).clicked() {
            app.new_typ(String::new());
        }
    });
    ui.spacing_mut().item_spacing.y = dims::S1;
    let name = app.project.file_name();
    let dir = app.project.path.as_ref().and_then(|p| p.parent()).map(|d| d.display().to_string()).unwrap_or_else(|| "Sin guardar".into());
    if block_row(ui, ic::FILE_TEXT, &name, &dir, app.active_typ.is_none()).clicked() {
        app.active_typ = None;
    }
    // Documentos Typst libres.
    let mut close: Option<usize> = None;
    for i in 0..app.typs.len() {
        let d = &app.typs[i];
        let label = format!("{}{}", d.name(), if d.is_modified() { " •" } else { "" });
        let meta = d.path.as_ref().and_then(|p| p.parent()).map(|p| p.display().to_string()).unwrap_or_else(|| "Sin guardar".into());
        let r = block_row(ui, ic::FILE_CODE, &label, &meta, app.active_typ == Some(i));
        if r.clicked() {
            app.active_typ = Some(i);
        }
        r.context_menu(|ui| {
            if ui.button("Cerrar").clicked() {
                close = Some(i);
                ui.close();
            }
        });
    }
    if let Some(i) = close {
        app.typs.remove(i);
        app.active_typ = match app.active_typ {
            Some(a) if a == i => None,
            Some(a) if a > i => Some(a - 1),
            other => other,
        };
    }
    if let Some(msg) = &app.status {
        ui.add_space(dims::S2);
        ui.label(RichText::new(msg).size(dims::F_CAPTION).color(pal().err));
    }
}

/// Editor de texto con números de línea; `true` si el texto cambió este fotograma.
fn code_editor(ui: &mut Ui, text: &mut String, bad: &std::collections::HashSet<usize>, sel: Option<usize>, id: &str, typst: bool) -> bool {
    let mut changed = false;
    Frame::new()
        .fill(pal().inset)
        .stroke(Stroke::new(1.0, pal().border))
        .corner_radius(CornerRadius::same(dims::R_LG))
        .inner_margin(Margin::symmetric(dims::S3 as i8, dims::S2 as i8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), ui.available_height()));
            let font = FontId::monospace(dims::F_CODE);
            let lines = text.split('\n').count().max(1);
            let mut gutter = egui::text::LayoutJob::default();
            for n in 0..lines {
                let (col, txt) = if sel == Some(n) { (pal().accent, format!("{}\n", n + 1)) } else if bad.contains(&n) { (pal().err, format!("{}\n", n + 1)) } else { (pal().muted.gamma_multiply(0.6), format!("{}\n", n + 1)) };
                gutter.append(txt.trim_end_matches('\n'), 0.0, egui::text::TextFormat::simple(font.clone(), col));
                if n + 1 < lines {
                    gutter.append("\n", 0.0, egui::text::TextFormat::simple(font.clone(), col));
                }
            }
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = dims::S3;
                    ui.add(egui::Label::new(gutter).selectable(false));
                    let font_l = font.clone();
                    let mut layouter = move |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                        let job = if typst { highlight::layout(text.as_str(), font_l.clone()) } else { highlight::layout_toml(text.as_str(), font_l.clone()) };
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    changed = ui
                        .add(
                            egui::TextEdit::multiline(text)
                                .id(egui::Id::new(id))
                                .font(font)
                                .code_editor()
                                .frame(Frame::NONE)
                                .desired_width(f32::INFINITY)
                                .desired_rows(lines)
                                .layouter(&mut layouter),
                        )
                        .changed();
                });
            });
        });
    changed
}
