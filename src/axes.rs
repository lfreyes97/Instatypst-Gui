//! Los ejes del contrato como controles. Todo sale del contrato
//! (`docs/contrato-v1.md`) y de las firmas del catálogo, no de listas a mano:
//!
//! - Plantilla: el estilo (con los bloques que admite cada uno).
//! - Pieza: formato, variante, fondo y `options`.
//! - Tipografía y marca.
//! - Color: la paleta.
//!
//! Cada panel edita una copia de la pieza y, si cambió, la devuelve al proyecto.
//! Los campos muestran lo que hereda la pieza de `[defaults]` y permiten volver a heredar.

use egui::{vec2, Color32, RichText, Ui};
use egui_phosphor::regular as ic;
use instatypst_color::{contrast, Rgb};
use instatypst_piece::model::{
    Background, Brand, BrandSpec, CustomSize, Font, FontSpec, Format, FormatName, Logo, PaletteDef, PaletteSpec,
    PieceSpec, Style, Typography,
};

use crate::theme::*;
use crate::widgets::*;
use crate::panels::block_row;
use crate::State;

// ── Utilidades ──────────────────────────────────────────────

/// Cabecera del panel y, si no hay pieza, el aviso. `false` = no hay nada que editar.
fn start(app: &State, ui: &mut Ui, title: &str) -> bool {
    panel_header(ui, title, |_| {});
    if app.project.doc.posts.is_empty() {
        ui.add_space(dims::S4);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No hay ninguna pieza").size(dims::F_BODY).color(pal().muted));
            ui.label(RichText::new("Crea una en el panel Piezas").size(dims::F_CAPTION).color(pal().muted));
        });
        return false;
    }
    true
}

/// Edita una copia de la pieza activa; si cambió, la guarda.
fn edit(app: &mut State, ui: &mut Ui, body: impl FnOnce(&mut Ui, &mut PieceSpec, &PieceSpec, &State)) {
    let own = app.project.doc.posts[app.piece].clone();
    let eff = app.project.doc.merged(app.piece);
    let mut new = own.clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing = vec2(dims::S2, dims::S2);
        body(ui, &mut new, &eff, app);
    });
    if new != own {
        app.project.set_piece(app.piece, new);
    }
}

/// Título de un grupo de controles; `inherits` muestra «heredado» y `Some(reset)` el botón de volver a heredar.
fn group_title(ui: &mut Ui, title: &str, own: bool, reset: &mut bool) {
    ui.add_space(dims::S2);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(dims::F_LABEL).strong());
        if own {
            if icon_btn_sized(ui, ic::ARROW_COUNTER_CLOCKWISE, "Volver a heredar del documento o del estilo", dims::H_CTRL, dims::G_SM, pal().muted, false).clicked() {
                *reset = true;
            }
        } else {
            ui.label(RichText::new("heredado").size(dims::F_CAPTION).color(pal().muted));
        }
    });
}

fn note(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).size(dims::F_CAPTION).color(pal().warn)).wrap());
}

fn text_field(ui: &mut Ui, value: &mut String, hint: &str) -> bool {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, dims::H_CTRL))
            .margin(egui::Margin::symmetric(10, 8)),
    )
    .changed()
}

fn opt_string(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.to_string())
}

// ── Plantilla: el estilo ────────────────────────────────────

/// Estilos del catálogo; cada uno con cuántos bloques de la pieza no admite
/// (`block_unsupported`) y cuántos le faltan (`block_missing`).
pub fn template(app: &mut State, ui: &mut Ui) {
    if !start(app, ui, "Plantilla") {
        return;
    }
    let key = (app.piece, app.project.revision);
    if app.tpl_fit.as_ref().map(|f| f.0) != Some(key) {
        let mut fit = std::collections::BTreeMap::new();
        for name in app.catalog.styles.keys() {
            let mut doc = app.project.doc.clone();
            doc.posts[app.piece].style = Some(Style::Catalog(name.clone()));
            let issues = instatypst_piece::model::validate_semantic(&doc, &app.catalog);
            let mine = |code: &str| issues.iter().filter(|i| i.code == code && crate::project::piece_of(&i.path) == Some(app.piece)).count();
            fit.insert(name.clone(), (mine("block_unsupported"), mine("block_missing")));
        }
        app.tpl_fit = Some((key, fit));
    }
    ui.add(
        egui::TextEdit::singleline(&mut app.tpl_search)
            .hint_text("Buscar estilo…")
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, dims::H_CTRL))
            .margin(egui::Margin::symmetric(10, 8)),
    );
    let q = app.tpl_search.to_lowercase();
    let eff = app.project.doc.merged(app.piece);
    let current = match &eff.style {
        Some(Style::Catalog(n)) => Some(n.clone()),
        _ => None,
    };
    if let Some(Style::Custom(c)) = &eff.style {
        ui.label(RichText::new(format!("Estilo propio: {} ({})", c.function, c.file)).size(dims::F_CAPTION).color(pal().muted));
    }
    let fit = app.tpl_fit.as_ref().map(|f| f.1.clone()).unwrap_or_default();
    let mut pick: Option<String> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = dims::S1;
        for (name, sig) in &app.catalog.styles {
            if !q.is_empty() && !name.to_lowercase().contains(&q) && !sig.uso.to_lowercase().contains(&q) {
                continue;
            }
            let (fuera, faltan) = fit.get(name).copied().unwrap_or_default();
            let traducible = sig.entradas.as_object().is_some_and(|o| !o.is_empty());
            let mut meta = sig.uso.clone();
            if fuera > 0 {
                meta = format!("{fuera} bloque(s) de la pieza no los admite · {meta}");
            } else if faltan > 0 {
                meta = format!("le faltan {faltan} bloque(s) requeridos · {meta}");
            }
            if !traducible {
                meta = format!("sin render aún · {meta}");
            }
            let icon = if fuera + faltan > 0 { ic::WARNING } else if !traducible { ic::PROHIBIT } else { ic::SQUARES_FOUR };
            if block_row(ui, icon, name, &meta, current.as_deref() == Some(name.as_str())).clicked() {
                pick = Some(name.clone());
            }
        }
    });
    if let Some(name) = pick {
        let mut own = app.project.doc.posts[app.piece].clone();
        own.style = Some(Style::Catalog(name));
        app.project.set_piece(app.piece, own);
    }
}

// ── Pieza: formato, variante, fondo, options ────────────────

const FORMATS: [(FormatName, &str); 6] = [
    (FormatName::Square, "Cuadrado"),
    (FormatName::Portrait, "Vertical"),
    (FormatName::Story, "Historia"),
    (FormatName::Landscape, "Horizontal"),
    (FormatName::A4, "A4"),
    (FormatName::A5, "A5"),
];

pub fn piece(app: &mut State, ui: &mut Ui) {
    if !start(app, ui, "Pieza") {
        return;
    }
    edit(app, ui, |ui, new, eff, app| {
        // Formato
        let mut reset = false;
        group_title(ui, "Formato", new.format.is_some(), &mut reset);
        let cur = new.format.clone().or_else(|| eff.format.clone());
        ui.horizontal_wrapped(|ui| {
            for (name, label) in FORMATS {
                let on = matches!(&cur, Some(Format::Named(n)) if *n == name);
                if chip(ui, label, None, on, vec2(chip_width(ui, label, false), dims::H_CTRL)).clicked() {
                    new.format = Some(Format::Named(name));
                }
            }
            let custom = matches!(&cur, Some(Format::Custom(_)));
            if chip(ui, "Otro", None, custom, vec2(chip_width(ui, "Otro", false), dims::H_CTRL)).clicked() && !custom {
                new.format = Some(Format::Custom(CustomSize { width: 1080, height: 1080 }));
            }
        });
        if let Some(Format::Custom(c)) = &mut new.format {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut c.width).range(100..=4320).suffix(" px"));
                ui.label("×");
                ui.add(egui::DragValue::new(&mut c.height).range(100..=4320).suffix(" px"));
            });
        }
        if cur.is_none() {
            ui.label(RichText::new("sin formato: manda el del estilo").size(dims::F_CAPTION).color(pal().muted));
        }
        if reset {
            new.format = None;
        }

        // Variante
        let mut reset = false;
        group_title(ui, "Variante", new.variant.is_some(), &mut reset);
        let mut v = new.variant.clone().or_else(|| eff.variant.clone()).unwrap_or_default();
        if text_field(ui, &mut v, "la del estilo, si la tiene") {
            new.variant = opt_string(&v);
        }
        if reset {
            new.variant = None;
        }

        // Fondo
        let mut reset = false;
        group_title(ui, "Fondo", new.background.is_some(), &mut reset);
        let cur_bg = new.background.clone().or_else(|| eff.background.clone());
        let label = |b: &Option<Background>| match b {
            None => "Del estilo",
            Some(Background::Solid) => "Liso",
            Some(Background::Grid) => "Cuadrícula",
            Some(Background::Rules) => "Renglones",
            Some(Background::Paper) => "Papel",
            Some(Background::Contours) => "Curvas de nivel",
            Some(Background::Image { .. }) => "Imagen",
        };
        egui::ComboBox::from_id_salt("bg").width(ui.available_width()).selected_text(label(&cur_bg)).show_ui(ui, |ui| {
            for b in [
                None,
                Some(Background::Solid),
                Some(Background::Grid),
                Some(Background::Rules),
                Some(Background::Paper),
                Some(Background::Contours),
                Some(Background::Image { src: String::new() }),
            ] {
                let on = std::mem::discriminant(&b) == std::mem::discriminant(&cur_bg);
                if ui.selectable_label(on, label(&b)).clicked() && !on {
                    new.background = b;
                }
            }
        });
        if let Some(Background::Image { src }) = &mut new.background {
            text_field(ui, src, "ruta de la imagen");
        }
        if matches!(cur_bg, Some(Background::Grid | Background::Rules | Background::Paper | Background::Contours)) {
            note(ui, "Estos fondos están en el contrato, pero el puente aún no los traduce a un preset: con un estilo del catálogo darán error al generar.");
        }
        if reset {
            new.background = None;
        }

        options(ui, new, eff, app);
    });
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Number,
    Bool,
    Kw,
    Len,
    Color,
}

const KINDS: [(Kind, &str); 6] = [
    (Kind::Text, "Texto"),
    (Kind::Number, "Número"),
    (Kind::Bool, "Sí / No"),
    (Kind::Kw, "Palabra clave"),
    (Kind::Len, "Longitud"),
    (Kind::Color, "Color"),
];

fn kind_of(v: &serde_json::Value) -> Kind {
    use serde_json::Value::*;
    match v {
        Number(_) => Kind::Number,
        Bool(_) => Kind::Bool,
        Object(o) if o.contains_key("kw") => Kind::Kw,
        Object(o) if o.contains_key("len") => Kind::Len,
        Object(o) if o.contains_key("color") => Kind::Color,
        _ => Kind::Text,
    }
}

fn text_of(v: &serde_json::Value) -> String {
    use serde_json::Value::*;
    match v {
        String(s) => s.clone(),
        Object(o) => o.values().next().and_then(|x| x.as_str()).unwrap_or_default().to_string(),
        other => other.to_string(),
    }
}

fn default_of(k: Kind) -> serde_json::Value {
    use serde_json::json;
    match k {
        Kind::Text => json!(""),
        Kind::Number => json!(0),
        Kind::Bool => json!(false),
        Kind::Kw => json!({ "kw": "" }),
        Kind::Len => json!({ "len": "0pt" }),
        Kind::Color => json!({ "color": "#000000" }),
    }
}

/// `options`: parámetros propios del estilo. El nombre sale de su firma; los valores usan las
/// tablas del contrato (`{ kw }`, `{ len }`, `{ color }`). `formato` y `tema` no van aquí.
fn options(ui: &mut Ui, new: &mut PieceSpec, eff: &PieceSpec, app: &State) {
    ui.add_space(dims::S2);
    ui.label(RichText::new("Opciones del estilo").size(dims::F_LABEL).strong());
    let sig = match &eff.style {
        Some(Style::Catalog(n)) => app.catalog.styles.get(n),
        _ => None,
    };
    let Some(sig) = sig else {
        ui.label(RichText::new("Un estilo propio recibe todo: sin firma que listar").size(dims::F_CAPTION).color(pal().muted));
        return;
    };
    let mut remove: Option<String> = None;
    for (key, value) in new.options.iter_mut() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(key).size(dims::F_BODY).color(pal().text));
            if icon_btn_sized(ui, ic::X, "Quitar", dims::H_CTRL, dims::G_SM, pal().muted, false).clicked() {
                remove = Some(key.clone());
            }
        });
        let mut kind = kind_of(value);
        egui::ComboBox::from_id_salt(("optk", key.as_str()))
            .width(ui.available_width())
            .selected_text(KINDS.iter().find(|(k, _)| *k == kind).map_or("", |(_, l)| l))
            .show_ui(ui, |ui| {
                for (k, l) in KINDS {
                    if ui.selectable_label(kind == k, l).clicked() && kind != k {
                        kind = k;
                        *value = default_of(k);
                    }
                }
            });
        match kind {
            Kind::Number => {
                let mut n = value.as_f64().unwrap_or(0.0);
                if ui.add(egui::DragValue::new(&mut n).speed(0.1)).changed() {
                    *value = serde_json::Number::from_f64(n).map(Into::into).unwrap_or_else(|| default_of(Kind::Number));
                }
            }
            Kind::Bool => {
                let mut b = value.as_bool().unwrap_or(false);
                if ui.checkbox(&mut b, "").changed() {
                    *value = b.into();
                }
            }
            Kind::Color => {
                let mut s = text_of(value);
                let mut rgb = Rgb::parse(&s).map(|c| c.0).unwrap_or([128, 128, 128]);
                ui.horizontal(|ui| {
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        s = Rgb(rgb).hex();
                        *value = serde_json::json!({ "color": s });
                    }
                    let mut t = text_of(value);
                    if text_field(ui, &mut t, "#RRGGBB") {
                        *value = serde_json::json!({ "color": t });
                    }
                });
            }
            Kind::Text | Kind::Kw | Kind::Len => {
                let mut t = text_of(value);
                if text_field(ui, &mut t, "valor") {
                    *value = match kind {
                        Kind::Kw => serde_json::json!({ "kw": t }),
                        Kind::Len => serde_json::json!({ "len": t }),
                        _ => t.into(),
                    };
                }
            }
        }
        ui.separator();
    }
    if let Some(k) = remove {
        new.options.remove(&k);
    }
    let libres: Vec<&String> = sig
        .options
        .iter()
        .filter(|o| !matches!(o.as_str(), "formato" | "tema") && !new.options.contains_key(*o))
        .collect();
    egui::ComboBox::from_id_salt("optadd").width(ui.available_width()).selected_text(format!("{}  Añadir opción…", ic::PLUS)).show_ui(ui, |ui| {
        for o in libres {
            if ui.selectable_label(false, o).clicked() {
                new.options.insert(o.clone(), default_of(Kind::Text));
            }
        }
    });
}

// ── Tipografía y marca ──────────────────────────────────────

fn font_row(ui: &mut Ui, label: &str, own: &mut Option<Font>, inherited: &Option<Font>) {
    let mut reset = false;
    group_title(ui, label, own.is_some(), &mut reset);
    let cur = own.clone().or_else(|| inherited.clone());
    let (mut family, mut weight) = match &cur {
        Some(Font::Family(f)) => (f.clone(), None),
        Some(Font::Spec(s)) => (s.family.clone(), s.weight),
        None => (String::new(), None),
    };
    let mut changed = text_field(ui, &mut family, "familia (la del estilo si se deja vacía)");
    ui.horizontal(|ui| {
        let mut has = weight.is_some();
        if ui.checkbox(&mut has, "peso").changed() {
            weight = has.then_some(400);
            changed = true;
        }
        if let Some(w) = &mut weight {
            changed |= ui.add(egui::DragValue::new(w).range(100..=900).speed(10)).changed();
        }
    });
    if changed {
        *own = opt_string(&family).map(|f| match weight {
            Some(w) => Font::Spec(FontSpec { family: f, weight: Some(w) }),
            None => Font::Family(f),
        });
    }
    if reset {
        *own = None;
    }
}

pub fn typography(app: &mut State, ui: &mut Ui) {
    if !start(app, ui, "Tipografía y marca") {
        return;
    }
    edit(app, ui, |ui, new, eff, _| {
        let eff_t = eff.typography.clone().unwrap_or_default();
        let mut t = new.typography.clone().unwrap_or_default();
        font_row(ui, "Títulos", &mut t.heading, &eff_t.heading);
        font_row(ui, "Cuerpo", &mut t.body, &eff_t.body);
        new.typography = (t != Typography::default()).then_some(t);
        if new.typography.as_ref().is_some_and(|t| t.heading.is_some()) {
            note(ui, "`typography.heading` está en el contrato, pero el puente aún no lo traduce a un preset.");
        }

        // Marca
        let mut reset = false;
        group_title(ui, "Marca", new.brand.is_some(), &mut reset);
        let cur = new.brand.clone().or_else(|| eff.brand.clone());
        let modo = match &cur {
            None => 0,
            Some(BrandSpec::Off) => 1,
            Some(BrandSpec::On(b)) if b.logo.text.is_some() => 2,
            Some(BrandSpec::On(b)) if b.logo.image.is_some() => 3,
            Some(BrandSpec::On(_)) => 4,
        };
        let nombres = ["Del documento", "Sin marca", "Texto", "Imagen", "Función de Typst"];
        let mut sel = modo;
        egui::ComboBox::from_id_salt("brand").width(ui.available_width()).selected_text(nombres[modo]).show_ui(ui, |ui| {
            for (i, n) in nombres.iter().enumerate() {
                ui.selectable_value(&mut sel, i, *n);
            }
        });
        if sel != modo {
            new.brand = match sel {
                0 => None,
                1 => Some(BrandSpec::Off),
                2 => Some(BrandSpec::On(Brand { logo: Logo { text: Some(String::new()), ..Default::default() }, size: None })),
                3 => Some(BrandSpec::On(Brand { logo: Logo { image: Some(String::new()), ..Default::default() }, size: None })),
                _ => Some(BrandSpec::On(Brand { logo: Logo { typst: Some(String::new()), function: Some(String::new()), ..Default::default() }, size: None })),
            };
        }
        if let Some(BrandSpec::On(b)) = &mut new.brand {
            if let Some(t) = &mut b.logo.text {
                text_field(ui, t, "@usuario");
            }
            if let Some(i) = &mut b.logo.image {
                text_field(ui, i, "ruta de la imagen");
            }
            if let Some(t) = &mut b.logo.typst {
                text_field(ui, t, "archivo .typ");
                let f = b.logo.function.get_or_insert_with(String::new);
                text_field(ui, f, "función");
            } else {
                let mut tiene = b.size.is_some();
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut tiene, "tamaño propio").changed() {
                        b.size = tiene.then_some(1.0 / 30.0);
                    }
                    if let Some(s) = &mut b.size {
                        ui.add(egui::DragValue::new(s).range(0.005..=0.5).speed(0.001));
                    }
                });
            }
        }
        if reset {
            new.brand = None;
        }
    });
}

// ── Color: la paleta ────────────────────────────────────────

const ROLES: [&str; 7] = ["background", "text", "muted", "accent", "line", "good", "bad"];

fn role<'a>(d: &'a mut PaletteDef, r: &str) -> &'a mut Option<String> {
    match r {
        "background" => &mut d.background,
        "text" => &mut d.text,
        "muted" => &mut d.muted,
        "accent" => &mut d.accent,
        "line" => &mut d.line,
        "good" => &mut d.good,
        _ => &mut d.bad,
    }
}

pub fn palette(app: &mut State, ui: &mut Ui) {
    if !start(app, ui, "Color") {
        return;
    }
    edit(app, ui, |ui, new, eff, app| {
        let mut reset = false;
        group_title(ui, "Paleta", new.palette.is_some(), &mut reset);
        let cur = new.palette.clone().or_else(|| eff.palette.clone());
        let modo = match &cur {
            None => 0,
            Some(PaletteSpec::Name(_)) => 1,
            Some(PaletteSpec::Def(_)) => 2,
        };
        let nombres = ["Del estilo", "Con nombre", "Definida aquí"];
        let mut sel = modo;
        egui::ComboBox::from_id_salt("palmode").width(ui.available_width()).selected_text(nombres[modo]).show_ui(ui, |ui| {
            for (i, n) in nombres.iter().enumerate() {
                ui.selectable_value(&mut sel, i, *n);
            }
        });
        if sel != modo {
            new.palette = match sel {
                0 => None,
                1 => Some(PaletteSpec::Name(String::new())),
                _ => Some(PaletteSpec::Def(PaletteDef::default())),
            };
        }
        match &mut new.palette {
            Some(PaletteSpec::Name(n)) => {
                text_field(ui, n, "nombre de la paleta");
                if !app.project.doc.palettes.is_empty() {
                    ui.label(RichText::new("Del documento:").size(dims::F_CAPTION).color(pal().muted));
                    ui.horizontal_wrapped(|ui| {
                        for name in app.project.doc.palettes.keys() {
                            if chip(ui, name, None, n == name, vec2(chip_width(ui, name, false), dims::H_CTRL)).clicked() {
                                *n = name.clone();
                            }
                        }
                    });
                }
            }
            Some(PaletteSpec::Def(d)) => {
                // Una sola fuente: base, from_image o file.
                let fuente = if d.base.is_some() { 1 } else if d.from_image.is_some() { 2 } else if d.file.is_some() { 3 } else { 0 };
                let fn_ = ["Sin fuente (solo roles)", "Un color base", "Desde una imagen", "Desde un archivo"];
                let mut f = fuente;
                egui::ComboBox::from_id_salt("palsrc").width(ui.available_width()).selected_text(fn_[fuente]).show_ui(ui, |ui| {
                    for (i, n) in fn_.iter().enumerate() {
                        ui.selectable_value(&mut f, i, *n);
                    }
                });
                if f != fuente {
                    (d.base, d.from_image, d.file) = (None, None, None);
                    match f {
                        1 => d.base = Some("#888888".into()),
                        2 => d.from_image = Some(String::new()),
                        3 => d.file = Some(String::new()),
                        _ => {}
                    }
                }
                if let Some(b) = &mut d.base {
                    color_row(ui, "base", b);
                }
                if let Some(s) = &mut d.from_image {
                    text_field(ui, s, "ruta de la imagen");
                }
                if let Some(s) = &mut d.file {
                    text_field(ui, s, "archivo de paleta");
                }
                ui.add_space(dims::S1);
                ui.label(RichText::new("Roles (los que falten se derivan al resolver)").size(dims::F_CAPTION).color(pal().muted));
                for r in ROLES {
                    let slot = role(d, r);
                    let mut on = slot.is_some();
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut on, "").changed() {
                            *slot = on.then(|| "#888888".to_string());
                        }
                        ui.label(RichText::new(r).size(dims::F_BODY).color(if on { pal().text } else { pal().muted }));
                    });
                    if let Some(h) = slot {
                        color_row(ui, r, h);
                    }
                }
            }
            None => {}
        }
        if reset {
            new.palette = None;
        }

        // Contraste con los mismos mínimos que `check` (4,5 y 3).
        if let Some(PaletteSpec::Def(d)) = &new.palette {
            let c = |s: &Option<String>| s.as_deref().and_then(|s| Rgb::parse(s).ok());
            if let Some(bg) = c(&d.background) {
                ui.add_space(dims::S2);
                for (nombre, fg, min) in [("texto", c(&d.text), contrast::MIN_TEXT), ("acento", c(&d.accent), contrast::MIN_ACCENT)] {
                    if let Some(fg) = fg {
                        let r = contrast::ratio(fg, bg);
                        let (sym, col) = if r >= min { (ic::CHECK_CIRCLE, pal().ok) } else { (ic::WARNING, pal().warn) };
                        ui.label(RichText::new(format!("{sym}  {nombre} sobre fondo: {r:.1}:1 (mínimo {min}:1)")).size(dims::F_CAPTION).color(col));
                    }
                }
            }
        }
    });
}

fn color_row(ui: &mut Ui, id: &str, hex: &mut String) {
    ui.horizontal(|ui| {
        let mut rgb = Rgb::parse(hex).map(|c| c.0).unwrap_or([128, 128, 128]);
        if ui.color_edit_button_srgb(&mut rgb).changed() {
            *hex = Rgb(rgb).hex();
        }
        ui.push_id(id, |ui| {
            text_field(ui, hex, "#RRGGBB");
        });
    });
}

/// Color de la paleta efectiva de la pieza como `Color32` (para el esquema de la vista previa).
pub fn palette_color(eff: &PieceSpec, role: &str) -> Option<Color32> {
    let Some(PaletteSpec::Def(d)) = &eff.palette else { return None };
    let mut d = d.clone();
    let hex = role_get(&mut d, role).clone().or(if role == "background" { d.base.clone() } else { None })?;
    let c = Rgb::parse(&hex).ok()?;
    Some(Color32::from_rgb(c.0[0], c.0[1], c.0[2]))
}

fn role_get<'a>(d: &'a mut PaletteDef, r: &str) -> &'a mut Option<String> {
    role(d, r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use instatypst_piece::model::{Block, Content};

    fn screen() -> egui::RawInput {
        egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 900.0))), ..Default::default() }
    }

    fn run(ctx: &egui::Context, st: &mut State, panel: fn(&mut State, &mut Ui)) {
        let mut out = ctx.run_ui(screen(), |ui| panel(st, ui));
        out.textures_delta.clear();
    }

    fn state_con(toml: &str) -> State {
        let dir = std::env::temp_dir().join(format!("insta-gui-axes-{}-{}", std::process::id(), toml.len()));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("a.toml");
        std::fs::write(&ruta, toml).unwrap();
        let mut st = State::new();
        st.open_path(&ruta);
        assert!(st.status.is_none(), "{:?}", st.status);
        st
    }

    const DOC: &str = "schema = \"instatypst-piece/v1\"\n\n[defaults]\nformat = \"portrait\"\n\n[[post]]\nname = \"a\"\nstyle = \"liso\"\ncontent = [ { type = \"body\", text = \"hola\" } ]\n\n[post.palette]\nbackground = \"#101010\"\ntext = \"#f0f0f0\"\naccent = \"#ff8800\"\n";

    #[test]
    fn todos_los_paneles_se_dibujan_con_un_documento_real_y_con_uno_vacio() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        for mut st in [state_con(DOC), State::new()] {
            for panel in [template, piece, typography, palette] {
                run(&ctx, &mut st, panel);
            }
        }
    }

    use crate::theme;

    #[test]
    fn la_plantilla_dice_que_estilos_admiten_los_bloques_de_la_pieza() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut st = state_con(DOC);
        run(&ctx, &mut st, template);
        let fit = &st.tpl_fit.as_ref().expect("se calcula").1;
        assert_eq!(fit.len(), st.catalog.styles.len());
        // la pieza tiene solo `body`: `liso` la acepta tal cual
        assert_eq!(fit["liso"], (0, 0));
        // y un estilo que exige título y lista le pide lo que le falta
        let (_, faltan) = fit["escalera"];
        assert!(faltan > 0, "escalera exige list y conclusion: {:?}", fit["escalera"]);
    }

    #[test]
    fn el_ajuste_se_recalcula_al_cambiar_los_bloques() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut st = state_con(DOC);
        run(&ctx, &mut st, template);
        let antes = st.tpl_fit.as_ref().unwrap().0;
        let mut p = st.project.doc.posts[0].clone();
        p.content = Some(Content::Blocks(vec![Block::Number { value: 3, total: None }]));
        st.project.set_piece(0, p);
        run(&ctx, &mut st, template);
        assert_ne!(st.tpl_fit.as_ref().unwrap().0, antes);
        // `liso` ya no admite un número
        assert!(st.tpl_fit.as_ref().unwrap().1["liso"].0 > 0);
    }

    #[test]
    fn los_valores_de_options_usan_las_tablas_del_contrato() {
        use serde_json::json;
        for (v, k) in [
            (json!("x"), Kind::Text),
            (json!(1.5), Kind::Number),
            (json!(true), Kind::Bool),
            (json!({ "kw": "center" }), Kind::Kw),
            (json!({ "len": "96pt" }), Kind::Len),
            (json!({ "color": "#691322" }), Kind::Color),
        ] {
            assert!(kind_of(&v) == k, "{v}");
            assert!(kind_of(&default_of(k)) == k);
        }
        assert_eq!(text_of(&json!({ "len": "96pt" })), "96pt");
    }

    #[test]
    fn la_paleta_de_la_pieza_alimenta_el_esquema_y_el_contraste() {
        let st = state_con(DOC);
        let eff = st.project.doc.merged(0);
        assert_eq!(palette_color(&eff, "background"), Some(Color32::from_rgb(0x10, 0x10, 0x10)));
        assert_eq!(palette_color(&eff, "accent"), Some(Color32::from_rgb(0xff, 0x88, 0x00)));
        assert_eq!(palette_color(&eff, "line"), None);
        let (bg, tx) = (Rgb::parse("#101010").unwrap(), Rgb::parse("#f0f0f0").unwrap());
        assert!(contrast::ratio(tx, bg) > contrast::MIN_TEXT);
    }

    #[test]
    fn formato_heredado_y_propio_no_se_confunden() {
        let st = state_con(DOC);
        assert!(st.project.doc.posts[0].format.is_none(), "la pieza no declara formato");
        assert!(matches!(st.project.doc.merged(0).format, Some(Format::Named(FormatName::Portrait))), "lo hereda de defaults");
    }

    /// Lo que un control cambia llega al modelo, al texto del editor y al archivo guardado.
    #[test]
    fn un_control_cambia_la_pieza_el_editor_y_el_archivo() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut st = state_con(DOC);
        let mut out = ctx.run_ui(screen(), |ui| {
            edit(&mut st, ui, |_, new, _, _| {
                new.variant = Some("oscuro".into());
                new.format = Some(Format::Named(FormatName::Story));
                new.options.insert("margen".into(), serde_json::json!({ "len": "120pt" }));
            });
        });
        out.textures_delta.clear();
        assert_eq!(st.project.doc.posts[0].variant.as_deref(), Some("oscuro"));
        assert!(st.project.is_modified());
        // el editor se recarga con lo nuevo
        let mut out = ctx.run_ui(screen(), |ui| crate::panels::editor(&mut st, ui));
        out.textures_delta.clear();
        assert!(st.editor_text.contains("variant = \"oscuro\""), "{}", st.editor_text);
        assert!(st.editor_text.contains("len = \"120pt\""), "{}", st.editor_text);
        // y se guarda con el diff mínimo: los defaults y la paleta siguen igual
        st.project.save().unwrap();
        let t = std::fs::read_to_string(st.project.path.clone().unwrap()).unwrap();
        assert!(t.contains("format = \"portrait\"") && t.contains("accent = \"#ff8800\"") && t.contains("oscuro"), "{t}");
        // sin cambios, no hay reescritura
        let antes = st.project.revision;
        let mut out = ctx.run_ui(screen(), |ui| piece(&mut st, ui));
        out.textures_delta.clear();
        assert_eq!(st.project.revision, antes, "dibujar un panel no debe modificar el documento");
    }
}
