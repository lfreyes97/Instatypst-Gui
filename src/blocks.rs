//! Editor de bloques guiado por la firma del estilo. La lógica (qué ofrecer, qué falta, cómo
//! guardar) es pura y se prueba sin interfaz; la interfaz está al final.
//!
//! El contenido de una pieza es una lista de bloques o texto en Typst, y las dos formas son
//! equivalentes. Editar bloques de una pieza escrita en Typst la vuelve a escribir en Typst
//! cuando se puede (`markup::render`); si algún bloque no cabe en Typst, pasa a lista de bloques
//! y se dice por qué.

use instatypst_piece::markup;
use instatypst_piece::model::{Block, Catalog, Content, Issue, Item, ItemFull, ListOrder, PieceSpec, Role, Side, StyleSig, Style};

/// Los bloques de una pieza. `Err` = su Typst no se puede leer (con línea y columna).
pub fn blocks_of_content(spec: &PieceSpec) -> Result<Vec<Block>, Vec<Issue>> {
    match &spec.content {
        Some(Content::Blocks(b)) => Ok(b.clone()),
        Some(Content::Markup(m)) => {
            let (b, issues) = markup::parse(m);
            if issues.is_empty() { Ok(b) } else { Err(issues) }
        }
        None => Ok(Vec::new()),
    }
}

/// La pieza con estos bloques como contenido. Si estaba escrita en Typst y los bloques caben en
/// Typst, sigue en Typst; si no, queda como lista de bloques y el segundo valor dice por qué.
pub fn commit(spec: &PieceSpec, blocks: Vec<Block>) -> (PieceSpec, Option<String>) {
    let mut s = spec.clone();
    if matches!(spec.content, Some(Content::Markup(_))) {
        match markup::render(&blocks) {
            Ok(t) => {
                s.content = Some(Content::Markup(t));
                return (s, None);
            }
            Err(e) => {
                s.content = Some(Content::Blocks(blocks));
                return (s, Some(format!("La pieza pasó de Typst a lista de bloques: {e}")));
            }
        }
    }
    s.content = Some(Content::Blocks(blocks));
    (s, None)
}

/// Aplica un texto en Typst al contenido de la pieza `i`. Si no se puede leer, devuelve las
/// incidencias (`@línea:columna`) y no toca el documento.
pub fn apply_typst(project: &mut crate::project::Project, i: usize, text: &str) -> Result<(), Vec<Issue>> {
    let (_, issues) = markup::parse(text);
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut spec = project.doc.posts[i].clone();
    spec.content = Some(Content::Markup(text.to_string()));
    project.set_piece(i, spec);
    Ok(())
}

/// El texto en Typst de los bloques de la pieza, o por qué no se puede escribir así.
pub fn typst_of(spec: &PieceSpec) -> Result<String, String> {
    match &spec.content {
        Some(Content::Markup(m)) => Ok(m.clone()),
        Some(Content::Blocks(b)) => markup::render(b),
        None => Ok(String::new()),
    }
}

pub fn sig_of<'a>(cat: &'a Catalog, spec: &PieceSpec) -> Option<&'a StyleSig> {
    match &spec.style {
        Some(Style::Catalog(n)) => cat.styles.get(n),
        _ => None,
    }
}

/// Los tipos de bloque que se pueden añadir: los que el estilo acepta (todos, con un estilo propio),
/// sin repetir los que por función aparecen una sola vez.
pub fn offerable(cat: &Catalog, sig: Option<&StyleSig>, blocks: &[Block]) -> Vec<String> {
    cat.blocks
        .iter()
        .filter(|k| sig.is_none_or(|s| cat.acepta(s, k)))
        .filter(|k| !(new_block(k).es_unico() && blocks.iter().any(|b| b.kind() == k.as_str())))
        .cloned()
        .collect()
}

/// Los requisitos del estilo y si están cubiertos: `(alternativas, cubierto)`.
pub fn requirements(cat: &Catalog, sig: &StyleSig, blocks: &[Block]) -> Vec<(Vec<String>, bool)> {
    let kinds: Vec<&str> = blocks.iter().map(Block::kind).collect();
    let faltan = cat.faltan(sig, &kinds);
    sig.bloques
        .requeridos
        .iter()
        .map(|r| {
            let alt: Vec<String> = match r {
                instatypst_piece::model::Slot::One(s) => vec![s.clone()],
                instatypst_piece::model::Slot::AnyOf(v) => v.clone(),
            };
            let cubierto = !faltan.contains(&alt);
            (alt, cubierto)
        })
        .collect()
}

/// Un bloque nuevo, vacío (las reglas del contrato lo señalan hasta que se rellena).
pub fn new_block(kind: &str) -> Block {
    let t = String::new;
    match kind {
        "title" => Block::Title { text: t() },
        "subtitle" => Block::Subtitle { text: t() },
        "label" => Block::Label { text: t() },
        "body" => Block::Body { text: t() },
        "quote" => Block::Quote { text: t() },
        "epigraph" => Block::Epigraph { text: t(), source: None },
        "verse" => Block::Verse { lines: vec![t()], role: None },
        "conclusion" => Block::Conclusion { text: t() },
        "attribution" => Block::Attribution { author: Some(t()), role: None, work: None, date: None },
        "list" => Block::List { order: ListOrder::None, items: vec![Item::Text(t())] },
        "contrast" => Block::Contrast {
            left: Side { label: None, title: None, items: vec![Item::Text(t())] },
            right: Side { label: None, title: None, items: vec![Item::Text(t())] },
        },
        "signature" => Block::Signature { text: t() },
        "footer" => Block::Footer { text: t() },
        "number" => Block::Number { value: 1, total: None },
        "date" => Block::Date { day: 1, month: String::new() },
        _ => Block::Body { text: t() },
    }
}

pub fn kind_name(kind: &str) -> &'static str {
    match kind {
        "title" => "Título",
        "subtitle" => "Subtítulo",
        "label" => "Rótulo",
        "body" => "Cuerpo",
        "quote" => "Cita",
        "epigraph" => "Epígrafe",
        "verse" => "Verso",
        "conclusion" => "Conclusión",
        "attribution" => "Atribución",
        "list" => "Lista",
        "contrast" => "Contraste",
        "signature" => "Firma",
        "footer" => "Pie",
        "number" => "Número",
        "date" => "Fecha",
        _ => "Bloque",
    }
}

/// Mueve el bloque `i` una posición; devuelve su índice nuevo.
pub fn move_block(blocks: &mut [Block], i: usize, dir: isize) -> usize {
    let j = i as isize + dir;
    if i >= blocks.len() || j < 0 || j as usize >= blocks.len() {
        return i;
    }
    blocks.swap(i, j as usize);
    j as usize
}

// ── Interfaz ────────────────────────────────────────────────

use egui::{vec2, RichText, Ui};
use egui_phosphor::regular as ic;

use crate::theme::*;
use crate::widgets::*;
use crate::State;

fn text_edit(ui: &mut Ui, id: egui::Id, text: &mut String, hint: &str, rows: usize) -> bool {
    ui.add(
        egui::TextEdit::multiline(text)
            .id(id)
            .hint_text(hint)
            .desired_width(f32::INFINITY)
            .desired_rows(rows)
            .margin(egui::Margin::symmetric(8, 6)),
    )
    .changed()
}

/// Un campo opcional de texto: vacío es `None`.
fn opt_field(ui: &mut Ui, id: egui::Id, label: &str, v: &mut Option<String>) -> bool {
    let mut t = v.clone().unwrap_or_default();
    ui.label(RichText::new(label).size(dims::F_CAPTION).color(pal().muted));
    let ch = ui.add(egui::TextEdit::singleline(&mut t).id(id).desired_width(f32::INFINITY).margin(egui::Margin::symmetric(8, 6))).changed();
    if ch {
        *v = (!t.is_empty()).then_some(t);
    }
    ch
}

fn items_ui(ui: &mut Ui, id: egui::Id, items: &mut Vec<Item>) -> bool {
    let mut changed = false;
    let mut remove = None;
    for (n, it) in items.iter_mut().enumerate() {
        let iid = id.with(n);
        ui.horizontal(|ui| {
            let mut text = match it {
                Item::Text(t) => t.clone(),
                Item::Full(f) => f.text.clone(),
            };
            let w = (ui.available_width() - dims::H_CTRL * 2.0 - dims::S2 * 2.0).max(40.0);
            if ui.add(egui::TextEdit::singleline(&mut text).id(iid).desired_width(w).margin(egui::Margin::symmetric(8, 6))).changed() {
                match it {
                    Item::Text(t) => *t = text,
                    Item::Full(f) => f.text = text,
                }
                changed = true;
            }
            // Con etiqueta o rol, el ítem es una tabla; sin ellos, un texto.
            let detalle = matches!(it, Item::Full(_));
            if icon_btn_sized(ui, ic::TAG, "Etiqueta y rol del ítem", dims::H_CTRL, dims::G_SM, if detalle { pal().accent } else { pal().muted }, detalle).clicked() {
                *it = match it.clone() {
                    Item::Text(t) => Item::Full(ItemFull { text: t, label: Some(String::new()), role: None }),
                    Item::Full(f) => Item::Text(f.text),
                };
                changed = true;
            }
            if icon_btn_sized(ui, ic::X, "Quitar ítem", dims::H_CTRL, dims::G_SM, pal().muted, false).clicked() {
                remove = Some(n);
            }
        });
        if let Item::Full(f) = it {
            ui.horizontal(|ui| {
                changed |= opt_field(ui, iid.with("l"), "etiqueta", &mut f.label);
            });
            let mut r = f.role;
            egui::ComboBox::from_id_salt(iid.with("r")).selected_text(match r { None => "sin rol", Some(Role::Neutral) => "neutro", Some(Role::Affirm) => "afirma", Some(Role::Deny) => "niega" }).show_ui(ui, |ui| {
                ui.selectable_value(&mut r, None, "sin rol");
                ui.selectable_value(&mut r, Some(Role::Neutral), "neutro");
                ui.selectable_value(&mut r, Some(Role::Affirm), "afirma");
                ui.selectable_value(&mut r, Some(Role::Deny), "niega");
            });
            if r != f.role {
                f.role = r;
                changed = true;
            }
        }
    }
    if let Some(n) = remove {
        items.remove(n);
        changed = true;
    }
    if ui.button(format!("{}  Añadir ítem", ic::PLUS)).clicked() {
        items.push(Item::Text(String::new()));
        changed = true;
    }
    changed
}

fn side_ui(ui: &mut Ui, id: egui::Id, title: &str, s: &mut Side) -> bool {
    let mut ch = false;
    ui.label(RichText::new(title).size(dims::F_BODY).strong());
    ch |= opt_field(ui, id.with("lab"), "rótulo", &mut s.label);
    ch |= opt_field(ui, id.with("tit"), "título", &mut s.title);
    ch |= items_ui(ui, id.with("it"), &mut s.items);
    ch
}

/// Los campos de un bloque, con las reglas del contrato (único, repetible, `total ≥ value`, día 1–31).
fn fields(ui: &mut Ui, id: egui::Id, b: &mut Block) -> bool {
    let mut ch = false;
    match b {
        Block::Title { text } | Block::Subtitle { text } | Block::Label { text } | Block::Conclusion { text } | Block::Signature { text } | Block::Footer { text } => {
            ch |= text_edit(ui, id, text, "texto", 1);
        }
        Block::Body { text } | Block::Quote { text } => ch |= text_edit(ui, id, text, "texto", 3),
        Block::Epigraph { text, source } => {
            ch |= text_edit(ui, id, text, "texto del epígrafe", 2);
            ch |= opt_field(ui, id.with("s"), "de quién es", source);
        }
        Block::Verse { lines, role } => {
            let mut t = lines.join("\n");
            if text_edit(ui, id, &mut t, "un verso por línea", 4) {
                *lines = t.split('\n').map(str::to_string).collect();
                ch = true;
            }
            ch |= opt_field(ui, id.with("r"), "rol", role);
        }
        Block::Attribution { author, role, work, date } => {
            ch |= opt_field(ui, id.with("a"), "autor", author);
            ch |= opt_field(ui, id.with("r"), "cargo o descripción", role);
            ch |= opt_field(ui, id.with("w"), "obra", work);
            ch |= opt_field(ui, id.with("d"), "fecha", date);
            ui.label(RichText::new("al menos uno de autor, obra o fecha").size(dims::F_CAPTION).color(pal().muted));
        }
        Block::List { order, items } => {
            let mut o = *order;
            egui::ComboBox::from_id_salt(id.with("o")).selected_text(match o { ListOrder::None => "sin orden", ListOrder::Numbered => "numerada", ListOrder::Chronological => "cronológica" }).show_ui(ui, |ui| {
                ui.selectable_value(&mut o, ListOrder::None, "sin orden");
                ui.selectable_value(&mut o, ListOrder::Numbered, "numerada");
                ui.selectable_value(&mut o, ListOrder::Chronological, "cronológica");
            });
            if o != *order {
                *order = o;
                ch = true;
            }
            ch |= items_ui(ui, id.with("i"), items);
        }
        Block::Contrast { left, right } => {
            ch |= side_ui(ui, id.with("l"), "Lado izquierdo", left);
            ui.separator();
            ch |= side_ui(ui, id.with("r"), "Lado derecho", right);
        }
        Block::Number { value, total } => {
            ui.horizontal(|ui| {
                ch |= ui.add(egui::DragValue::new(value).range(0..=9999)).changed();
                ui.label("de");
                let mut t = total.unwrap_or(0);
                let mut con = total.is_some();
                if ui.checkbox(&mut con, "").changed() {
                    *total = con.then_some((*value).max(1));
                    ch = true;
                }
                if let Some(tt) = total {
                    t = *tt;
                    if ui.add(egui::DragValue::new(&mut t).range(*value..=9999)).changed() {
                        *tt = t.max(*value);
                        ch = true;
                    }
                }
            });
        }
        Block::Date { day, month } => {
            ui.horizontal(|ui| {
                ch |= ui.add(egui::DragValue::new(day).range(1..=31).prefix("día ")).changed();
                ch |= ui.add(egui::TextEdit::singleline(month).id(id.with("m")).hint_text("mes (NOV)").desired_width(120.0)).changed();
            });
        }
    }
    ch
}

/// El panel «Bloques»: requisitos del estilo, la lista de bloques con sus campos y el menú de añadir.
pub fn ui(app: &mut State, ui: &mut Ui) {
    let own = app.project.doc.posts[app.piece].clone();
    let eff = app.project.doc.merged(app.piece);
    let sig = sig_of(&app.catalog, &eff);
    let mut blocks = match blocks_of_content(&own) {
        Ok(b) => b,
        Err(issues) => {
            ui.add_space(dims::S2);
            ui.label(RichText::new("El Typst del contenido no se puede leer como bloques:").size(dims::F_BODY).color(pal().err));
            for i in issues.iter().take(6) {
                ui.label(RichText::new(format!("{}  {}", i.path.trim_start_matches('@'), i.message)).size(dims::F_CAPTION).color(pal().err));
            }
            ui.label(RichText::new("Arréglalo en la pestaña «Typst».").size(dims::F_CAPTION).color(pal().muted));
            return;
        }
    };
    let original = blocks.clone();
    let name = app.catalog.styles.keys().find(|n| matches!(&eff.style, Some(Style::Catalog(s)) if s == *n)).cloned();

    // Requisitos del estilo.
    ui.add_space(dims::S1);
    match (&name, sig) {
        (Some(n), Some(sig)) => {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{n} pide:")).size(dims::F_CAPTION).color(pal().muted));
                for (alt, ok) in requirements(&app.catalog, sig, &blocks) {
                    let txt = alt.iter().map(|k| kind_name(k)).collect::<Vec<_>>().join(" o ");
                    let (sym, col) = if ok { (ic::CHECK_CIRCLE, pal().ok) } else { (ic::WARNING, pal().warn) };
                    ui.label(RichText::new(format!("{sym} {txt}")).size(dims::F_CAPTION).color(col));
                }
            });
        }
        _ => {
            ui.label(RichText::new("Sin estilo del catálogo: se aceptan todos los bloques").size(dims::F_CAPTION).color(pal().muted));
        }
    }

    let issues: Vec<crate::checker::Finding> = app.issues().iter().filter(|f| f.piece == Some(app.piece)).cloned().collect();
    let mut remove = None;
    let mut mv: Option<(usize, isize)> = None;
    let n_blocks = blocks.len();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = dims::S2;
        for (i, b) in blocks.iter_mut().enumerate() {
            let selected = app.block_sel == Some(i);
            let frame = egui::Frame::new()
                .fill(pal().inset)
                .stroke(egui::Stroke::new(1.0, if selected { pal().accent } else { pal().border }))
                .corner_radius(egui::CornerRadius::same(dims::R_LG))
                .inner_margin(egui::Margin::same(dims::S2 as i8));
            let r = frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(kind_name(b.kind())).size(dims::F_LABEL).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_btn_sized(ui, ic::TRASH, "Quitar bloque", dims::H_CTRL, dims::G_SM, pal().muted, false).clicked() {
                            remove = Some(i);
                        }
                        if icon_btn_sized(ui, ic::ARROW_DOWN, "Bajar", dims::H_CTRL, dims::G_SM, pal().muted, false).clicked() && i + 1 < n_blocks {
                            mv = Some((i, 1));
                        }
                        if icon_btn_sized(ui, ic::ARROW_UP, "Subir", dims::H_CTRL, dims::G_SM, pal().muted, false).clicked() && i > 0 {
                            mv = Some((i, -1));
                        }
                    });
                });
                fields(ui, egui::Id::new(("bloque", app.piece, i)), b);
                // Lo que el contrato dice de este bloque.
                let prefijo = format!("post[{}].content[{}]", app.piece, i);
                for f in issues.iter().filter(|f| f.path == prefijo || f.path.starts_with(&format!("{prefijo}."))) {
                    ui.label(RichText::new(format!("{}  {}", ic::WARNING_CIRCLE, f.message)).size(dims::F_CAPTION).color(pal().err));
                }
            });
            if selected {
                r.response.scroll_to_me(Some(egui::Align::Center));
            }
        }
        if blocks.is_empty() {
            ui.label(RichText::new("La pieza no tiene bloques. Añade el primero:").size(dims::F_BODY).color(pal().muted));
        }
        // Añadir: solo lo que el estilo acepta.
        let ofrecidos = offerable(&app.catalog, sig, &blocks);
        egui::ComboBox::from_id_salt("add-block").width(ui.available_width()).selected_text(format!("{}  Añadir bloque…", ic::PLUS)).show_ui(ui, |ui| {
            for k in &ofrecidos {
                if ui.selectable_label(false, kind_name(k)).clicked() {
                    blocks.push(new_block(k));
                }
            }
        });
        if let Some(note) = &app.blocks_note {
            ui.label(RichText::new(note).size(dims::F_CAPTION).color(pal().warn));
        }
    });
    if let Some(i) = remove {
        blocks.remove(i);
    }
    if let Some((i, d)) = mv {
        move_block(&mut blocks, i, d);
    }
    if blocks != original {
        let (spec, note) = commit(&own, blocks);
        app.blocks_note = note;
        app.project.set_piece(app.piece, spec);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use instatypst_piece::model::Syntax;

    fn cat() -> Catalog {
        Catalog::embedded()
    }

    fn spec(style: &str, content: Option<Content>) -> PieceSpec {
        PieceSpec { name: Some("p".into()), style: Some(Style::Catalog(style.into())), content, ..Default::default() }
    }

    #[test]
    fn solo_se_ofrece_lo_que_el_estilo_acepta() {
        let c = cat();
        let liso = c.styles.get("liso");
        let o = offerable(&c, liso, &[]);
        // liso: body requerido; epigraph, attribution y signature opcionales; quote y verse cuentan como body
        for k in ["body", "quote", "verse", "epigraph", "attribution", "signature"] {
            assert!(o.contains(&k.to_string()), "liso debe ofrecer {k}: {o:?}");
        }
        for k in ["title", "list", "contrast", "number", "date"] {
            assert!(!o.contains(&k.to_string()), "liso no acepta {k}");
        }
        // un estilo propio no tiene firma: se ofrece todo
        assert_eq!(offerable(&c, None, &[]).len(), c.blocks.len());
    }

    #[test]
    fn un_bloque_unico_no_se_ofrece_dos_veces_pero_uno_repetible_si() {
        let c = cat();
        let suizo = c.styles.get("suizo");
        let con_titulo = vec![new_block("title")];
        assert!(!offerable(&c, suizo, &con_titulo).contains(&"title".to_string()), "el título es único");
        let con_rotulo = vec![new_block("label")];
        assert!(offerable(&c, suizo, &con_rotulo).contains(&"label".to_string()), "un rótulo puede repetirse");
    }

    #[test]
    fn los_requisitos_marcan_lo_que_falta_y_lo_que_ya_esta() {
        let c = cat();
        let liso = &c.styles["liso"];
        let r = requirements(&c, liso, &[]);
        assert!(r.iter().all(|(_, ok)| !ok) && !r.is_empty());
        let r = requirements(&c, liso, &[Block::Body { text: "x".into() }]);
        assert!(r.iter().all(|(_, ok)| *ok));
        // un «uno de»: cita-orlada pide quote o body
        let orlada = &c.styles["cita-orlada"];
        let r = requirements(&c, orlada, &[]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, vec!["quote".to_string(), "body".to_string()]);
        assert!(requirements(&c, orlada, &[Block::Quote { text: "x".into() }])[0].1);
    }

    #[test]
    fn editar_bloques_de_una_pieza_en_typst_la_deja_en_typst() {
        let s = spec("liso", Some(Content::Markup("= Hola\n\nUn texto".into())));
        let mut bs = blocks_of_content(&s).unwrap();
        bs.push(Block::Footer { text: "pie".into() });
        let (nuevo, nota) = commit(&s, bs);
        assert_eq!(nota, None);
        let Some(Content::Markup(t)) = &nuevo.content else { panic!("debía seguir en Typst: {:?}", nuevo.content) };
        assert!(t.contains("#footer[pie]") && t.contains("= Hola"), "{t}");
        // y los bloques son los mismos que si se hubieran escrito a mano
        assert_eq!(blocks_of_content(&nuevo).unwrap().len(), 3);
    }

    #[test]
    fn si_un_bloque_no_cabe_en_typst_la_pieza_pasa_a_bloques_y_se_dice_por_que() {
        let s = spec("liso", Some(Content::Markup("Un texto".into())));
        let mut bs = blocks_of_content(&s).unwrap();
        bs.push(new_block("contrast"));
        let (nuevo, nota) = commit(&s, bs);
        assert!(matches!(nuevo.content, Some(Content::Blocks(_))));
        assert!(nota.unwrap().contains("contrast"));
        // una pieza que ya era de bloques sigue siéndolo
        let s = spec("liso", Some(Content::Blocks(vec![])));
        assert!(matches!(commit(&s, vec![new_block("body")]).0.content, Some(Content::Blocks(_))));
    }

    #[test]
    fn el_typst_se_aplica_al_documento_y_un_error_trae_linea_y_columna_sin_tocarlo() {
        let mut p = crate::project::Project::new_empty();
        p.add_piece();
        assert!(apply_typst(&mut p, 0, "= Título\n\nCuerpo").is_ok());
        assert!(matches!(&p.doc.posts[0].content, Some(Content::Markup(m)) if m.starts_with("= Título")));
        let antes = p.doc.clone();
        let e = apply_typst(&mut p, 0, "= Bien\n\n#image(\"x.png\")").unwrap_err();
        assert!(!e.is_empty() && e[0].path.starts_with('@'), "{e:?}");
        assert_eq!(p.doc, antes, "un Typst inválido no toca el documento");
        let e = apply_typst(&mut p, 0, "Hola @usuario").unwrap_err();
        assert_eq!(e[0].path, "@1:6", "línea:columna de la arroba");
    }

    #[test]
    fn de_bloques_a_typst_y_vuelta_o_dice_que_no_cabe() {
        let s = spec("liso", Some(Content::Blocks(vec![Block::Title { text: "T".into() }, Block::Body { text: "B".into() }])));
        let t = typst_of(&s).unwrap();
        assert_eq!(t, "= T\n\nB");
        let s = spec("liso", Some(Content::Blocks(vec![new_block("contrast")])));
        assert!(typst_of(&s).unwrap_err().contains("contrast"));
    }

    #[test]
    fn mover_un_bloque_respeta_los_limites() {
        let mut v = vec![new_block("title"), new_block("body"), new_block("footer")];
        assert_eq!(move_block(&mut v, 1, 1), 2);
        assert_eq!(v[2].kind(), "body");
        assert_eq!(move_block(&mut v, 2, 1), 2, "no pasa del final");
        assert_eq!(move_block(&mut v, 0, -1), 0, "no pasa del principio");
        assert_eq!(move_block(&mut v, 9, 1), 9, "índice fuera de rango");
    }

    // ── Interfaz ──

    fn frame(ctx: &egui::Context, st: &mut State) {
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(520.0, 900.0))), ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| crate::panels::editor(st, ui));
        out.textures_delta.clear();
    }

    /// Una pieza con los 15 tipos de bloque se dibuja en los tres modos sin problema.
    #[test]
    fn el_editor_dibuja_los_tres_modos_con_todos_los_bloques() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let todo = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Documentos/GitHub/typst-rrss/instatypst-cli/tests/fixtures/contrato/congelado/todo.toml")).unwrap();
        let doc = instatypst_piece::model::parse(&todo, Syntax::Toml).unwrap();
        let mut st = State::new();
        st.project.doc = doc;
        st.project.add_piece(); // otra, vacía
        assert!(!st.project.doc.posts.is_empty());
        for pieza in 0..st.project.doc.posts.len() {
            st.piece = pieza;
            for modo in 0..3 {
                st.editor_mode = modo;
                frame(&ctx, &mut st);
            }
        }
    }

    /// El flujo de la fase: una pieza de `liso` pide cuerpo; sin él lo marca, con él no.
    #[test]
    fn la_interfaz_guia_por_la_firma_y_editar_llega_al_documento() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let mut st = State::new();
        let i = st.project.add_piece();
        let mut p = st.project.doc.posts[i].clone();
        p.name = Some("a".into());
        p.style = Some(Style::Catalog("liso".into()));
        p.content = Some(Content::Blocks(vec![]));
        st.project.set_piece(i, p);
        st.editor_mode = 0;
        frame(&ctx, &mut st);
        let own = st.project.doc.posts[0].clone();
        let sig = sig_of(&st.catalog, &own).unwrap();
        assert!(requirements(&st.catalog, sig, &blocks_of_content(&own).unwrap()).iter().all(|(_, ok)| !ok), "sin bloques falta el cuerpo");
        // se añade el cuerpo con las mismas operaciones que usa la interfaz y se guarda
        let mut bs = blocks_of_content(&own).unwrap();
        bs.push(new_block("body"));
        let (nuevo, _) = commit(&own, bs);
        st.project.set_piece(0, nuevo);
        frame(&ctx, &mut st);
        let own = st.project.doc.posts[0].clone();
        let sig = sig_of(&st.catalog, &own).unwrap();
        assert!(requirements(&st.catalog, sig, &blocks_of_content(&own).unwrap()).iter().all(|(_, ok)| *ok));
        assert!(st.project.is_modified());
    }

    /// El criterio de la fase: todas las piezas de `fixtures/contrato/valid` se muestran como
    /// bloques y como Typst sin salir de los paneles, y dibujarlas sin tocar nada no cambia el documento.
    #[test]
    fn todas_las_piezas_de_las_fixtures_validas_se_editan_sin_salir_de_los_paneles() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Documentos/GitHub/typst-rrss/instatypst-cli/tests/fixtures/contrato/valid");
        let (mut piezas, mut en_typst, mut sin_typst) = (0, 0, 0);
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let ruta = e.path();
            if ruta.extension().is_none_or(|x| x != "toml") {
                continue;
            }
            let mut st = State::new();
            st.open_path(&ruta);
            assert!(st.status.is_none(), "{}: {:?}", ruta.display(), st.status);
            let antes = st.project.doc.clone();
            for pieza in 0..st.project.doc.posts.len() {
                st.piece = pieza;
                // los bloques se leen (de la lista o del Typst)
                let own = st.project.doc.posts[pieza].clone();
                assert!(blocks_of_content(&own).is_ok(), "{} pieza {pieza}", ruta.display());
                match typst_of(&own) {
                    Ok(_) => en_typst += 1,
                    Err(m) => {
                        assert!(m.contains("bloque"), "{m}");
                        sin_typst += 1;
                    }
                }
                for modo in [0, 1, 2] {
                    st.editor_mode = modo;
                    frame(&ctx, &mut st);
                }
                piezas += 1;
            }
            assert_eq!(st.project.doc, antes, "{}: dibujar los paneles modificó el documento", ruta.display());
            assert!(!st.project.is_modified());
        }
        assert!(piezas >= 5, "solo {piezas} piezas");
        eprintln!("{piezas} piezas: {en_typst} caben en Typst, {sin_typst} solo como bloques");
    }
}
