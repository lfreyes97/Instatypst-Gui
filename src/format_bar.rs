//! Barra de formato (fuente, tamaño, B/I/U, color, alineación, listas) y su
//! selector de color con colores guardados. Solo maqueta: el estado se guarda
//! pero todavía no escribe en el documento.

use egui::ecolor::Hsva;
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Mesh, Rect, Sense,
    Shape, Stroke, StrokeKind, Ui,
};
use egui_phosphor::regular as ic;

use crate::theme::{dims, pal};
use crate::widgets::*;
use crate::layout::{LayoutAction, Tab};
use crate::State;

pub const FONTS: [&str; 5] = ["Inter", "Libertinus Serif", "New Computer Modern", "JetBrains Mono", "Fira Sans"];
const SIZES: [u32; 11] = [8, 10, 12, 14, 18, 24, 32, 48, 64, 96, 128];
const ALIGNS: [(&str, &str); 4] = [
    (ic::TEXT_ALIGN_LEFT, "Izquierda"),
    (ic::TEXT_ALIGN_CENTER, "Centro"),
    (ic::TEXT_ALIGN_RIGHT, "Derecha"),
    (ic::TEXT_ALIGN_JUSTIFY, "Justificado"),
];
const QUICK: [u32; 8] = [0xff5a1f, 0x3b3bff, 0xd400ff, 0x1f7a4d, 0xe0b000, 0xdb7a1f, 0xd63b3b, 0x808080];

fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub struct SavedGroup {
    pub name: &'static str,
    pub open: bool,
    pub colors: Vec<(&'static str, Color32)>,
}

pub struct FormatState {
    pub font: usize,
    pub size: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Color32,
    pub applied: bool,
    pub hsva: Hsva,
    pub hex: String,
    pub align: usize,
    pub bullets: bool,
    pub numbered: bool,
    pub saved: Vec<SavedGroup>,
}

fn to_color(h: &Hsva) -> Color32 {
    let [r, g, b, a] = h.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn hex_string(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

impl Default for FormatState {
    fn default() -> Self {
        let color = hex(0x9751f2);
        Self {
            font: 0,
            size: 18,
            bold: false,
            italic: false,
            underline: false,
            color,
            applied: false,
            hsva: Hsva::from_srgba_unmultiplied([color.r(), color.g(), color.b(), 255]),
            hex: hex_string(color),
            align: 0,
            bullets: false,
            numbered: false,
            saved: vec![
                SavedGroup {
                    name: "Atención",
                    open: true,
                    colors: vec![("Alertas", hex(0xff5a1f)), ("Éxito", hex(0x0f9d6a)), ("Atención", hex(0xffb62e))],
                },
                SavedGroup { name: "Escala de grises", open: false, colors: vec![("Negro", hex(0x111111)), ("Gris", hex(0x808080)), ("Blanco", hex(0xffffff))] },
                SavedGroup { name: "Primarios", open: false, colors: vec![("Acento", hex(0xf2a14e)), ("Azul", hex(0x2b59ff))] },
            ],
        }
    }
}

impl FormatState {
    fn set_color(&mut self, c: Color32) {
        self.color = c;
        self.applied = true;
        self.hsva = Hsva::from_srgba_unmultiplied([c.r(), c.g(), c.b(), c.a()]);
        self.hex = hex_string(c);
    }
}

// ---------- Widgets locales ----------

fn seg<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(pal().inset)
        .corner_radius(CornerRadius::same(dims::R_LG))
        .inner_margin(Margin::same(dims::SEG_PAD))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(add).inner
        })
        .inner
}

fn group(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = dims::S1;
        small_label(ui, label);
        add(ui);
    });
}

/// Dos grupos lado a lado; en paneles estrechos se apilan.
fn pair_layout(ui: &Ui) -> Layout {
    if ui.available_width() >= 240.0 { Layout::left_to_right(Align::Min) } else { Layout::top_down(Align::Min) }
}

fn toggle_btn(ui: &mut Ui, symbol: &str, tip: &str, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(dims::H_SEG_BTN, dims::H_SEG_BTN), Sense::click());
    if active {
        ui.painter().rect_filled(rect, CornerRadius::same(dims::R_MD), pal().accent_bg);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(dims::R_MD), pal().hover);
    }
    let col = if active { pal().accent } else if resp.hovered() { pal().text } else { pal().muted };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, symbol, egui::FontId::proportional(dims::G_MD), col);
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn swatch(ui: &mut Ui, color: Color32, selected: bool, side: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(dims::R_SM), color);
    if selected {
        ui.painter().rect_stroke(rect.expand(2.0), CornerRadius::same(dims::R_MD), Stroke::new(1.5, pal().text), StrokeKind::Outside);
    } else if resp.hovered() {
        ui.painter().rect_stroke(rect.expand(1.0), CornerRadius::same(dims::R_SM + 1), Stroke::new(1.0, pal().muted), StrokeKind::Outside);
    }
    resp.on_hover_text(hex_string(color)).on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---------- Paneles del dock ----------

/// Panel "Formato": una fila por familia de controles (cabe en paneles estrechos).
pub fn panel(st: &mut State, ui: &mut Ui) {
    let f = &mut st.fmt;
    let mut open_color = false;
    ui.spacing_mut().interact_size.y = dims::H_CTRL;
    ui.spacing_mut().item_spacing = vec2(dims::S4, dims::S3);

    group(ui, "Fuente", |ui| {
        egui::ComboBox::from_id_salt("fmt-font")
            .width(ui.available_width().min(240.0))
            .selected_text(FONTS[f.font])
            .show_ui(ui, |ui| {
                for (i, n) in FONTS.iter().enumerate() {
                    ui.selectable_value(&mut f.font, i, *n);
                }
            });
    });

    ui.with_layout(pair_layout(ui), |ui| {
        group(ui, "Tamaño", |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("fmt-size")
                    .width(36.0)
                    .selected_text(f.size.to_string())
                    .show_ui(ui, |ui| {
                        for s in SIZES {
                            ui.selectable_value(&mut f.size, s, s.to_string());
                        }
                    });
                seg(ui, |ui| {
                    let i = SIZES.iter().position(|s| *s >= f.size).unwrap_or(0);
                    if icon_btn_sized(ui, ic::TEXT_AA, "Aumentar", dims::H_SEG_BTN, dims::G_MD, pal().muted, false).clicked() {
                        f.size = SIZES[(i + 1).min(SIZES.len() - 1)];
                    }
                    if icon_btn_sized(ui, ic::TEXT_T, "Reducir", dims::H_SEG_BTN, dims::G_MD, pal().muted, false).clicked() {
                        f.size = SIZES[i.saturating_sub(1)];
                    }
                });
            });
        });
        group(ui, "Posición", |ui| {
            seg(ui, |ui| {
                icon_btn_sized(ui, ic::STACK, "Traer al frente", dims::H_SEG_BTN, dims::G_MD, pal().muted, false);
                icon_btn_sized(ui, ic::STACK_SIMPLE, "Enviar atrás", dims::H_SEG_BTN, dims::G_MD, pal().muted, false);
            });
        });
    });

    ui.with_layout(pair_layout(ui), |ui| {
        group(ui, "Formato", |ui| {
            seg(ui, |ui| {
                f.bold ^= toggle_btn(ui, ic::TEXT_B, "Negrita", f.bold);
                f.italic ^= toggle_btn(ui, ic::TEXT_ITALIC, "Cursiva", f.italic);
                f.underline ^= toggle_btn(ui, ic::TEXT_UNDERLINE, "Subrayado", f.underline);
            });
        });
        group(ui, "Alinear", |ui| {
            seg(ui, |ui| {
                egui::ComboBox::from_id_salt("fmt-align")
                    .width(40.0)
                    .selected_text(ALIGNS[f.align].0)
                    .show_ui(ui, |ui| {
                        for (i, (sym, name)) in ALIGNS.iter().enumerate() {
                            ui.selectable_value(&mut f.align, i, format!("{sym}  {name}"));
                        }
                    });
            });
        });
    });

    group(ui, "Color", |ui| {
        seg(ui, |ui| {
            ui.add_space(dims::S1);
            for q in QUICK.iter().take(5) {
                let c = hex(*q);
                if swatch(ui, c, f.applied && f.color == c, 20.0).clicked() {
                    f.set_color(c);
                }
            }
            if toggle_btn(ui, ic::CARET_RIGHT, "Más colores (abre el panel Color)", false) {
                open_color = true;
            }
        });
    });

    ui.with_layout(pair_layout(ui), |ui| {
        group(ui, "Lista", |ui| {
            seg(ui, |ui| {
                f.bullets ^= toggle_btn(ui, ic::LIST_BULLETS, "Viñetas", f.bullets);
                f.numbered ^= toggle_btn(ui, ic::LIST_NUMBERS, "Numerada", f.numbered);
            });
        });
        group(ui, "Más", |ui| {
            seg(ui, |ui| {
                toggle_btn(ui, ic::TEXT_T_SLASH, "Quitar formato", false);
                toggle_btn(ui, ic::CHAT_CIRCLE_DOTS, "Comentar", false);
            });
        });
    });

    if open_color {
        st.layout_actions.push(LayoutAction::Focus(Tab::Color));
    }
}

/// Panel "Color": rápidos, guardados y selector libre.
pub fn color_panel(st: &mut State, ui: &mut Ui) {
    let f = &mut st.fmt;
    ui.spacing_mut().item_spacing = vec2(dims::S2, dims::S2);
    small_label(ui, "Rápidos");
    ui.horizontal_wrapped(|ui| {
        for q in QUICK {
            let c = hex(q);
            if swatch(ui, c, f.applied && f.color == c, 28.0).clicked() {
                f.set_color(c);
            }
        }
    });
    ui.add_space(dims::S1);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Colores guardados").size(dims::F_LABEL).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if icon_btn_sized(ui, ic::PLUS, "Guardar el color actual", dims::H_SEG_BTN, dims::G_MD, pal().muted, false).clicked() {
                let c = f.color;
                if let Some(g) = f.saved.first_mut() {
                    g.colors.push(("Nuevo", c));
                    g.open = true;
                }
            }
        });
    });
    let mut picked = None;
    for g in f.saved.iter_mut() {
        let caret = if g.open { ic::CARET_DOWN } else { ic::CARET_RIGHT };
        let r = ui.add(egui::Label::new(egui::RichText::new(format!("{caret}  {}", g.name)).size(dims::F_BODY).color(pal().muted)).sense(Sense::click()));
        if r.clicked() {
            g.open = !g.open;
        }
        if g.open {
            ui.horizontal_wrapped(|ui| {
                for (name, c) in &g.colors {
                    let w = chip_width(ui, name, true);
                    if chip(ui, name, Some(*c), false, vec2(w, dims::H_CTRL)).clicked() {
                        picked = Some(*c);
                    }
                }
            });
        }
    }
    if let Some(c) = picked {
        f.set_color(c);
    }
    ui.add_space(dims::S1);
    picker(f, ui);
}

fn quad(mesh: &mut Mesh, rect: Rect, colors: [Color32; 4]) {
    let i = mesh.vertices.len() as u32;
    for (p, c) in [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()].into_iter().zip(colors) {
        mesh.colored_vertex(p, c);
    }
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
}

fn handle(ui: &Ui, center: egui::Pos2, fill: Color32, r: f32) {
    ui.painter().circle(center, r, fill, Stroke::new(2.0, Color32::WHITE));
    ui.painter().circle_stroke(center, r + 1.0, Stroke::new(1.0, Color32::from_black_alpha(90)));
}

fn picker(f: &mut FormatState, ui: &mut Ui) {
    let mut changed = false;

    // Cuadro saturación/brillo.
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 160.0), Sense::click_and_drag());
    if let Some(p) = resp.interact_pointer_pos() {
        f.hsva.s = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        f.hsva.v = 1.0 - ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        changed = true;
    }
    let hue = to_color(&Hsva::new(f.hsva.h, 1.0, 1.0, 1.0));
    let mut m = Mesh::default();
    quad(&mut m, rect, [Color32::WHITE, hue, hue, Color32::WHITE]);
    quad(&mut m, rect, [Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::BLACK, Color32::BLACK]);
    ui.painter().add(Shape::mesh(m));
    ui.painter().rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.0, pal().border), StrokeKind::Inside);
    let hp = pos2(rect.left() + f.hsva.s * rect.width(), rect.top() + (1.0 - f.hsva.v) * rect.height());
    handle(ui, hp, to_color(&Hsva::new(f.hsva.h, f.hsva.s, f.hsva.v, 1.0)), 7.0);

    // Cuentagotas + deslizadores.
    ui.horizontal(|ui| {
        let cur = to_color(&f.hsva);
        let (r, resp) = ui.allocate_exact_size(vec2(dims::H_CTRL_LG, dims::H_CTRL_LG), Sense::click());
        ui.painter().rect_filled(r, CornerRadius::same(dims::R_MD), cur);
        let lum = cur.r() as u32 * 3 + cur.g() as u32 * 6 + cur.b() as u32;
        let fg = if lum > 1280 { Color32::BLACK } else { Color32::WHITE };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, ic::EYEDROPPER, egui::FontId::proportional(dims::G_MD), fg);
        resp.on_hover_text("Cuentagotas (próximamente)");

        ui.vertical(|ui| {
            // Dos barras de 14 px + hueco de 8 = alto del cuentagotas (36).
            ui.spacing_mut().item_spacing.y = dims::S2;
            let w = ui.available_width() - dims::S2;
            // Matiz.
            let (hr, hresp) = ui.allocate_exact_size(vec2(w, 14.0), Sense::click_and_drag());
            if let Some(p) = hresp.interact_pointer_pos() {
                f.hsva.h = ((p.x - hr.left()) / hr.width()).clamp(0.0, 1.0);
                changed = true;
            }
            let mut m = Mesh::default();
            for i in 0..6 {
                let a = hr.left() + hr.width() * i as f32 / 6.0;
                let b = hr.left() + hr.width() * (i + 1) as f32 / 6.0;
                let ca = to_color(&Hsva::new(i as f32 / 6.0, 1.0, 1.0, 1.0));
                let cb = to_color(&Hsva::new((i + 1) as f32 / 6.0, 1.0, 1.0, 1.0));
                quad(&mut m, Rect::from_min_max(pos2(a, hr.top()), pos2(b, hr.bottom())), [ca, cb, cb, ca]);
            }
            ui.painter().add(Shape::mesh(m));
            handle(ui, pos2(hr.left() + f.hsva.h * hr.width(), hr.center().y), to_color(&Hsva::new(f.hsva.h, 1.0, 1.0, 1.0)), 8.0);

            // Opacidad.
            let (ar, aresp) = ui.allocate_exact_size(vec2(w, 14.0), Sense::click_and_drag());
            if let Some(p) = aresp.interact_pointer_pos() {
                f.hsva.a = ((p.x - ar.left()) / ar.width()).clamp(0.0, 1.0);
                changed = true;
            }
            ui.painter().rect_filled(ar, CornerRadius::ZERO, Color32::from_gray(if pal().dark { 70 } else { 190 }));
            let solid = to_color(&Hsva::new(f.hsva.h, f.hsva.s, f.hsva.v, 1.0));
            let mut m = Mesh::default();
            quad(&mut m, ar, [Color32::TRANSPARENT, solid, solid, Color32::TRANSPARENT]);
            ui.painter().add(Shape::mesh(m));
            handle(ui, pos2(ar.left() + f.hsva.a * ar.width(), ar.center().y), solid, 8.0);
        });
    });

    // HEX.
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(64.0, dims::H_CTRL), Sense::hover());
        ui.painter().rect_filled(r, CornerRadius::same(dims::R_MD), pal().inset);
        ui.painter().text(pos2(r.left() + dims::S3, r.center().y), Align2::LEFT_CENTER, "HEX", egui::FontId::proportional(dims::F_BODY), pal().text);
        ui.painter().text(pos2(r.right() - dims::S3, r.center().y), Align2::CENTER_CENTER, ic::CARET_DOWN, egui::FontId::proportional(dims::F_CAPTION), pal().muted);
        let resp = ui.add_sized(
            [ui.available_width(), dims::H_CTRL],
            egui::TextEdit::singleline(&mut f.hex).margin(Margin::symmetric(10, 8)),
        );
        if resp.changed() {
            let t = f.hex.trim().trim_start_matches('#');
            if t.len() == 6
                && let Ok(v) = u32::from_str_radix(t, 16)
            {
                let c = hex(v);
                f.hsva = Hsva::from_srgba_unmultiplied([c.r(), c.g(), c.b(), (f.hsva.a * 255.0) as u8]);
                f.color = c;
                f.applied = true;
            }
        }
    });

    if changed {
        let c = to_color(&f.hsva);
        f.color = c;
        f.applied = true;
        f.hex = hex_string(c);
    }
}
