//! Resaltado mínimo de Typst para el editor (solo maqueta; el real vendrá del binario).

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

pub fn layout(text: &str, font: FontId) -> LayoutJob {
    let k = colors();
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let mut push = |s: &str, color: Color32| {
        if !s.is_empty() {
            job.append(s, 0.0, TextFormat::simple(font.clone(), color));
        }
    };

    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("//") {
            push(line, pal().muted);
            continue;
        }
        if line.starts_with('=') {
            push(line, k.heading);
            continue;
        }
        let chars: Vec<(usize, char)> = line.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            let (start, ch) = chars[i];
            let mut j = i + 1;
            let color = if ch == '#' {
                while j < chars.len() && (chars[j].1.is_alphanumeric() || chars[j].1 == '-') {
                    j += 1;
                }
                k.keyword
            } else if ch == '"' {
                while j < chars.len() && chars[j].1 != '"' {
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
                while j < chars.len() && !matches!(chars[j].1, '#' | '"') && !chars[j].1.is_ascii_digit() {
                    j += 1;
                }
                pal().text
            };
            let end = chars.get(j).map_or(line.len(), |c| c.0);
            push(&line[start..end], color);
            i = j;
        }
    }
    job
}
