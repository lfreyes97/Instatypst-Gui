//! Modo debug de la lámina: de una caja de la página al bloque del contrato, y las medidas que
//! enseña el inspector. Lógica pura, sin interfaz.
//!
//! Los presets reciben cada texto como `eval("…", mode: "markup")`, así que los glifos no apuntan a
//! una línea del archivo generado. El enlace caja → bloque se hace por el **texto**: la caja de una
//! línea del título contiene un trozo del texto del bloque `title`. Es una heurística: dos bloques
//! con el mismo texto son indistinguibles, y un texto que el preset genera (números de serie,
//! rótulos fijos) no corresponde a ningún bloque.

use instatypst_color::{contrast, Rgb};
use instatypst_engine::{BoxInfo, BoxKind};
use instatypst_piece::markup;
use instatypst_piece::model::{Block, Content, Item, PieceSpec};

/// Los bloques de la pieza, vengan como lista o como Typst (en ese caso, ya convertidos).
pub fn blocks_of(spec: &PieceSpec) -> Vec<Block> {
    match &spec.content {
        Some(Content::Blocks(b)) => b.clone(),
        Some(Content::Markup(m)) => markup::parse(m).0,
        None => Vec::new(),
    }
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '*' | '_' | '\\'))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn item_text(i: &Item) -> String {
    match i {
        Item::Text(t) => t.clone(),
        Item::Full(f) => f.text.clone(),
    }
}

/// Los textos que un bloque pone en la lámina.
fn block_texts(b: &Block) -> Vec<String> {
    let opt = |o: &Option<String>| o.iter().cloned().collect::<Vec<_>>();
    match b {
        Block::Title { text }
        | Block::Subtitle { text }
        | Block::Label { text }
        | Block::Body { text }
        | Block::Quote { text }
        | Block::Conclusion { text }
        | Block::Signature { text }
        | Block::Footer { text } => vec![text.clone()],
        Block::Epigraph { text, source } => std::iter::once(text.clone()).chain(opt(source)).collect(),
        Block::Verse { lines, .. } => lines.clone(),
        Block::Attribution { author, role, work, date } => [author, role, work, date].into_iter().flat_map(opt).collect(),
        Block::List { items, .. } => items.iter().map(item_text).collect(),
        Block::Contrast { left, right } => [left, right]
            .into_iter()
            .flat_map(|s| opt(&s.label).into_iter().chain(opt(&s.title)).chain(s.items.iter().map(item_text)))
            .collect(),
        Block::Number { value, total } => {
            let mut v = vec![value.to_string()];
            if let Some(t) = total {
                v.push(t.to_string());
            }
            v
        }
        Block::Date { day, month } => vec![day.to_string(), month.clone()],
    }
}

/// El bloque cuyo texto contiene (o es) el de la caja. Gana la coincidencia exacta; luego la caja
/// como trozo del bloque (una línea de un párrafo partido); por último el bloque dentro de la caja.
pub fn block_for_text(blocks: &[Block], text: &str) -> Option<usize> {
    let t = norm(text);
    if t.is_empty() {
        return None;
    }
    let textos: Vec<Vec<String>> = blocks.iter().map(|b| block_texts(b).iter().map(|s| norm(s)).collect()).collect();
    let buscar = |f: &dyn Fn(&str) -> bool| textos.iter().position(|ts| ts.iter().any(|s| !s.is_empty() && f(s)));
    buscar(&|s| s == t)
        .or_else(|| if t.chars().count() >= 2 { buscar(&|s| s.contains(&t)) } else { None })
        .or_else(|| if t.chars().count() >= 2 { buscar(&|s| s.chars().count() >= 2 && t.contains(s)) } else { None })
}

/// La caja bajo el punto `(x, y)` en puntos: el texto manda sobre las formas, y la más pequeña sobre la grande.
pub fn hit(boxes: &[BoxInfo], x: f32, y: f32) -> Option<usize> {
    boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.contains(x, y))
        .min_by(|(_, a), (_, b)| {
            ((a.kind != BoxKind::Text), a.area())
                .partial_cmp(&((b.kind != BoxKind::Text), b.area()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
}

/// Contraste real de un texto sobre lo que tiene detrás: la forma rellena más pequeña que lo
/// cubre, o el fondo de la página.
pub fn text_contrast(boxes: &[BoxInfo], i: usize, page_bg: Option<[u8; 3]>) -> Option<f64> {
    let t = &boxes[i];
    let fg = t.fill?;
    let (cx, cy) = (t.rect[0] + t.rect[2] / 2.0, t.rect[1] + t.rect[3] / 2.0);
    let detras = boxes
        .iter()
        .enumerate()
        .filter(|(j, b)| *j != i && b.kind == BoxKind::Shape && b.fill.is_some() && b.contains(cx, cy))
        .min_by(|(_, a), (_, b)| a.area().partial_cmp(&b.area()).unwrap_or(std::cmp::Ordering::Equal))
        .and_then(|(_, b)| b.fill.map(|c| [c[0], c[1], c[2]]))
        .or(page_bg)?;
    Some(contrast::ratio(Rgb([fg[0], fg[1], fg[2]]), Rgb(detras)))
}

/// Caja que envuelve el texto y las imágenes, y los márgenes que deja a la página
/// `(izquierda, arriba, derecha, abajo)` en puntos.
pub fn content_margins(boxes: &[BoxInfo], page: (f32, f32)) -> Option<([f32; 4], (f32, f32, f32, f32))> {
    let mut it = boxes.iter().filter(|b| b.kind != BoxKind::Shape && b.area() > 0.0);
    let first = it.next()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.rect[0], first.rect[1], first.rect[0] + first.rect[2], first.rect[1] + first.rect[3]);
    for b in it {
        x0 = x0.min(b.rect[0]);
        y0 = y0.min(b.rect[1]);
        x1 = x1.max(b.rect[0] + b.rect[2]);
        y1 = y1.max(b.rect[1] + b.rect[3]);
    }
    Some(([x0, y0, x1 - x0, y1 - y0], (x0, y0, page.0 - x1, page.1 - y1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use instatypst_piece::model::{ItemFull, Side};

    fn b(kind: BoxKind, rect: [f32; 4], text: Option<&str>, fill: Option<[u8; 4]>) -> BoxInfo {
        BoxInfo { kind, rect, text: text.map(Into::into), font: None, size: None, fill }
    }

    fn bloques() -> Vec<Block> {
        vec![
            Block::Title { text: "La regla de los *90* minutos".into() },
            Block::Epigraph { text: "Una cita breve".into(), source: Some("Alguien".into()) },
            Block::Body { text: "Elige una tarea, fija un temporizador y trabaja sin interrupciones.".into() },
            Block::List { order: Default::default(), items: vec![Item::Text("Primero".into()), Item::Full(ItemFull { text: "Segundo".into(), label: None, role: None })] },
            Block::Number { value: 3, total: Some(8) },
            Block::Contrast { left: Side { label: Some("antes".into()), title: None, items: vec![] }, right: Side { label: Some("después".into()), title: None, items: vec![] } },
        ]
    }

    #[test]
    fn una_caja_se_empareja_con_su_bloque_por_el_texto() {
        let bs = bloques();
        assert_eq!(block_for_text(&bs, "La regla de los 90 minutos"), Some(0), "sin el énfasis");
        assert_eq!(block_for_text(&bs, "LA REGLA"), Some(0), "mayúsculas del preset");
        assert_eq!(block_for_text(&bs, "Una cita breve"), Some(1));
        assert_eq!(block_for_text(&bs, "Alguien"), Some(1), "la fuente del epígrafe es del epígrafe");
        // una línea de un párrafo partido
        assert_eq!(block_for_text(&bs, "trabaja sin interrupciones."), Some(2));
        assert_eq!(block_for_text(&bs, "Segundo"), Some(3));
        assert_eq!(block_for_text(&bs, "después"), Some(5));
        assert_eq!(block_for_text(&bs, "8"), Some(4));
    }

    #[test]
    fn lo_que_el_preset_genera_no_corresponde_a_ningun_bloque() {
        let bs = bloques();
        assert_eq!(block_for_text(&bs, "SERIE · 2026"), None);
        assert_eq!(block_for_text(&bs, ""), None);
        assert_eq!(block_for_text(&bs, "   "), None);
        // un trozo de una sola letra no se empareja por inclusión (daría falsos positivos)
        assert_eq!(block_for_text(&bs, "a"), None);
    }

    #[test]
    fn el_texto_manda_sobre_la_forma_y_la_pequena_sobre_la_grande() {
        let boxes = vec![
            b(BoxKind::Shape, [0.0, 0.0, 200.0, 100.0], None, Some([10, 10, 10, 255])),
            b(BoxKind::Shape, [20.0, 20.0, 60.0, 30.0], None, Some([200, 0, 0, 255])),
            b(BoxKind::Text, [25.0, 25.0, 40.0, 10.0], Some("hola"), Some([255, 255, 255, 255])),
        ];
        assert_eq!(hit(&boxes, 30.0, 30.0), Some(2), "el texto");
        assert_eq!(hit(&boxes, 70.0, 45.0), Some(1), "la forma pequeña");
        assert_eq!(hit(&boxes, 150.0, 80.0), Some(0));
        assert_eq!(hit(&boxes, 300.0, 300.0), None);
    }

    #[test]
    fn el_contraste_es_contra_lo_que_hay_detras_del_texto() {
        let boxes = vec![
            b(BoxKind::Shape, [20.0, 20.0, 60.0, 30.0], None, Some([255, 255, 255, 255])),
            b(BoxKind::Text, [25.0, 25.0, 40.0, 10.0], Some("sobre blanco"), Some([255, 255, 255, 255])),
            b(BoxKind::Text, [120.0, 60.0, 40.0, 10.0], Some("sobre el fondo"), Some([255, 255, 255, 255])),
        ];
        let bg = Some([0, 0, 0]);
        assert!(text_contrast(&boxes, 1, bg).unwrap() < 1.1, "blanco sobre blanco");
        assert!(text_contrast(&boxes, 2, bg).unwrap() > 20.0, "blanco sobre negro");
        assert_eq!(text_contrast(&boxes, 2, None), None, "sin fondo conocido no se inventa");
    }

    #[test]
    fn los_margenes_salen_de_la_caja_del_contenido() {
        let boxes = vec![
            b(BoxKind::Shape, [0.0, 0.0, 1000.0, 1000.0], None, Some([0, 0, 0, 255])),
            b(BoxKind::Text, [96.0, 100.0, 300.0, 50.0], Some("a"), None),
            b(BoxKind::Text, [96.0, 900.0, 200.0, 30.0], Some("b"), None),
        ];
        let (caja, (l, t, r, b_)) = content_margins(&boxes, (1000.0, 1000.0)).unwrap();
        assert_eq!(caja, [96.0, 100.0, 300.0, 830.0]);
        assert_eq!((l, t, r, b_), (96.0, 100.0, 604.0, 70.0));
        assert!(content_margins(&boxes[..1], (1000.0, 1000.0)).is_none(), "una forma sola no es contenido");
    }

    #[test]
    fn el_contenido_en_typst_se_convierte_a_bloques() {
        let spec = PieceSpec { content: Some(Content::Markup("= Hola\n\nUn texto".into())), ..Default::default() };
        let bs = blocks_of(&spec);
        assert_eq!(bs.len(), 2);
        assert_eq!(block_for_text(&bs, "Hola"), Some(0));
    }
}
