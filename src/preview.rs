//! Vista previa: la pieza activa compilada con el motor de Typst (`instatypst-engine`), el mismo
//! renderizador que `typst compile`. Un hilo propio recibe peticiones; si llega otra mientras
//! compila, la vieja se descarta. El documento se espera 300 ms tras el último cambio.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use instatypst_bridge::render::render_pieza;
use instatypst_engine::{page_boxes, pixmap_to_rgba, render_page, BoxInfo, Engine, Severity};
use instatypst_piece::model::{Catalog, Document};

use crate::project::Project;

const DEBOUNCE: Duration = Duration::from_millis(300);

pub struct Request {
    pub id: u64,
    pub doc: Document,
    pub piece: usize,
    pub root: Option<PathBuf>,
    /// Lado mayor deseado de la imagen, en píxeles.
    pub target_px: f32,
}

pub struct Output {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    /// Tamaño de la página en puntos (= px del formato a 72 ppi).
    pub size_pt: (f32, f32),
    pub ms: u128,
    pub warnings: Vec<String>,
    /// Cajas de la página (texto, formas, imágenes) y color de fondo, para el modo debug.
    pub boxes: Vec<BoxInfo>,
    pub bg: Option<[u8; 3]>,
}

pub struct Response {
    pub id: u64,
    pub result: Result<Output, String>,
}

/// Compila la pieza `req.piece`: contrato → fuente Typst → página → píxeles.
pub fn compile(cat: &Catalog, engine: &mut Option<(Vec<PathBuf>, Engine)>, req: &Request) -> Result<Output, String> {
    let t0 = Instant::now();
    let (rendered, _post) = render_pieza(&req.doc, req.piece, cat).map_err(|e| format!("{e:#}"))?;
    let root = req.root.clone().unwrap_or_else(std::env::temp_dir);
    // Carpetas de fuentes del documento (`render.fonts`), relativas a la carpeta del documento.
    let dirs: Vec<PathBuf> = req
        .doc
        .render
        .as_ref()
        .map(|r| r.fonts.iter().map(|f| if Path::new(f).is_relative() { root.join(f) } else { PathBuf::from(f) }).filter(|p| p.exists()).collect())
        .unwrap_or_default();
    if engine.as_ref().is_none_or(|(d, _)| *d != dirs) {
        *engine = Some((dirs.clone(), Engine::new(dirs)));
    }
    let c = engine.as_ref().expect("motor").1.compile(&rendered.typ, &root, "preview.typ");
    let Some(doc) = c.doc else {
        let e = c.diags.iter().find(|d| d.severity == Severity::Error);
        return Err(match e {
            Some(d) => match d.line {
                Some(l) => format!("{} (línea {l} de la fuente generada)", d.message),
                None => d.message.clone(),
            },
            None => "no compila".to_string(),
        });
    };
    if doc.pages().len() != 1 {
        return Err(format!("la pieza produce {} páginas; una pieza es una sola imagen", doc.pages().len()));
    }
    let size = doc.pages()[0].frame.size();
    let (wpt, hpt) = (size.x.to_pt() as f32, size.y.to_pt() as f32);
    let scale = (req.target_px / wpt.max(hpt).max(1.0)).clamp(0.05, 4.0);
    let pix = render_page(&doc, 0, scale).ok_or("sin página")?;
    let (width, height, rgba) = pixmap_to_rgba(&pix);
    let mut warnings = rendered.avisos;
    warnings.extend(c.diags.iter().filter(|d| d.severity == Severity::Warning).map(|d| d.message.clone()));
    let (boxes, bg) = page_boxes(&doc, 0).unwrap_or_default();
    Ok(Output { width, height, rgba, size_pt: (wpt, hpt), ms: t0.elapsed().as_millis(), warnings, boxes, bg })
}

fn spawn(ctx: egui::Context) -> (Sender<Request>, Receiver<Response>) {
    let (tx, rx) = channel::<Request>();
    let (rtx, rrx) = channel::<Response>();
    std::thread::Builder::new()
        .name("preview".into())
        .spawn(move || {
            let cat = Catalog::embedded();
            let mut engine = None;
            while let Ok(mut req) = rx.recv() {
                while let Ok(newer) = rx.try_recv() {
                    req = newer; // cancela lo viejo: solo importa lo último
                }
                let result = compile(&cat, &mut engine, &req);
                if rtx.send(Response { id: req.id, result }).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        })
        .expect("hilo de la vista previa");
    (tx, rrx)
}

/// Qué se pidió: pieza, versión del documento y tamaño (a saltos de 64 px).
type Key = (usize, u64, u32);

pub struct PreviewState {
    tx: Sender<Request>,
    rx: Receiver<Response>,
    next_id: u64,
    sent: Option<(Key, u64)>,
    pending: Option<(Key, Instant)>,
    /// Lo que muestra la textura.
    pub shown: Option<Key>,
    pub texture: Option<egui::TextureHandle>,
    pub size_pt: (f32, f32),
    pub ms: u128,
    pub warnings: Vec<String>,
    pub boxes: Vec<BoxInfo>,
    pub bg: Option<[u8; 3]>,
    pub error: Option<String>,
    pub busy: bool,
    pub current: Option<Key>,
}

impl PreviewState {
    pub fn new(ctx: &egui::Context) -> Self {
        let (tx, rx) = spawn(ctx.clone());
        Self {
            tx,
            rx,
            next_id: 0,
            sent: None,
            pending: None,
            shown: None,
            texture: None,
            size_pt: (1.0, 1.0),
            ms: 0,
            warnings: Vec::new(),
            boxes: Vec::new(),
            bg: None,
            error: None,
            busy: false,
            current: None,
        }
    }

    /// La imagen no corresponde a lo que hay en el documento ahora.
    pub fn stale(&self) -> bool {
        self.shown != self.current
    }

    /// Recoge lo que terminó y pide lo que falta. Se llama en cada fotograma de la vista previa.
    pub fn tick(&mut self, ctx: &egui::Context, project: &Project, piece: usize, target_px: f32) {
        while let Ok(resp) = self.rx.try_recv() {
            let Some((key, id)) = self.sent else { continue };
            if resp.id != id {
                continue; // de una petición que ya quedó atrás
            }
            self.busy = false;
            match resp.result {
                Ok(o) => {
                    let img = egui::ColorImage::from_rgba_unmultiplied([o.width, o.height], &o.rgba);
                    self.texture = Some(ctx.load_texture("preview", img, egui::TextureOptions::LINEAR));
                    self.size_pt = o.size_pt;
                    self.ms = o.ms;
                    self.warnings = o.warnings;
                    self.boxes = o.boxes;
                    self.bg = o.bg;
                    self.error = None;
                }
                Err(e) => self.error = Some(e),
            }
            self.shown = Some(key);
        }
        if project.doc.posts.is_empty() || piece >= project.doc.posts.len() {
            self.current = None;
            return;
        }
        let key: Key = (piece, project.revision, (target_px / 64.0).round().max(1.0) as u32);
        self.current = Some(key);
        if self.sent.map(|s| s.0) == Some(key) {
            self.pending = None;
            return;
        }
        // Cambiar de pieza o de tamaño se pide ya; un cambio del documento espera 300 ms.
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
        let req = Request {
            id: self.next_id,
            doc: project.doc.clone(),
            piece,
            root: project.path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf),
            target_px,
        };
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
    use instatypst_piece::model::Syntax;

    const DOC: &str = "schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"liso\"\nformat = \"square\"\n[post.palette]\nbackground = \"#13294b\"\ntext = \"#f4efe6\"\naccent = \"#f2a14e\"\n[[post.content]]\ntype = \"body\"\ntext = \"Hola, mundo\"\n";

    fn cargar() -> (Document, Catalog) {
        let cat = Catalog::embedded();
        (instatypst_piece::model::load_checked(DOC, Syntax::Toml, &cat).unwrap(), cat)
    }

    #[test]
    fn compila_una_pieza_a_una_imagen_con_el_formato_del_contrato() {
        let (doc, cat) = cargar();
        let req = Request { id: 1, doc, piece: 0, root: None, target_px: 540.0 };
        let o = compile(&cat, &mut None, &req).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((o.size_pt.0.round(), o.size_pt.1.round()), (1080.0, 1080.0), "square = 1080×1080");
        assert_eq!((o.width, o.height), (540, 540));
        // el fondo de la paleta (#13294b) está en la esquina
        assert_eq!(&o.rgba[0..4], &[0x13, 0x29, 0x4b, 255]);
    }

    #[test]
    fn un_error_del_puente_o_de_typst_llega_como_mensaje() {
        let (mut doc, cat) = cargar();
        doc.posts[0].style = Some(instatypst_piece::model::Style::Catalog("noexiste".into()));
        let req = Request { id: 1, doc, piece: 0, root: None, target_px: 300.0 };
        assert!(compile(&cat, &mut None, &req).is_err());
        let (doc, cat) = cargar();
        assert!(compile(&cat, &mut None, &Request { id: 2, doc, piece: 9, root: None, target_px: 300.0 }).is_err());
    }

    /// El criterio de la fase 3: la vista previa es idéntica, píxel a píxel, a la imagen que da
    /// `typst compile` (lo que `instatypst build` lanza) para la misma fuente.
    #[test]
    fn la_vista_previa_es_identica_a_typst_compile() {
        let (doc, cat) = cargar();
        let (rendered, _) = render_pieza(&doc, 0, &cat).unwrap();
        let dir = std::env::temp_dir().join(format!("insta-gui-paridad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("p.typ"), &rendered.typ).unwrap();
        let st = std::process::Command::new("typst")
            .args(["compile", "--root"]).arg(&dir).args(["--ppi", "72"]).arg(dir.join("p.typ")).arg(dir.join("p.png"))
            .output().expect("typst en el PATH");
        assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));
        let cli = std::fs::read(dir.join("p.png")).unwrap();

        let c = Engine::new(vec![]).compile(&rendered.typ, &dir, "p.typ");
        let pix = render_page(c.doc.as_ref().expect("compila"), 0, 1.0).unwrap();
        let motor = pix.encode_png().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        // Se comparan los píxeles decodificados: el contenedor PNG puede variar en bytes de metadatos.
        let (a, b) = (tiny_skia::Pixmap::decode_png(&motor).unwrap(), tiny_skia::Pixmap::decode_png(&cli).unwrap());
        assert_eq!((a.width(), a.height()), (b.width(), b.height()), "tamaño");
        let distintos = a.data().iter().zip(b.data()).filter(|(x, y)| x != y).count();
        assert_eq!(distintos, 0, "{distintos} bytes de píxel distintos de {}", a.data().len());
    }
}
