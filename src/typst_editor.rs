//! Editor de Typst: resaltado con `typst-syntax`, números de línea, sangría automática y
//! autocompletado de `typst-ide` (el mismo del compilador) con popup junto al cursor.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::text::{CCursor, CCursorRange};
use egui::{Align2, Color32, FontId, Frame, Id, Key, Margin, Modifiers, RichText, Stroke, Ui, vec2};
use instatypst_engine::{Completion, Engine};

use crate::theme::*;

struct Query {
    rev: u64,
    cursor: usize,
    explicit: bool,
    text: String,
    root: PathBuf,
    filename: String,
}

struct Reply {
    rev: u64,
    cursor: usize,
    from: usize,
    items: Vec<Completion>,
}

/// Estado de un editor de Typst: la conversación con el hilo del IDE y el popup.
pub struct TypEditor {
    tx: Sender<Query>,
    rx: Receiver<Reply>,
    last: Option<Reply>,
    asked: Option<(u64, usize, bool)>,
    /// Versión del texto (sube con cada cambio).
    pub rev: u64,
    sel: usize,
    /// El popup se cerró con Esc para esta versión.
    dismissed: Option<u64>,
    /// Aplicación pendiente de un elemento (clic en el popup).
    explicit_next: bool,
}

impl TypEditor {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, rx_q) = channel::<Query>();
        let (tx_r, rx) = channel::<Reply>();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("typst-ide".into())
            .stack_size(64 << 20)
            .spawn(move || {
                let engine = Engine::new(vec![]);
                while let Ok(mut q) = rx_q.recv() {
                    while let Ok(newer) = rx_q.try_recv() {
                        q = newer;
                    }
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| instatypst_engine::complete(&engine, &q.root, &q.filename, &q.text, q.cursor, q.explicit)));
                    if let Ok((from, items)) = r {
                        if tx_r.send(Reply { rev: q.rev, cursor: q.cursor, from, items }).is_err() {
                            break;
                        }
                    }
                    ctx.request_repaint();
                }
            })
            .expect("hilo typst-ide");
        Self { tx, rx, last: None, asked: None, rev: 0, sel: 0, dismissed: None, explicit_next: false }
    }

    fn request(&mut self, cursor: usize, explicit: bool, text: &str, root: &Path, filename: &str) {
        let key = (self.rev, cursor, explicit);
        if self.asked == Some(key) {
            return;
        }
        self.asked = Some(key);
        let _ = self.tx.send(Query { rev: self.rev, cursor, explicit, text: text.to_string(), root: root.to_path_buf(), filename: filename.to_string() });
    }

    fn poll(&mut self) {
        while let Ok(r) = self.rx.try_recv() {
            self.last = Some(r);
            self.sel = 0;
        }
    }
}

fn byte_of(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(i, _)| i)
}

fn char_of(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].chars().count()
}

/// Opciones que casan con lo ya escrito (`prefix`), las que empiezan por él primero.
fn filtered<'a>(items: &'a [Completion], prefix: &str) -> Vec<&'a Completion> {
    let p = prefix.to_lowercase();
    let mut out: Vec<&Completion> = items.iter().filter(|c| c.label.to_lowercase().starts_with(&p)).collect();
    if out.len() < 8 {
        out.extend(items.iter().filter(|c| !c.label.to_lowercase().starts_with(&p) && c.label.to_lowercase().contains(&p)));
    }
    out.truncate(40);
    out
}

/// Cuerpo de un elemento: los marcadores `${nombre}` pasan a `nombre`; devuelve también el
/// rango (en chars, relativo al cuerpo) del primero para dejarlo seleccionado.
fn expand(c: &Completion) -> (String, Option<(usize, usize)>) {
    let raw = c.apply.as_deref().unwrap_or(&c.label);
    let mut out = String::new();
    let mut first = None;
    let mut rest = raw;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('}') else { break };
        let name = &rest[i + 2..i + j];
        let a = out.chars().count();
        out.push_str(name);
        if first.is_none() && !name.is_empty() {
            first = Some((a, out.chars().count()));
        }
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    (out, first)
}

fn set_cursor(ctx: &egui::Context, id: Id, a: usize, b: usize) {
    let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
    st.store(ctx, id);
}

fn cursor_range(ctx: &egui::Context, id: Id) -> Option<(usize, usize)> {
    let st = egui::TextEdit::load_state(ctx, id)?;
    let r = st.cursor.char_range()?;
    let (p, s): (usize, usize) = (r.primary.index.into(), r.secondary.index.into());
    Some((p.min(s), p.max(s)))
}

/// Sustituye `[a, b)` (chars) por `with` y deja el cursor en `[c, d)`.
fn replace(ctx: &egui::Context, id: Id, text: &mut String, a: usize, b: usize, with: &str, c: usize, d: usize) {
    let (ba, bb) = (byte_of(text, a), byte_of(text, b));
    text.replace_range(ba..bb, with);
    set_cursor(ctx, id, c, d);
}

/// Editor con números de línea. `bad`: líneas (desde 0) con error. `true` si el texto cambió.
pub fn show(ui: &mut Ui, ed: &mut TypEditor, text: &mut String, id: &str, bad: &HashSet<usize>, root: &Path, filename: &str) -> bool {
    let wid = Id::new(id);
    let ctx = ui.ctx().clone();
    ed.poll();
    let focused = ctx.memory(|m| m.has_focus(wid));
    let mut changed = false;
    let mut explicit = false;

    // Popup vigente: la respuesta corresponde a esta versión y a este cursor.
    let cur = cursor_range(&ctx, wid);
    let cur_byte = cur.map(|(_, b)| byte_of(text, b));
    let live = match (&ed.last, cur_byte) {
        (Some(r), Some(cb)) if r.rev == ed.rev && r.cursor == cb && ed.dismissed != Some(ed.rev) && focused => Some(r),
        _ => None,
    };
    let shown: Vec<Completion> = live.map(|r| filtered(&r.items, &text[r.from.min(text.len())..cur_byte.unwrap_or(0).max(r.from.min(text.len()))]).into_iter().cloned().collect()).unwrap_or_default();
    let from = live.map(|r| r.from);
    let popup = !shown.is_empty();

    // Teclas que el popup o el editor se quedan antes que el TextEdit.
    let mut accept: Option<usize> = None;
    if focused {
        ui.input_mut(|i| {
            if popup {
                if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    ed.sel = (ed.sel + 1).min(shown.len() - 1);
                }
                if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    ed.sel = ed.sel.saturating_sub(1);
                }
                if i.consume_key(Modifiers::NONE, Key::Tab) || i.consume_key(Modifiers::NONE, Key::Enter) {
                    accept = Some(ed.sel.min(shown.len() - 1));
                }
                if i.consume_key(Modifiers::NONE, Key::Escape) {
                    ed.dismissed = Some(ed.rev);
                }
            }
            if i.consume_key(Modifiers::COMMAND, Key::Space) {
                explicit = true;
            }
        });
        if accept.is_none() && !popup {
            let (tab, enter) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::Tab), i.consume_key(Modifiers::NONE, Key::Enter)));
            if let Some((a, b)) = cur {
                if tab {
                    replace(&ctx, wid, text, a, b, "  ", a + 2, a + 2);
                    changed = true;
                } else if enter {
                    let before = &text[..byte_of(text, a)];
                    let line = before.rsplit('\n').next().unwrap_or("");
                    let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
                    let prev = before.trim_end_matches([' ', '\t']).chars().last();
                    let next = text[byte_of(text, b)..].chars().next();
                    let opens = matches!(prev, Some('[' | '{' | '('));
                    let closes = opens && matches!((prev, next), (Some('['), Some(']')) | (Some('{'), Some('}')) | (Some('('), Some(')')));
                    let inner = if opens { format!("{indent}  ") } else { indent.clone() };
                    let ins = if closes { format!("\n{inner}\n{indent}") } else { format!("\n{inner}") };
                    let at = a + 1 + inner.chars().count();
                    replace(&ctx, wid, text, a, b, &ins, at, at);
                    changed = true;
                }
            }
        }
    }
    if let (Some(i), Some(from), Some((_, b))) = (accept, from, cur) {
        let (body, sel) = expand(&shown[i]);
        let a = char_of(text, from);
        let (c, d) = sel.map_or((a + body.chars().count(), a + body.chars().count()), |(x, y)| (a + x, a + y));
        replace(&ctx, wid, text, a, b, &body, c, d);
        changed = true;
    }

    let font = FontId::monospace(dims::F_CODE);
    let lines = text.split('\n').count().max(1);
    let mut out_galley = None;
    Frame::new()
        .fill(pal().inset)
        .stroke(Stroke::new(1.0, pal().border))
        .corner_radius(egui::CornerRadius::same(dims::R_LG))
        .inner_margin(Margin::symmetric(dims::S3 as i8, dims::S2 as i8))
        .show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), ui.available_height()));
            let mut gutter = egui::text::LayoutJob::default();
            for n in 0..lines {
                let col = if bad.contains(&n) { pal().err } else { pal().muted.gamma_multiply(0.6) };
                let s = if n + 1 < lines { format!("{}\n", n + 1) } else { (n + 1).to_string() };
                gutter.append(&s, 0.0, egui::text::TextFormat::simple(font.clone(), col));
            }
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = dims::S3;
                    ui.add(egui::Label::new(gutter).selectable(false));
                    let font_l = font.clone();
                    let mut layouter = move |ui: &Ui, t: &dyn egui::TextBuffer, _wrap: f32| ui.fonts_mut(|f| f.layout_job(crate::highlight::layout(t.as_str(), font_l.clone())));
                    let out = egui::TextEdit::multiline(text)
                        .id(wid)
                        .font(font.clone())
                        .code_editor()
                        .lock_focus(true)
                        .frame(Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .desired_rows(lines)
                        .layouter(&mut layouter)
                        .show(ui);
                    if out.response.changed() {
                        changed = true;
                    }
                    out_galley = Some((out.galley, out.galley_pos));
                });
            });
        });

    if changed {
        ed.rev += 1;
        ed.dismissed = None;
    }
    // Pide completar tras escribir (o con Ctrl+Espacio); `typst-ide` decide si hay algo que ofrecer.
    if (changed || explicit) && focused {
        if let Some((_, b)) = cursor_range(&ctx, wid) {
            ed.explicit_next = explicit;
            ed.request(byte_of(text, b), explicit, text, root, filename);
        }
    }

    if popup {
        if let Some((galley, pos)) = out_galley {
            if let Some((_, b)) = cur {
                let rect = galley.pos_from_cursor(CCursor::new(b));
                let at = pos + vec2(rect.min.x, rect.max.y + 2.0);
                let mut click = None;
                egui::Area::new(Id::new((id, "popup"))).order(egui::Order::Foreground).fixed_pos(at).pivot(Align2::LEFT_TOP).show(&ctx, |ui| {
                    Frame::popup(ui.style()).fill(pal().panel).stroke(Stroke::new(1.0, pal().border)).show(ui, |ui| {
                        ui.set_max_width(360.0);
                        for (i, c) in shown.iter().take(8).enumerate() {
                            let selected = i == ed.sel.min(shown.len() - 1);
                            let r = ui.horizontal(|ui| {
                                let col = if selected { pal().accent } else { pal().text };
                                ui.label(RichText::new(&c.label).monospace().size(dims::F_CODE).color(col));
                                if let Some(d) = &c.detail {
                                    ui.label(RichText::new(d.chars().take(40).collect::<String>()).size(dims::F_CAPTION).color(pal().muted));
                                }
                            });
                            let rect = r.response.rect;
                            if selected {
                                ui.painter().rect_filled(rect.expand(2.0), 3.0, pal().accent_bg);
                            }
                            if ui.interact(rect, Id::new((id, "it", i)), egui::Sense::click()).clicked() {
                                click = Some(i);
                            }
                        }
                    });
                });
                if let (Some(i), Some(from)) = (click, from) {
                    let (body, sel) = expand(&shown[i]);
                    let a = char_of(text, from);
                    let (c, d) = sel.map_or((a + body.chars().count(), a + body.chars().count()), |(x, y)| (a + x, a + y));
                    replace(&ctx, wid, text, a, b, &body, c, d);
                    ctx.memory_mut(|m| m.request_focus(wid));
                    ed.rev += 1;
                    changed = true;
                }
            }
        }
    }
    let _ = Color32::TRANSPARENT;
    changed
}
