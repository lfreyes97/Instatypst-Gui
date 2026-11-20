//! El informe de `check` (contrato, firmas y render, con los avisos de contraste) calculado en un
//! hilo propio con `instatypst_bridge::check::check_document`. Renderizar a Typst puede leer
//! imágenes (`from_image`), así que no se hace en el hilo de la interfaz.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use instatypst_bridge::check::{check_document, Severity};
use instatypst_piece::model::Document;

use crate::project::{piece_of, Project};

const DEBOUNCE: Duration = Duration::from_millis(300);

/// Una incidencia lista para la interfaz.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub error: bool,
    pub code: String,
    /// `post[2].content[1]`; vacío si es del documento entero o un aviso de render.
    pub path: String,
    /// Índice de la pieza, si se sabe (por la ruta o, si no, por el nombre).
    pub piece: Option<usize>,
    pub message: String,
}

/// El informe de un documento en memoria, con las piezas ya como índices.
pub fn findings(doc: &Document, nombre: &str) -> Vec<Finding> {
    let r = check_document(doc, nombre);
    r.findings
        .into_iter()
        .map(|f| {
            let piece = piece_of(&f.path).or_else(|| {
                let n = f.piece.as_deref()?;
                doc.posts.iter().position(|p| p.name.as_deref() == Some(n))
            });
            Finding { error: f.severity == Severity::Error, code: f.code, path: f.path, piece, message: f.message }
        })
        .collect()
}

struct Request {
    id: u64,
    doc: Document,
    name: String,
}

struct Response {
    id: u64,
    findings: Vec<Finding>,
}

pub struct CheckState {
    tx: Sender<Request>,
    rx: Receiver<Response>,
    next_id: u64,
    sent: Option<(u64, u64)>, // (revisión pedida, id)
    pending: Option<(u64, Instant)>,
    /// Revisión del documento a la que corresponde `findings`.
    pub done: Option<u64>,
    pub findings: Vec<Finding>,
}

impl CheckState {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, rx) = channel::<Request>();
        let (rtx, rrx) = channel::<Response>();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("check".into())
            .spawn(move || {
                while let Ok(mut req) = rx.recv() {
                    while let Ok(newer) = rx.try_recv() {
                        req = newer;
                    }
                    let findings = findings(&req.doc, &req.name);
                    if rtx.send(Response { id: req.id, findings }).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            })
            .expect("hilo de check");
        Self { tx, rx: rrx, next_id: 0, sent: None, pending: None, done: None, findings: Vec::new() }
    }

    /// El informe corresponde al documento actual.
    pub fn up_to_date(&self, project: &Project) -> bool {
        self.done == Some(project.revision)
    }

    /// Recoge lo que terminó y pide lo que falte (con 300 ms de espera tras el último cambio).
    pub fn tick(&mut self, ctx: &egui::Context, project: &Project) {
        while let Ok(resp) = self.rx.try_recv() {
            if let Some((rev, id)) = self.sent {
                if resp.id == id {
                    self.findings = resp.findings;
                    self.done = Some(rev);
                }
            }
        }
        let rev = project.revision;
        if self.sent.map(|s| s.0) == Some(rev) {
            return;
        }
        let primero = self.sent.is_none();
        let desde = match self.pending {
            Some((r, t)) if r == rev => t,
            _ => {
                self.pending = Some((rev, Instant::now()));
                Instant::now()
            }
        };
        if !primero && desde.elapsed() < DEBOUNCE {
            ctx.request_repaint_after(DEBOUNCE - desde.elapsed());
            return;
        }
        self.next_id += 1;
        let req = Request { id: self.next_id, doc: project.doc.clone(), name: project.file_name() };
        if self.tx.send(req).is_ok() {
            self.sent = Some((rev, self.next_id));
            self.pending = None;
        }
    }

    /// Fuerza que la próxima llamada a `tick` pida el informe de un documento recién abierto.
    pub fn reset(&mut self) {
        self.sent = None;
        self.pending = None;
        self.done = None;
        self.findings.clear();
    }
}
