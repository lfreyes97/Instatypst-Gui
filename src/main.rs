//! Instatypst GUI — maqueta (sin conexión al binario de Typst todavía).

mod axes;
mod blocks;
mod checker;
mod debug;
mod highlight;
mod layout;
mod panels;
mod preview;
mod project;
mod typdoc;
mod typpanels;
mod typst_editor;
mod theme;
mod widgets;

use layout::{LayoutAction, Preset, Tab};

use egui::{Frame, Margin};
use egui_dock::{DockArea, DockState, TabViewer};

struct Viewer<'a> {
    st: &'a mut State,
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(("dock-tab", *tab))
    }

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        egui::RichText::new(tab.title()).size(theme::dims::F_BODY).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        let st = &mut *self.st;
        match *tab {
            Tab::Content => panels::source(st, ui),
            Tab::Files => panels::files(st, ui),
            Tab::Editor => panels::editor(st, ui),
            Tab::Preview => panels::preview(st, ui),
            Tab::Template => axes::template(st, ui),
            Tab::Style => axes::piece(st, ui),
            Tab::Format => axes::typography(st, ui),
            Tab::Color => axes::palette(st, ui),
            Tab::Issues => panels::issues(st, ui),
        }
    }

    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        // Editor y vista previa gestionan su propio scroll.
        match tab {
            Tab::Editor | Tab::Preview => [false, false],
            _ => [false, true],
        }
    }
}

pub struct App {
    st: State,
    dock: DockState<Tab>,
}

#[derive(Clone, Copy)]
pub enum FileAction {
    NewTyp,
    Open,
    Save,
    SaveAs,
}

pub struct State {
    pub dark: bool,
    pub layout_actions: Vec<LayoutAction>,
    pub open: Vec<Tab>,
    pub layout_preset: Preset,
    pub panels_menu: bool,
    pub project: project::Project,
    /// Pieza activa (índice en `project.doc.posts`).
    pub piece: usize,
    pub status: Option<String>,
    pub file_action: Option<FileAction>,
    pub search_open: bool,
    pub search: String,
    pub tpl_search: String,
    /// Por estilo: (bloques que no admite, bloques que le faltan) para la pieza activa.
    pub tpl_fit: Option<((usize, u64), std::collections::BTreeMap<String, (usize, usize)>)>,
    /// Texto de la pieza activa en el editor, y de qué pieza/versión viene.
    pub editor_text: String,
    pub editor_key: Option<(usize, u64)>,
    pub editor_error: Option<String>,
    /// El catálogo de firmas (se carga una vez) y las incidencias del documento.
    pub catalog: instatypst_piece::model::Catalog,
    pub checker: Option<checker::CheckState>,
    pub preview: Option<preview::PreviewState>,
    pub issue_sel: Option<usize>,
    /// Modo debug de la lámina: capa de cajas, inspector y clic → bloque.
    pub debug: bool,
    pub block_sel: Option<usize>,
    /// Modo del editor: 0 bloques, 1 Typst, 2 la pieza como TOML.
    pub editor_mode: usize,
    pub blocks_note: Option<String>,
    pub typst_text: String,
    pub typst_key: Option<(usize, u64)>,
    pub typst_issues: Vec<instatypst_piece::model::Issue>,
    pub editor_goto: Option<usize>,
    pub typ_ed: Option<typst_editor::TypEditor>,
    /// Documentos `.typ` libres abiertos y cuál se edita (`None`: la pieza del contrato).
    pub typs: Vec<typdoc::TypDoc>,
    pub active_typ: Option<usize>,
    pub typ_preview: Option<typdoc::TypPreview>,
    pub typ_next_id: u64,
    /// Visor: todas las páginas con scroll (o una a una), página actual y salto pendiente.
    pub typ_scroll: bool,
    pub typ_page: usize,
    pub typ_jump: Option<usize>,
    pub typ_goto_line: Option<usize>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut st = State::new();
        if let Some(path) = std::env::args().nth(1) {
            st.open_path(std::path::Path::new(&path));
        }
        theme::set_dark(&cc.egui_ctx, st.dark);
        Self { st, dock: Preset::Studio.build() }
    }
}

impl State {
    /// Abre el documento de `path`; deja el motivo en `status` si falla.
    fn open_path(&mut self, path: &std::path::Path) {
        if path.extension().is_some_and(|e| e == "typ") {
            match typdoc::TypDoc::open(self.typ_next_id, path) {
                Ok(d) => self.add_typ(d),
                Err(e) => self.status = Some(e),
            }
            return;
        }
        match project::Project::open(path) {
            Ok(p) => {
                instatypst_bridge::colors::set_root(path.parent().map(|d| d.to_path_buf()));
                self.project = p;
                self.piece = 0;
                self.status = None;
                if let Some(c) = &mut self.checker {
                    c.reset();
                }
            }
            Err(e) => self.status = Some(e),
        }
    }

    /// Un documento Typst nuevo (sin ruta) con `text`.
    pub fn new_typ(&mut self, text: String) {
        let n = self.typs.iter().filter(|t| t.path.is_none()).count() + 1;
        let d = typdoc::TypDoc::new(self.typ_next_id, &format!("sin-titulo-{n}.typ"), text);
        self.add_typ(d);
    }

    /// Añade un documento `.typ` (o lo reutiliza si ya está abierto) y lo deja activo.
    pub fn add_typ(&mut self, d: typdoc::TypDoc) {
        if let Some(i) = d.path.as_ref().and_then(|p| self.typs.iter().position(|t| t.path.as_ref() == Some(p))) {
            self.active_typ = Some(i);
            return;
        }
        self.typ_next_id += 1;
        self.typs.push(d);
        self.active_typ = Some(self.typs.len() - 1);
        self.typ_page = 0;
        self.status = None;
        self.layout_actions.push(LayoutAction::Focus(Tab::Editor));
    }

    /// Clic en la lámina en `(x, y)` puntos: selecciona la caja y, si su texto es de un bloque,
    /// lleva el editor a ese bloque. Devuelve el bloque.
    fn debug_pick(&mut self, x: f32, y: f32) -> Option<usize> {
        let pv = self.preview.as_ref()?;
        let i = debug::hit(&pv.boxes, x, y)?;
        let texto = pv.boxes[i].text.clone()?;
        let spec = self.project.doc.merged(self.piece);
        let bloque = debug::block_for_text(&debug::blocks_of(&spec), &texto)?;
        self.block_sel = Some(bloque);
        self.editor_mode = 0;
        let path = format!("post[{}].content[{}]", self.piece, bloque);
        self.editor_goto = Some(project::line_of(&self.editor_text, &path));
        Some(bloque)
    }

    /// Las incidencias del documento (contrato, firmas y render); el informe llega de un hilo.
    fn issues(&self) -> &[checker::Finding] {
        self.checker.as_ref().map_or(&[], |c| &c.findings)
    }

    /// Pide el informe si el documento cambió y recoge el que haya terminado.
    fn refresh_issues(&mut self, ctx: &egui::Context) {
        let c = self.checker.get_or_insert_with(|| checker::CheckState::new(ctx));
        c.tick(ctx, &self.project);
    }

    fn run_file_action(&mut self) {
        let Some(action) = self.file_action.take() else { return };
        let filter = |d: rfd::FileDialog| d.add_filter("Instatypst y Typst", &["toml", "json", "yaml", "yml", "typ"]);
        // Con un `.typ` activo, guardar guarda ese archivo.
        if let Some(i) = self.active_typ {
            let result = match action {
                FileAction::Save if self.typs[i].path.is_some() => self.typs[i].save(),
                FileAction::Save | FileAction::SaveAs => match rfd::FileDialog::new().add_filter("Typst", &["typ"]).set_file_name(self.typs[i].name()).save_file() {
                    Some(path) => self.typs[i].save_as(&path),
                    None => return,
                },
                _ => Ok(()),
            };
            if matches!(action, FileAction::Save | FileAction::SaveAs) {
                self.status = result.err();
                return;
            }
        }
        let result = match action {
            FileAction::NewTyp => {
                self.new_typ(String::new());
                return;
            }
            FileAction::Open => {
                if let Some(path) = filter(rfd::FileDialog::new()).pick_file() {
                    self.open_path(&path);
                }
                return;
            }
            FileAction::Save if self.project.path.is_some() => self.project.save(),
            FileAction::Save | FileAction::SaveAs => match rfd::FileDialog::new().add_filter("Documento Instatypst", &["toml", "json", "yaml", "yml"]).set_file_name(self.project.file_name()).save_file() {
                Some(path) => self.project.save_as(&path),
                None => return,
            },
        };
        self.status = result.err();
    }

    fn new() -> Self {
        Self {
            dark: true,
            layout_actions: Vec::new(),
            open: Vec::new(),
            layout_preset: Preset::Studio,
            panels_menu: false,
            project: project::Project::new_empty(),
            piece: 0,
            status: None,
            file_action: None,
            search_open: false,
            search: String::new(),
            tpl_search: String::new(),
            tpl_fit: None,
            editor_text: String::new(),
            editor_key: None,
            editor_error: None,
            catalog: instatypst_piece::model::Catalog::embedded(),
            checker: None,
            preview: None,
            issue_sel: None,
            debug: false,
            block_sel: None,
            editor_mode: 0,
            blocks_note: None,
            typst_text: String::new(),
            typst_key: None,
            typst_issues: Vec::new(),
            editor_goto: None,
            typ_ed: None,
            typs: Vec::new(),
            active_typ: None,
            typ_preview: None,
            typ_next_id: 1,
            typ_scroll: false,
            typ_page: 0,
            typ_jump: None,
            typ_goto_line: None,
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(theme::pal().bg).to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        use theme::dims::{GUTTER, TOPBAR_H};
        let m = |l, r, t, b| Margin { left: l, right: r, top: t, bottom: b };

        use egui::{Key, Modifiers};
        let (open, save) = ui.input_mut(|i| (i.consume_key(Modifiers::COMMAND, Key::O), i.consume_key(Modifiers::COMMAND, Key::S)));
        if open {
            self.st.file_action = Some(FileAction::Open);
        } else if save {
            self.st.file_action = Some(FileAction::Save);
        }
        self.st.run_file_action();
        let ctx = ui.ctx().clone();
        self.st.refresh_issues(&ctx);
        self.st.piece = self.st.piece.min(self.st.project.doc.posts.len().saturating_sub(1));

        self.st.open = Tab::ALL.into_iter().filter(|t| layout::is_open(&self.dock, *t)).collect();

        egui::Panel::top("top")
            .exact_size(TOPBAR_H)
            .resizable(false)
            .frame(Frame::new().inner_margin(m(GUTTER, GUTTER, GUTTER, GUTTER / 2)))
            .show(ui, |ui| panels::top_bar(&mut self.st, ui));

        let style = theme::dock_style(ui.style());
        egui::CentralPanel::default()
            .frame(Frame::new().inner_margin(Margin::same(GUTTER)))
            .show(ui, |ui| {
                DockArea::new(&mut self.dock)
                    .style(style)
                    .show_add_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show_leaf_close_all_buttons(false)
                    .show_inside(ui, &mut Viewer { st: &mut self.st });
            });

        for action in std::mem::take(&mut self.st.layout_actions) {
            match action {
                LayoutAction::Preset(p) => {
                    self.dock = p.build();
                    self.st.layout_preset = p;
                }
                LayoutAction::Toggle(t) => layout::toggle(&mut self.dock, t),
                LayoutAction::Focus(t) => layout::focus(&mut self.dock, t),
            }
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Instatypst")
            .with_app_id("instatypst-gui")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([1180.0, 660.0]),
        ..Default::default()
    };
    eframe::run_native("Instatypst", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Espera a que el hilo de check termine el informe del documento actual.
    fn esperar_informe(ctx: &egui::Context, st: &mut State) {
        let t0 = std::time::Instant::now();
        loop {
            st.refresh_issues(ctx);
            if st.checker.as_ref().unwrap().up_to_date(&st.project) {
                return;
            }
            assert!(t0.elapsed().as_secs() < 60, "el informe no llegó");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    fn frame(ctx: &egui::Context, st: &mut State, mut input: egui::RawInput) {
        input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0)));
        let mut out = ctx.run_ui(input, |ui| panels::editor(st, ui));
        out.textures_delta.clear();
    }

    /// El editor carga la pieza al abrir, y al escribir en él cambia el documento.
    #[test]
    fn el_editor_carga_la_pieza_y_lo_que_se_escribe_llega_al_documento() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut st = State::new();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Documentos/GitHub/typst-rrss/instatypst-cli/tests/fixtures/contrato/valid/presets.toml");
        st.open_path(&fixture);
        st.editor_mode = 2; // la pieza como TOML
        assert!(st.status.is_none(), "{:?}", st.status);
        assert!(!st.project.doc.posts.is_empty());

        frame(&ctx, &mut st, egui::RawInput::default());
        assert!(st.editor_text.starts_with("[[post]]"), "{}", st.editor_text);
        assert_eq!(st.editor_text, st.project.piece_text(0));
        assert!(st.editor_error.is_none());

        // cambiar de pieza recarga el texto
        st.piece = 1;
        frame(&ctx, &mut st, egui::RawInput::default());
        assert_eq!(st.editor_text, st.project.piece_text(1));

        // escribir: se enfoca el campo y llega un evento de texto
        let antes = st.project.doc.clone();
        frame(&ctx, &mut st, egui::RawInput::default());
        let id = ctx.memory(|m| m.focused());
        if id.is_none() {
            // pulsa dentro del editor para enfocarlo
            let click = |pressed| egui::Event::PointerButton {
                pos: egui::pos2(200.0, 200.0),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            let mut input = egui::RawInput::default();
            input.events = vec![egui::Event::PointerMoved(egui::pos2(200.0, 200.0)), click(true), click(false)];
            frame(&ctx, &mut st, input);
        }
        let mut input = egui::RawInput::default();
        input.events = vec![egui::Event::Text("# nota\n".into())];
        frame(&ctx, &mut st, input);
        assert!(st.editor_text.contains("# nota\n"), "el texto no llegó al editor");

        // el modelo sigue al texto: pieza nueva si es válido, documento intacto y error si no
        match instatypst_piece::model::parse(&st.editor_text, instatypst_piece::model::Syntax::Toml) {
            Ok(d) => {
                assert!(st.editor_error.is_none(), "{:?}", st.editor_error);
                assert_eq!(st.project.doc.posts[1], d.posts[0]);
            }
            Err(_) => {
                assert!(st.editor_error.is_some());
                assert_eq!(st.project.doc, antes);
            }
        }
    }

    /// Un typo en `style` aparece como incidencia, se ve en el panel y en el editor, y se va al corregirlo.
    #[test]
    fn un_typo_de_estilo_aparece_y_desaparece_al_corregirlo_en_el_editor() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let dir = std::env::temp_dir().join(format!("insta-gui-diag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("d.toml");
        std::fs::write(&ruta, "schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"lisoo\"\ncontent = [ { type = \"body\", text = \"x\" } ]\n").unwrap();
        let mut st = State::new();
        st.open_path(&ruta);
        esperar_informe(&ctx, &mut st);
        assert_eq!(st.issues().len(), 1, "{:?}", st.issues());
        assert_eq!(st.issues()[0].code, "style_unknown");
        assert_eq!(st.issues()[0].path, "post[0].style");
        assert_eq!(st.issues()[0].piece, Some(0));
        assert!(st.issues()[0].error);

        // los dos paneles se dibujan con la incidencia presente
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0))), ..Default::default() }, |ui| {
            panels::issues(&mut st, ui);
        });
        out.textures_delta.clear();
        frame(&ctx, &mut st, egui::RawInput::default());

        // se corrige a través del texto del editor, como haría la persona
        let corregido = st.editor_text.replace("lisoo", "liso");
        assert_ne!(corregido, st.editor_text);
        st.editor_error = st.project.set_piece_text(st.piece, &corregido).err();
        st.editor_text = corregido;
        assert!(st.editor_error.is_none());
        esperar_informe(&ctx, &mut st);
        assert!(st.issues().is_empty(), "{:?}", st.issues());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// El panel de vista previa compila en su hilo, muestra la lámina y la recompila al cambiar el documento.
    #[test]
    fn la_vista_previa_compila_y_se_actualiza_al_cambiar_el_formato() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let dir = std::env::temp_dir().join(format!("insta-gui-prev-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("p.toml");
        std::fs::write(&ruta, "schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"liso\"\nformat = \"square\"\ncontent = [ { type = \"body\", text = \"Hola\" } ]\n").unwrap();
        let mut st = State::new();
        st.open_path(&ruta);

        // Dibuja fotogramas hasta que lo pedido esté mostrado (o se agote el tiempo).
        fn hasta_listo(ctx: &egui::Context, st: &mut State) {
            let t0 = std::time::Instant::now();
            loop {
                let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 700.0))), ..Default::default() };
                let mut out = ctx.run_ui(input, |ui| panels::preview(st, ui));
                out.textures_delta.clear();
                let pv = st.preview.as_ref().unwrap();
                if !pv.busy && !pv.stale() {
                    return;
                }
                assert!(t0.elapsed().as_secs() < 60, "la vista previa no terminó");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        hasta_listo(&ctx, &mut st);
        let pv = st.preview.as_ref().unwrap();
        assert!(pv.error.is_none(), "{:?}", pv.error);
        assert!(pv.texture.is_some());
        assert_eq!((pv.size_pt.0.round(), pv.size_pt.1.round()), (1080.0, 1080.0));

        // un control cambia el formato → la lámina se recompila con otra proporción
        let mut p = st.project.doc.posts[0].clone();
        p.format = Some(instatypst_piece::model::Format::Named(instatypst_piece::model::FormatName::Story));
        st.project.set_piece(0, p);
        hasta_listo(&ctx, &mut st);
        let pv = st.preview.as_ref().unwrap();
        assert!(pv.error.is_none(), "{:?}", pv.error);
        assert_eq!((pv.size_pt.0.round(), pv.size_pt.1.round()), (1080.0, 1920.0), "story = 1080×1920");

        // un estilo roto se muestra como error, sin tumbar la app
        let mut p = st.project.doc.posts[0].clone();
        p.style = Some(instatypst_piece::model::Style::Catalog("noexiste".into()));
        st.project.set_piece(0, p);
        hasta_listo(&ctx, &mut st);
        assert!(st.preview.as_ref().unwrap().error.is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Un contraste bajo sale como aviso (no como error) y se asigna a su pieza por el nombre.
    #[test]
    fn un_contraste_bajo_aparece_como_aviso_de_su_pieza() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let dir = std::env::temp_dir().join(format!("insta-gui-contraste-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("c.toml");
        std::fs::write(&ruta, "schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"bien\"\nstyle = \"liso\"\ncontent = [ { type = \"body\", text = \"x\" } ]\n[post.palette]\nbackground = \"#000000\"\ntext = \"#ffffff\"\naccent = \"#ffcc00\"\n\n[[post]]\nname = \"mal\"\nstyle = \"liso\"\ncontent = [ { type = \"body\", text = \"x\" } ]\n[post.palette]\nbackground = \"#777777\"\ntext = \"#7a7a7a\"\naccent = \"#808080\"\n").unwrap();
        let mut st = State::new();
        st.open_path(&ruta);
        esperar_informe(&ctx, &mut st);
        let avisos: Vec<_> = st.issues().iter().filter(|i| !i.error).collect();
        assert!(!avisos.is_empty(), "{:?}", st.issues());
        assert!(avisos.iter().all(|a| a.code == "contrast_low" && a.piece == Some(1)), "{avisos:?}");
        assert!(!st.issues().iter().any(|i| i.error), "un contraste bajo no es un error");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// El criterio de la fase 4: un clic en el epígrafe de una lámina `liso` selecciona el bloque
    /// `epigraph` y lleva el editor a su línea.
    #[test]
    fn un_clic_en_el_epigrafe_de_una_lamina_liso_selecciona_el_bloque_epigraph() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let dir = std::env::temp_dir().join(format!("insta-gui-debug-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("d.toml");
        std::fs::write(&ruta, "schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"liso\"\nformat = \"square\"\n\n[[post.content]]\ntype = \"epigraph\"\ntext = \"Una cita breve\"\nsource = \"Alguien\"\n\n[[post.content]]\ntype = \"body\"\ntext = \"El cuerpo de la lamina\"\n").unwrap();
        let mut st = State::new();
        st.open_path(&ruta);
        st.debug = true;
        let screen = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 800.0))), ..Default::default() };
        // el editor carga el texto de la pieza; la vista previa compila y trae las cajas
        let t0 = std::time::Instant::now();
        loop {
            let mut out = ctx.run_ui(screen(), |ui| { panels::editor(&mut st, ui); panels::preview(&mut st, ui); });
            out.textures_delta.clear();
            let pv = st.preview.as_ref().unwrap();
            if !pv.busy && !pv.stale() && !pv.boxes.is_empty() {
                break;
            }
            assert!(t0.elapsed().as_secs() < 60, "no compiló");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let boxes = st.preview.as_ref().unwrap().boxes.clone();
        let centro = |texto: &str| {
            let b = boxes.iter().find(|b| b.text.as_deref().is_some_and(|t| t.contains(texto))).unwrap_or_else(|| panic!("sin caja con {texto:?}: {:?}", boxes.iter().filter_map(|b| b.text.clone()).collect::<Vec<_>>()));
            (b.rect[0] + b.rect[2] / 2.0, b.rect[1] + b.rect[3] / 2.0)
        };
        // epígrafe → content[0]; cuerpo → content[1]
        let (x, y) = centro("cita");
        assert_eq!(st.debug_pick(x, y), Some(0), "el epígrafe es el bloque 0");
        assert_eq!(st.block_sel, Some(0));
        let linea = st.editor_goto.expect("lleva el editor a su línea");
        assert_eq!(st.editor_text.lines().nth(linea), Some("[[post.content]]"));
        assert_eq!(linea, st.editor_text.lines().position(|l| l == "[[post.content]]").unwrap(), "la primera tabla de contenido");
        let (x, y) = centro("cuerpo");
        assert_eq!(st.debug_pick(x, y), Some(1), "el cuerpo es el bloque 1");
        let segunda = st.editor_text.lines().enumerate().filter(|(_, l)| *l == "[[post.content]]").nth(1).unwrap().0;
        assert_eq!(st.editor_goto, Some(segunda));
        // un punto sin texto no selecciona nada
        assert_eq!(st.debug_pick(1.0, 1.0), None);

        // con el ratón encima de una caja y el modo debug, el panel se dibuja sin problema
        let mut input = screen();
        input.events = vec![egui::Event::PointerMoved(egui::pos2(300.0, 300.0))];
        let mut out = ctx.run_ui(input, |ui| panels::preview(&mut st, ui));
        out.textures_delta.clear();
        // y el goto se consume en el editor
        let mut out = ctx.run_ui(screen(), |ui| panels::editor(&mut st, ui));
        out.textures_delta.clear();
        assert_eq!(st.editor_goto, None);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Un `.typ` libre se edita, compila a todas sus páginas y el visor las navega.
    #[test]
    fn un_typ_libre_compila_a_varias_paginas() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut st = State::new();
        st.new_typ("= Uno\n#pagebreak()\n= Dos\n#pagebreak()\n= Tres".into());
        assert_eq!(st.active_typ, Some(0));
        let t0 = std::time::Instant::now();
        loop {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0)));
            let mut out = ctx.run_ui(input, |ui| {
                panels::editor(&mut st, ui);
                panels::preview(&mut st, ui);
            });
            out.textures_delta.clear();
            if st.typ_preview.as_ref().is_some_and(|p| p.pages.len() == 3 && !p.busy) {
                break;
            }
            assert!(t0.elapsed().as_secs() < 60, "no compiló");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(st.typ_preview.as_ref().unwrap().diags.is_empty());
    }
}
