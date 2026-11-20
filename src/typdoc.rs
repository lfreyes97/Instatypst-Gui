//! Documentos `.typ` libres: el usuario escribe Typst completo (puede salirse del contrato de
//! las piezas). Se compilan con el mismo motor, a todas las páginas, en un hilo propio.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use instatypst_engine::{pixmap_to_rgba, render_page, Diagnostic, Engine, SyncEntry};

const DEBOUNCE: Duration = Duration::from_millis(250);

pub struct TypDoc {
    pub id: u64,
    pub path: Option<PathBuf>,
    /// Nombre mostrado cuando aún no tiene ruta.
    pub title: String,
    pub text: String,
    saved: String,
    /// Sube con cada cambio del texto.
    pub rev: u64,
}

impl TypDoc {
    pub fn new(id: u64, title: &str, text: String) -> Self {
        Self { id, path: None, title: title.into(), saved: text.clone(), text, rev: 0 }
    }

    pub fn open(id: u64, path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut d = Self::new(id, "", text);
        d.path = Some(path.to_path_buf());
        Ok(d)
    }

    pub fn name(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_name()).map_or_else(|| self.title.clone(), |n| n.to_string_lossy().into_owned())
    }

    pub fn is_modified(&self) -> bool {
        self.text != self.saved
    }

    pub fn root(&self) -> PathBuf {
        self.path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf).unwrap_or_else(std::env::temp_dir)
    }

    /// Nombre del archivo relativo a la raíz (para `#include` relativos).
    pub fn filename(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_name()).map_or_else(|| "main.typ".into(), |n| n.to_string_lossy().into_owned())
    }

    pub fn save(&mut self) -> Result<(), String> {
        let Some(p) = &self.path else { return Err("sin ruta".into()) };
        std::fs::write(p, &self.text).map_err(|e| format!("{}: {e}", p.display()))?;
        self.saved = self.text.clone();
        Ok(())
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), String> {
        self.path = Some(path.to_path_buf());
        self.save()
    }
}

pub struct Request {
    id: u64,
    text: String,
    root: PathBuf,
    filename: String,
    /// Ancho deseado de la página más ancha, en píxeles.
    target_px: f32,
}

pub struct Page {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    pub size_pt: (f32, f32),
}

struct Response {
    id: u64,
    pages: Option<Vec<Page>>,
    diags: Vec<Diagnostic>,
    sync: Vec<SyncEntry>,
    ms: u128,
}

fn compile(engine: &Engine, req: &Request) -> Response {
    let c = engine.compile(&req.text, &req.root, &req.filename);
    let pages = c.doc.as_ref().map(|doc| {
        let widest = doc.pages().iter().map(|p| p.frame.size().x.to_pt() as f32).fold(1.0, f32::max);
        let scale = (req.target_px / widest).clamp(0.1, 3.0);
        (0..doc.pages().len())
            .filter_map(|i| {
                let size = doc.pages()[i].frame.size();
                let pix = render_page(doc, i, scale)?;
                let (width, height, rgba) = pixmap_to_rgba(&pix);
                Some(Page { width, height, rgba, size_pt: (size.x.to_pt() as f32, size.y.to_pt() as f32) })
            })
            .collect()
    });
    Response { id: req.id, pages, diags: c.diags, sync: c.sync, ms: c.ms }
}

/// Qué se muestra: documento, versión y tamaño (a saltos de 64 px).
type Key = (u64, u64, u32);

pub struct PageTex {
    pub tex: egui::TextureHandle,
    pub size_pt: (f32, f32),
}

pub struct TypPreview {
    tx: Sender<Request>,
    rx: Receiver<Response>,
    next_id: u64,
    sent: Option<(Key, u64)>,
    pending: Option<(Key, Instant)>,
    shown: Option<Key>,
    current: Option<Key>,
    pub pages: Vec<PageTex>,
    pub diags: Vec<Diagnostic>,
    pub sync: Vec<SyncEntry>,
    pub ms: u128,
    pub busy: bool,
    /// El último intento no compiló (las páginas son las del último que sí).
    pub failed: bool,
}

impl TypPreview {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, rx_q) = channel::<Request>();
        let (tx_r, rx) = channel::<Response>();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("typ-preview".into())
            .stack_size(64 << 20)
            .spawn(move || {
                let engine = Engine::new(vec![]);
                while let Ok(mut req) = rx_q.recv() {
                    while let Ok(newer) = rx_q.try_recv() {
                        req = newer;
                    }
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compile(&engine, &req)));
                    if let Ok(r) = r {
                        if tx_r.send(r).is_err() {
                            break;
                        }
                    }
                    ctx.request_repaint();
                }
            })
            .expect("hilo de la vista previa libre");
        Self { tx, rx, next_id: 0, sent: None, pending: None, shown: None, current: None, pages: Vec::new(), diags: Vec::new(), sync: Vec::new(), ms: 0, busy: false, failed: false }
    }

    pub fn stale(&self) -> bool {
        self.shown != self.current
    }

    /// Líneas (desde 0) con error en el propio documento.
    pub fn error_lines(&self) -> std::collections::HashSet<usize> {
        self.diags.iter().filter(|d| d.severity == instatypst_engine::Severity::Error && d.file.is_none()).filter_map(|d| d.line).map(|l| l.saturating_sub(1)).collect()
    }

    pub fn tick(&mut self, ctx: &egui::Context, doc: &TypDoc, target_px: f32) {
        while let Ok(resp) = self.rx.try_recv() {
            let Some((key, id)) = self.sent else { continue };
            if resp.id != id {
                continue;
            }
            self.busy = false;
            self.failed = resp.pages.is_none();
            if let Some(pages) = resp.pages {
                self.pages = pages
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let img = egui::ColorImage::from_rgba_unmultiplied([p.width, p.height], &p.rgba);
                        PageTex { tex: ctx.load_texture(format!("typ-page-{i}"), img, egui::TextureOptions::LINEAR), size_pt: p.size_pt }
                    })
                    .collect();
                self.sync = resp.sync;
            }
            self.diags = resp.diags;
            self.ms = resp.ms;
            self.shown = Some(key);
        }
        let key: Key = (doc.id, doc.rev, (target_px / 64.0).round().max(1.0) as u32);
        self.current = Some(key);
        if self.sent.map(|s| s.0) == Some(key) {
            self.pending = None;
            return;
        }
        let inmediato = self.sent.is_none_or(|(s, _)| s.0 != key.0 || s.2 != key.2);
        let desde = match self.pending {
            Some((k, t)) if k == key => t,
            _ => {
                self.pending = Some((key, Instant::now()));
                Instant::now()
            }
        };
        if !inmediato && desde.elapsed() < DEBOUNCE {
            ctx.request_repaint_after(DEBOUNCE - desde.elapsed());
            return;
        }
        self.next_id += 1;
        let req = Request { id: self.next_id, text: doc.text.clone(), root: doc.root(), filename: doc.filename(), target_px };
        if self.tx.send(req).is_ok() {
            self.sent = Some((key, self.next_id));
            self.pending = None;
            self.busy = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compila_varias_paginas_y_reporta_errores() {
        let e = Engine::new(vec![]);
        let r = compile(&e, &Request { id: 1, text: "= Uno\n#pagebreak()\n= Dos".into(), root: std::env::temp_dir(), filename: "main.typ".into(), target_px: 300.0 });
        assert_eq!(r.pages.as_ref().map(Vec::len), Some(2));
        let r = compile(&e, &Request { id: 2, text: "#foo(".into(), root: std::env::temp_dir(), filename: "main.typ".into(), target_px: 300.0 });
        assert!(r.pages.is_none());
        assert!(r.diags.iter().any(|d| d.line == Some(1)), "{:?}", r.diags);
    }
}
