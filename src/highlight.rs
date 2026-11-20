//! Resaltado mínimo para el editor: TOML (la pieza como texto) y Typst (maqueta).

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};

use crate::theme::*;

struct Colors {
    keyword: Color32,
    string: Color32,
    heading: Color32,
    number: Color32,
}

fn colors() -> Colors {
    if pal().dark {
        Colors {
            keyword: pal().accent,
            string: Color32::from_rgb(0x8b, 0xe0, 0x9b),
            heading: Color32::from_rgb(0xf5, 0xc4, 0x8a),
            number: Color32::from_rgb(0x8f, 0xb8, 0xff),
        }
    } else {
        Colors {
            keyword: Color32::from_rgb(0xc2, 0x5e, 0x00),
            string: Color32::from_rgb(0x1a, 0x7f, 0x4b),
            heading: Color32::from_rgb(0x9a, 0x4a, 0x00),
            number: Color32::from_rgb(0x1f, 0x5f, 0xd1),
        }
    }
}

/// Estilo acumulado de las etiquetas del nodo y sus ancestros.
#[derive(Clone, Copy, PartialEq)]
struct Style {
    color: Color32,
    italics: bool,
    underline: Option<Color32>,
}

/// Resaltado de Typst con el analizador oficial (`typst-syntax`): entiende marcado, código y
/// matemáticas igual que el compilador.
pub fn layout(text: &str, font: FontId) -> LayoutJob {
    use typst_syntax::LinkedNode;
    let root = typst_syntax::parse(text);
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let base = Style { color: pal().text, italics: false, underline: None };
    let mut run: Option<(usize, usize, Style)> = None;
    let mut flush = |job: &mut LayoutJob, a: usize, b: usize, s: Style| {
        let mut f = TextFormat::simple(font.clone(), s.color);
        f.italics = s.italics;
        if let Some(c) = s.underline {
            f.underline = egui::Stroke::new(1.0, c);
        }
        job.append(&text[a..b], 0.0, f);
    };
    walk(&LinkedNode::new(&root), base, &mut |start, end, st| match &mut run {
        Some((_, e, s)) if *e == start && *s == st => *e = end,
        _ => {
            if let Some((a, b, s)) = run.replace((start, end, st)) {
                flush(&mut job, a, b, s);
            }
        }
    });
    let mut covered = 0;
    if let Some((a, b, s)) = run {
        flush(&mut job, a, b, s);
        covered = b;
    }
    if covered < text.len() {
        flush(&mut job, covered, text.len(), base);
    }
    job
}

fn walk(node: &typst_syntax::LinkedNode, inherited: Style, out: &mut impl FnMut(usize, usize, Style)) {
    let mut st = inherited;
    if let Some(tag) = typst_syntax::highlight(node) {
        apply(&mut st, tag);
    }
    if node.children().len() == 0 {
        let r = node.range();
        if !r.is_empty() {
            out(r.start, r.end, st);
        }
        return;
    }
    for child in node.children() {
        walk(&child, st, out);
    }
}

fn apply(st: &mut Style, tag: typst_syntax::Tag) {
    use typst_syntax::Tag;
    let k = colors();
    let c = |r, g, b| Color32::from_rgb(r, g, b);
    let (func, label, math, raw) = if pal().dark { (c(0xd2, 0xa8, 0xff), c(0x7f, 0xd4, 0xe6), c(0x8f, 0xb8, 0xff), c(0xb8, 0xc7, 0x9a)) } else { (c(0x7a, 0x3e, 0xc2), c(0x0a, 0x7e, 0x93), c(0x1f, 0x5f, 0xd1), c(0x5c, 0x6e, 0x2e)) };
    match tag {
        Tag::Comment => st.color = pal().muted,
        Tag::Punctuation => st.color = pal().muted,
        Tag::Escape => st.color = k.number,
        Tag::Strong => st.color = k.heading,
        Tag::Emph => st.italics = true,
        Tag::Link => {
            st.color = label;
            st.underline = Some(label);
        }
        Tag::Raw => st.color = raw,
        Tag::Label | Tag::Ref => st.color = label,
        Tag::Heading => st.color = k.heading,
        Tag::ListMarker | Tag::ListTerm => st.color = k.keyword,
        Tag::MathDelimiter | Tag::MathOperator | Tag::MathGroupingParens => st.color = math,
        Tag::Keyword => st.color = k.keyword,
        Tag::Operator => st.color = pal().muted,
        Tag::Number => st.color = k.number,
        Tag::String => st.color = k.string,
        Tag::Function => st.color = func,
        Tag::Interpolated => st.color = k.keyword,
        Tag::Error => st.underline = Some(pal().err),
    }
}

/// Resaltado de TOML: comentarios, cabeceras de tabla, claves, cadenas y números.
pub fn layout_toml(text: &str, font: FontId) -> LayoutJob {
    let k = colors();
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let mut push = |s: &str, color: Color32| {
        if !s.is_empty() {
            job.append(s, 0.0, TextFormat::simple(font.clone(), color));
        }
    };
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with('#') {
            push(line, pal().muted);
            continue;
        }
        if t.starts_with('[') {
            push(line, k.heading);
            continue;
        }
        // clave = valor (la clave llega hasta el primer `=` fuera de comillas)
        let (key, rest) = match line.find('=') {
            Some(i) if !line[..i].contains('"') => line.split_at(i),
            _ => ("", line),
        };
        push(key, k.keyword);
        let chars: Vec<(usize, char)> = rest.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            let (start, ch) = chars[i];
            let mut j = i + 1;
            let color = if ch == '"' {
                while j < chars.len() && chars[j].1 != '"' {
                    if chars[j].1 == '\\' {
                        j += 1;
                    }
                    j += 1;
                }
                j = (j + 1).min(chars.len());
                k.string
            } else if ch.is_ascii_digit() {
                while j < chars.len() && (chars[j].1.is_ascii_alphanumeric() || chars[j].1 == '.') {
                    j += 1;
                }
                k.number
            } else {
                while j < chars.len() && chars[j].1 != '"' && !chars[j].1.is_ascii_digit() {
                    j += 1;
                }
                pal().text
            };
            let end = chars.get(j).map_or(rest.len(), |c| c.0);
            push(&rest[start..end], color);
            i = j;
        }
    }
    job
}
