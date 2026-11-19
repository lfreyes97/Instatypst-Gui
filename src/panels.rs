//! Contenido de cada zona de la ventana.

use egui::text::LayoutJob;
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect,
    RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder,
};
use egui_phosphor::regular as ic;

use crate::theme::*;
use crate::widgets::*;
use crate::layout::{LayoutAction, Preset, Tab};
use crate::{highlight, State};

/// (nombre, fondo, tinta, acento)
const PRESETS: [(&str, Color32, Color32, Color32); 4] = [
    ("Minimal", Color32::from_rgb(0xf4, 0xef, 0xe6), Color32::from_rgb(0x1b, 0x1b, 0x1b), Color32::from_rgb(0xd9, 0x82, 0x2b)),
    ("Editorial", Color32::from_rgb(0xff, 0xff, 0xff), Color32::from_rgb(0x11, 0x11, 0x11), Color32::from_rgb(0x2b, 0x59, 0xff)),
    ("Noir", Color32::from_rgb(0x10, 0x10, 0x10), Color32::from_rgb(0xf2, 0xf2, 0xf2), Color32::from_rgb(0xe5, 0x48, 0x4d)),
    ("Pastel", Color32::from_rgb(0xcf, 0xe8, 0xe0), Color32::from_rgb(0x1e, 0x3a, 0x34), Color32::from_rgb(0xff, 0x7a, 0x59)),
];
const QUALITY: [(&str, u32); 3] = [("Base", 72), ("Pro", 144), ("Ultra", 300)];
/// (ancho, alto, etiqueta, px de ancho base)
const RATIOS: [(f32, f32, &str); 4] = [(1.0, 1.0, "1:1"), (4.0, 5.0, "4:5"), (16.0, 9.0, "16:9"), (9.0, 16.0, "9:16")];

pub fn top_bar(app: &mut State, ui: &mut Ui) {
    use dims::*;
    let rect = ui.max_rect();
    let row = Layout::left_to_right(Align::Center);
    const STATUS: &str = "typst · sin conectar";
    // Gestor de presets: botones de altura `H_CTRL` dentro de un marco de `S1`.
    let seg_pad = S1;
    let gap = S4;

    // Anchos de cada zona en su forma completa y compacta.
    let preset_full: Vec<f32> = Preset::ALL
        .iter()
        .map(|p| S3 + G_MD + S2 + text_width(ui, p.label(), F_BODY) + S3)
        .collect();
    let center_full = preset_full.iter().sum::<f32>() + seg_pad * 2.0;
    let center_compact = H_CTRL * Preset::ALL.len() as f32 + seg_pad * 2.0;
    let pill_full = S3 * 2.0 + status_dot_width(ui, STATUS);
    let buttons = H_CTRL_LG * 4.0 + S2 * 4.0; // avatar + 3 botones + huecos
    let right_full = buttons + pill_full;
    let right_compact = buttons + H_CTRL_LG;
    let left_min = H_CTRL_LG * 2.0 + S2; // logo + volver

    // Se compacta primero el estado y luego los presets (solo iconos).
    let fits = |c: f32, r: f32| rect.width() / 2.0 - c / 2.0 - gap >= r.max(left_min);
    let (center_w, compact_presets, compact_status) = if fits(center_full, right_full) {
        (center_full, false, false)
    } else if fits(center_full, right_compact) {
        (center_full, false, true)
    } else {
        (center_compact, true, true)
    };

    // Izquierda: logo + volver + título del documento (recortado al hueco libre).
    let left_w = (rect.width() / 2.0 - center_w / 2.0 - gap).max(left_min);
    let left = Rect::from_min_size(rect.min, vec2(left_w, rect.height()));
    ui.scope_builder(UiBuilder::new().max_rect(left).layout(row), |ui| {
        let (logo, _) = ui.allocate_exact_size(vec2(H_CTRL_LG, H_CTRL_LG), Sense::hover());
        ui.painter().rect_filled(logo, CornerRadius::same(R_LG), pal().text);
        ui.painter().text(logo.center(), Align2::CENTER_CENTER, "T", FontId::proportional(20.0), pal().bg);
        icon_btn(ui, ic::CARET_LEFT, "Volver a proyectos");
        let room = ui.available_width();
        if room > 48.0 {
            let g = truncated(ui, "Cartel — Enfoca tu día", FontId::proportional(F_LABEL), pal().text, room);
            let (r, _) = ui.allocate_exact_size(vec2(g.size().x, H_CTRL_LG), Sense::hover());
            ui.painter().galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, pal().text);
        }
    });

    // Centro: gestor de layout (presets).
    let center = Rect::from_center_size(rect.center(), vec2(center_w, rect.height()));
    ui.scope_builder(UiBuilder::new().max_rect(center).layout(row), |ui| {
        Frame::new()
            .fill(pal().panel)
            .stroke(Stroke::new(1.0, pal().border))
            .corner_radius(CornerRadius::same(R_XL))
            .inner_margin(Margin::same(seg_pad as i8))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (i, preset) in Preset::ALL.into_iter().enumerate() {
                    let w = if compact_presets { H_CTRL } else { preset_full[i] };
                    let (r, resp) = ui.allocate_exact_size(vec2(w, H_CTRL), Sense::click());
                    let active = app.layout_preset == preset;
                    if active || resp.hovered() {
                        let fill = if active { pal().hover } else { pal().inset };
                        ui.painter().rect_filled(r, CornerRadius::same(R_MD), fill);
                    }
                    let (ic_col, tx_col) = if active { (pal().accent, pal().text) } else { (pal().muted, pal().muted) };
                    if compact_presets {
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, preset.icon(), FontId::proportional(G_MD), ic_col);
                    } else {
                        ui.painter().text(pos2(r.left() + S3, r.center().y), Align2::LEFT_CENTER, preset.icon(), FontId::proportional(G_MD), ic_col);
                        ui.painter().text(pos2(r.left() + S3 + G_MD + S2, r.center().y), Align2::LEFT_CENTER, preset.label(), FontId::proportional(F_BODY), tx_col);
                    }
                    let tip = if compact_presets { format!("{} — {}", preset.label(), preset.description()) } else { preset.description().to_owned() };
                    let resp = resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.clicked() {
                        app.layout_actions.push(LayoutAction::Preset(preset));
                    }
                }
            });
    });

    // Derecha: estado, paneles, formato, tema, ajustes, avatar.
    let right_w = if compact_status { right_compact } else { right_full };
    let right = Rect::from_min_size(pos2(rect.right() - right_w, rect.top()), vec2(right_w, rect.height()));
    ui.scope_builder(UiBuilder::new().max_rect(right).layout(Layout::right_to_left(Align::Center)), |ui| {
        let (av, _) = ui.allocate_exact_size(vec2(H_CTRL_LG, H_CTRL_LG), Sense::hover());
        ui.painter().circle_filled(av.center(), H_CTRL_LG / 2.0, pal().accent);
        ui.painter().text(av.center(), Align2::CENTER_CENTER, "L", FontId::proportional(G_MD), pal().on_accent);
        icon_btn_sized(ui, ic::GEAR_SIX, "Ajustes", H_CTRL_LG, G_LG, pal().muted, true);
        let (sym, tip) = if app.dark { (ic::SUN, "Modo claro") } else { (ic::MOON, "Modo oscuro") };
        if icon_btn_sized(ui, sym, tip, H_CTRL_LG, G_LG, pal().muted, true).clicked() {
            app.dark = !app.dark;
            crate::theme::set_dark(ui.ctx(), app.dark);
        }
        let menu = icon_btn_sized(ui, ic::LAYOUT, "Paneles", H_CTRL_LG, G_LG, pal().muted, true);
        if menu.clicked() {
            app.panels_menu = !app.panels_menu;
        }
        if app.panels_menu {
            panels_menu(app, ui.ctx(), &menu);
        }
        let pill_w = if compact_status { H_CTRL_LG } else { pill_full };
        let (pill, resp) = ui.allocate_exact_size(vec2(pill_w, H_CTRL_LG), Sense::hover());
        ui.painter().rect(pill, CornerRadius::same((H_CTRL_LG / 2.0) as u8), pal().panel, Stroke::new(1.0, pal().border), StrokeKind::Inside);
        if compact_status {
            ui.painter().circle_filled(pill.center(), 4.0, pal().warn);
            resp.on_hover_text(STATUS);
        } else {
            ui.scope_builder(UiBuilder::new().max_rect(pill.shrink2(vec2(S3, 0.0))).layout(row), |ui| {
                status_dot(ui, pal().warn, STATUS);
            });
        }
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
        panel_header(ui, "Contenido", |ui| {
            icon_btn_sized(ui, ic::PLUS, "Añadir bloque", dims::H_CTRL, dims::G_MD, pal().muted, true);
            if icon_btn_sized(ui, ic::MAGNIFYING_GLASS, "Buscar", dims::H_CTRL, dims::G_MD, pal().muted, true).clicked() {
                app.search_open = !app.search_open;
            }
        });
        if app.search_open {
            ui.add(
                egui::TextEdit::singleline(&mut app.search)
                    .hint_text("Buscar bloques…")
                    .desired_width(f32::INFINITY)
                    .min_size(vec2(0.0, dims::H_CTRL))
                    .margin(Margin::symmetric(10, 8)),
            );
        }
        ui.add_space(dims::S1);

        let query = app.search.to_lowercase();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = dims::S1;
            for si in 0..app.sections.len() {
                section_header(app, ui, si);
                if !app.sections[si].open {
                    continue;
                }
                for bi in 0..app.sections[si].blocks.len() {
                    let (symbol, title, meta) = {
                        let b = &app.sections[si].blocks[bi];
                        (b.icon, b.title, b.meta)
                    };
                    if !query.is_empty() && !title.to_lowercase().contains(&query) {
                        continue;
                    }
                    let selected = app.selected == (si, bi);
                    if block_row(ui, symbol, title, meta, selected).clicked() {
                        app.selected = (si, bi);
                    }
                }
                ui.add_space(dims::S2);
            }
        });
    });
}

fn section_header(app: &mut State, ui: &mut Ui, si: usize) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), dims::H_ROW), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(dims::R_LG), pal().inset);
    }
    let s = &app.sections[si];
    let caret = if s.open { ic::CARET_DOWN } else { ic::CARET_RIGHT };
    let p = ui.painter();
    p.text(pos2(rect.left() + dims::S3, rect.center().y), Align2::CENTER_CENTER, caret, FontId::proportional(dims::G_SM), pal().muted);
    let thumb = Rect::from_center_size(pos2(rect.left() + ROW_ICON_X, rect.center().y), vec2(28.0, 28.0));
    p.rect_filled(thumb, CornerRadius::same(dims::R_MD), pal().accent_bg);
    p.text(thumb.center(), Align2::CENTER_CENTER, s.icon, FontId::proportional(dims::G_MD), pal().accent);
    p.text(pos2(rect.left() + ROW_TEXT_X, rect.center().y), Align2::LEFT_CENTER, s.title, FontId::proportional(dims::F_LABEL), pal().text);
    p.text(pos2(rect.right() - dims::S3, rect.center().y), Align2::CENTER_CENTER, ic::DOTS_THREE_VERTICAL, FontId::proportional(dims::G_MD), pal().muted);
    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        app.sections[si].open = !app.sections[si].open;
    }
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

fn block_row(ui: &mut Ui, symbol: &str, title: &str, meta: &str, selected: bool) -> egui::Response {
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

pub fn template(app: &mut State, ui: &mut Ui) {
    // Plantillas
    card(ui, false, |ui| {
        card_title(ui, "Plantilla");
        for i in 0..app.templates.len() {
            let (name, desc, colors) = {
                let t = &app.templates[i];
                (t.name, t.desc, t.colors)
            };
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), dims::H_ROW_2), Sense::click());
            let selected = app.template == i;
            let fill = if selected { pal().accent_bg } else if resp.hovered() { pal().hover } else { pal().inset };
            let stroke = if selected { Stroke::new(1.0, pal().accent) } else { Stroke::new(1.0, pal().border) };
            let p = ui.painter();
            p.rect(rect, CornerRadius::same(dims::R_LG), fill, stroke, StrokeKind::Inside);
            let c = pos2(rect.left() + 26.0, rect.center().y);
            p.circle_filled(c, 14.0, colors[0]);
            p.circle_stroke(c, 14.0, Stroke::new(1.0, pal().border));
            p.circle_filled(c + vec2(-3.0, 3.0), 5.0, colors[2]);
            // Las muestras de la derecha ocupan 3×16 px; el texto se recorta antes.
            let text_w = rect.width() - 52.0 - 3.0 * 16.0 - dims::S4;
            let p = ui.painter();
            let g = truncated(ui, name, FontId::proportional(dims::F_BODY), pal().text, text_w);
            let d = truncated(ui, desc, FontId::proportional(dims::F_CAPTION), pal().muted, text_w);
            p.galley(pos2(rect.left() + 52.0, rect.top() + 7.0), g, pal().text);
            p.galley(pos2(rect.left() + 52.0, rect.top() + 26.0), d, pal().muted);
            for (k, col) in colors.iter().enumerate() {
                let dc = pos2(rect.right() - 20.0 - k as f32 * 16.0, rect.center().y);
                p.circle_filled(dc, 6.0, *col);
                p.circle_stroke(dc, 6.0, Stroke::new(1.0, pal().border));
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                app.template = i;
                app.preset = i.min(PRESETS.len() - 1);
            }
        }
    });
}

pub fn style(app: &mut State, ui: &mut Ui) {
    // Estilo visual y exportación
    card(ui, false, |ui| {
        card_title(ui, "Estilo visual");
        small_label(ui, "Preajuste");
        let w = (ui.available_width() - dims::S2) / 2.0;
        egui::Grid::new("presets").num_columns(2).spacing([dims::S2, dims::S2]).show(ui, |ui| {
            for (i, (name, bg, _, _)) in PRESETS.iter().enumerate() {
                if chip(ui, name, Some(*bg), app.preset == i, vec2(w, dims::H_CTRL_LG)).clicked() {
                    app.preset = i;
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        ui.add_space(dims::S1);
        small_label(ui, "Resolución");
        let names: Vec<&str> = QUALITY.iter().map(|q| q.0).collect();
        segmented(ui, &names, &mut app.quality);
        ui.add_space(dims::S1);
        small_label(ui, "Relación de aspecto");
        let gap = ui.spacing().item_spacing.x;
        let bw = (ui.available_width() - gap * 3.0) / 4.0;
        ui.horizontal(|ui| {
            for (i, (w, h, tip)) in RATIOS.iter().enumerate() {
                if ratio_btn(ui, *w, *h, tip, app.ratio == i, vec2(bw, dims::H_CTRL_LG)).clicked() {
                    app.ratio = i;
                }
            }
        });
        ui.add_space(dims::S1);
        small_label(ui, "Formato");
        segmented(ui, &["PNG", "SVG", "PDF"], &mut app.format);

        ui.add_space(dims::S2);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), dims::H_ROW), Sense::click());
        let fill = if resp.hovered() { Color32::from_rgb(0xff, 0xb4, 0x6a) } else { pal().accent };
        ui.painter().rect_filled(rect, CornerRadius::same(dims::R_LG), fill);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, format!("{}  Exportar imagen", ic::EXPORT), FontId::proportional(dims::F_LABEL), pal().on_accent);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    });
}

pub fn preview(app: &mut State, ui: &mut Ui) {
    let area = ui.available_rect_before_wrap();
    ui.allocate_rect(area, Sense::hover());
    let (rw, rh, _) = RATIOS[app.ratio];
    // La página ocupa el hueco libre sobre la píldora inferior (altura `PILL_H`).
    const PILL_H: f32 = 24.0;
    let stage = Rect::from_min_max(
        area.min + vec2(dims::S3, dims::S3),
        area.max - vec2(dims::S3, PILL_H + dims::S4 + dims::S2),
    );
    let k = ((stage.width() / rw).min(stage.height() / rh)).max(20.0 / rw.max(rh));
    let page = Rect::from_center_size(stage.center(), vec2(rw * k, rh * k));

    let (_, bg, ink, accent) = PRESETS[app.preset];
    let painter = ui.painter();
    painter.rect_filled(page.translate(vec2(0.0, 8.0)).expand(2.0), CornerRadius::same(20), Color32::from_black_alpha(90));
    painter.rect_filled(page, CornerRadius::same(16), bg);
    let clip = painter.with_clip_rect(page);

    // Contenido de ejemplo escalado al ancho de la página.
    let u = page.width() / 1080.0;
    let pad = 96.0 * u;
    let sel = (app.selected == (0, 0)).then_some(());
    clip.circle_filled(page.right_top() + vec2(-60.0, 120.0) * u, 260.0 * u, Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 40));
    clip.text(page.left_top() + vec2(pad, pad), Align2::LEFT_TOP, "ENFOQUE · 90 MIN", FontId::proportional(24.0 * u), accent);
    let title_ink = if app.fmt.applied { app.fmt.color } else { ink };
    let title = clip.layout("La regla de\nlos 90 minutos".into(), FontId::proportional(104.0 * u), title_ink, page.width() - pad * 2.0);
    let title_pos = page.left_top() + vec2(pad, pad + 70.0 * u);
    let title_rect = Rect::from_min_size(title_pos, title.size());
    clip.galley(title_pos, title, title_ink);
    if sel.is_some() {
        clip.rect_stroke(title_rect.expand(8.0 * u), CornerRadius::same(6), Stroke::new(1.5, pal().accent), StrokeKind::Outside);
    }
    let body = clip.layout(
        "Elige una tarea, fija un temporizador y trabaja sin interrupciones hasta que suene.".into(),
        FontId::proportional(34.0 * u),
        ink.gamma_multiply(0.75),
        page.width() - pad * 2.0,
    );
    clip.galley(pos2(title_pos.x, title_rect.bottom() + 40.0 * u), body, ink);
    clip.rect_filled(Rect::from_min_size(pos2(page.left() + pad, page.bottom() - pad - 8.0 * u), vec2(120.0 * u, 8.0 * u)), CornerRadius::same(4), accent);
    clip.text(pos2(page.right() - pad, page.bottom() - pad), Align2::RIGHT_BOTTOM, "@usuario", FontId::proportional(28.0 * u), ink.gamma_multiply(0.6));

    // Píldora con información de exportación.
    let (_, _, ratio) = RATIOS[app.ratio];
    let width_px = 1080;
    let height_px = (width_px as f32 * rh / rw).round() as u32;
    let info = format!("{ratio}  ·  {width_px}×{height_px}  ·  {} ppi  ·  {:.0}%", QUALITY[app.quality].1, app.zoom * 100.0);
    let pill_w = (text_width(ui, &info, dims::F_CAPTION) + dims::S4 * 2.0).min(area.width());
    let pill = Rect::from_center_size(pos2(area.center().x, area.bottom() - dims::S1 - PILL_H / 2.0), vec2(pill_w, PILL_H));
    let painter = ui.painter();
    painter.rect(pill, CornerRadius::same((PILL_H / 2.0) as u8), pal().panel, Stroke::new(1.0, pal().border), StrokeKind::Inside);
    painter.with_clip_rect(pill).text(pill.center(), Align2::CENTER_CENTER, info, FontId::proportional(dims::F_CAPTION), pal().muted);
}

pub fn editor(app: &mut State, ui: &mut Ui) {
    use dims::*;
    // Cabecera: archivos abiertos (izquierda) + acciones (derecha).
    // Si no cabe todo, primero se oculta el estado y luego los archivos no
    // activos pasan a chips de solo icono (con el nombre en el tooltip).
    const STATUS: &str = "Listo · 38 ms";
    let gap = ui.spacing().item_spacing.x;
    let total = ui.available_width();
    let labels: Vec<String> = app.files.iter().map(|f| format!("{}  {}", f.icon, f.name)).collect();
    let full: Vec<f32> = labels.iter().map(|l| chip_width(ui, l, false)).collect();
    let chips_full = full.iter().sum::<f32>() + gap * (full.len().saturating_sub(1)) as f32;
    let actions_w = H_SEG_BTN * 3.0 + 2.0 * 2.0 + SEG_PAD as f32 * 2.0;
    let status_w = status_dot_width(ui, STATUS) + gap;
    let show_status = chips_full + gap + actions_w + status_w <= total;
    let compact = chips_full + gap + actions_w > total;
    let right_w = actions_w + if show_status { status_w } else { 0.0 };

    let top = ui.cursor().min;
    let left_rect = Rect::from_min_size(top, vec2((total - right_w - gap).max(H_CTRL), H_CTRL));
    let right_rect = Rect::from_min_size(pos2(top.x + total - right_w, top.y), vec2(right_w, H_CTRL));
    ui.scope_builder(UiBuilder::new().max_rect(left_rect).layout(Layout::left_to_right(Align::Center)), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(left_rect));
        for i in 0..app.files.len() {
            let active = app.file == i;
            let resp = if compact && !active {
                chip(ui, app.files[i].icon, None, false, vec2(H_CTRL, H_CTRL)).on_hover_text(app.files[i].name)
            } else {
                chip(ui, &labels[i], None, active, vec2(full[i], H_CTRL))
            };
            if resp.clicked() {
                app.file = i;
            }
        }
    });
    ui.scope_builder(UiBuilder::new().max_rect(right_rect).layout(Layout::right_to_left(Align::Center)), |ui| {
        Frame::new().fill(pal().inset).corner_radius(CornerRadius::same(R_LG)).inner_margin(Margin::same(SEG_PAD)).show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            icon_btn_sized(ui, ic::PLAY, "Compilar (Ctrl+Enter)", H_SEG_BTN, G_MD, pal().accent, false);
            icon_btn_sized(ui, ic::ARROW_CLOCKWISE, "Rehacer", H_SEG_BTN, G_MD, pal().muted, false);
            icon_btn_sized(ui, ic::ARROW_COUNTER_CLOCKWISE, "Deshacer", H_SEG_BTN, G_MD, pal().muted, false);
        });
        if show_status {
            status_dot(ui, pal().ok, STATUS);
        }
    });
    code_editor(app, ui);
}

/// Panel de archivos del proyecto.
pub fn files(app: &mut State, ui: &mut Ui) {
    panel_header(ui, "Archivos", |ui| {
        icon_btn_sized(ui, ic::PLUS, "Nuevo archivo", dims::H_CTRL, dims::G_MD, pal().muted, true);
    });
    ui.spacing_mut().item_spacing.y = dims::S1;
    for i in 0..app.files.len() {
        file_row(app, ui, i);
    }
}

fn file_row(app: &mut State, ui: &mut Ui, i: usize) {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), dims::H_ROW), Sense::click());
    let f = &mut app.files[i];
    let selected = app.file == i;
    let fill = if selected { pal().hover } else if resp.hovered() { pal().inset } else { pal().inset };
    let stroke = if selected { Stroke::new(1.0, pal().accent) } else { Stroke::NONE };
    ui.painter().rect(rect, CornerRadius::same(dims::R_LG), fill, stroke, StrokeKind::Inside);
    let col = if selected { pal().accent } else { pal().muted };
    let p = ui.painter();
    p.text(pos2(rect.left() + dims::S4, rect.center().y), Align2::CENTER_CENTER, f.icon, FontId::proportional(dims::G_MD), col);
    let lock = if f.locked { ic::LOCK_SIMPLE } else { ic::LOCK_SIMPLE_OPEN };
    let eye = if f.visible { ic::EYE } else { ic::EYE_SLASH };
    let lock_c = pos2(rect.right() - dims::S4 - 28.0, rect.center().y);
    let eye_c = pos2(rect.right() - dims::S4, rect.center().y);
    let text_x = rect.left() + dims::S4 * 2.0 + dims::S1;
    let name = truncated(ui, f.name, FontId::proportional(dims::F_BODY), pal().text, (lock_c.x - dims::S3 - text_x).max(0.0));
    let p = ui.painter();
    p.galley(pos2(text_x, rect.center().y - name.size().y / 2.0), name, pal().text);
    p.text(lock_c, Align2::CENTER_CENTER, lock, FontId::proportional(dims::G_SM), pal().muted);
    p.text(eye_c, Align2::CENTER_CENTER, eye, FontId::proportional(dims::G_SM), pal().muted);
    if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
        if pos.distance(lock_c) < 12.0 {
            f.locked = !f.locked;
        } else if pos.distance(eye_c) < 12.0 {
            f.visible = !f.visible;
        } else {
            app.file = i;
        }
    }
}

fn code_editor(app: &mut State, ui: &mut Ui) {
    Frame::new()
        .fill(pal().inset)
        .stroke(Stroke::new(1.0, pal().border))
        .corner_radius(CornerRadius::same(dims::R_LG))
        .inner_margin(Margin::symmetric(dims::S3 as i8, dims::S2 as i8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), ui.available_height()));
            let font = FontId::monospace(dims::F_CODE);
            let lines = app.code.split('\n').count().max(1);
            let gutter: String = (1..=lines).map(|n| n.to_string()).collect::<Vec<_>>().join("\n");
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = dims::S3;
                    ui.add(egui::Label::new(RichText::new(gutter).font(font.clone()).color(pal().muted.gamma_multiply(0.6))).selectable(false));
                    let font_l = font.clone();
                    let mut layouter = move |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                        let job = highlight::layout(text.as_str(), font_l.clone());
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    ui.add(
                        egui::TextEdit::multiline(&mut app.code)
                            .font(font)
                            .code_editor()
                            .frame(Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .desired_rows(lines)
                            .layouter(&mut layouter),
                    );
                });
            });
        });
}
