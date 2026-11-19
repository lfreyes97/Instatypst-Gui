//! Instatypst GUI — maqueta (sin conexión al binario de Typst todavía).

mod format_bar;
mod highlight;
mod layout;
mod panels;
mod theme;
mod widgets;

use layout::{LayoutAction, Preset, Tab};

use egui::{Frame, Margin};
use egui_dock::{DockArea, DockState, TabViewer};

pub struct Block {
    pub icon: &'static str,
    pub title: &'static str,
    pub meta: &'static str,
}

pub struct Section {
    pub title: &'static str,
    pub icon: &'static str,
    pub open: bool,
    pub blocks: Vec<Block>,
}

pub struct Template {
    pub name: &'static str,
    pub desc: &'static str,
    pub colors: [egui::Color32; 3],
}

pub struct FileRow {
    pub icon: &'static str,
    pub name: &'static str,
    pub visible: bool,
    pub locked: bool,
}

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
            Tab::Template => panels::template(st, ui),
            Tab::Style => panels::style(st, ui),
            Tab::Format => format_bar::panel(st, ui),
            Tab::Color => format_bar::color_panel(st, ui),
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

pub struct State {
    pub dark: bool,
    pub fmt: format_bar::FormatState,
    pub layout_actions: Vec<LayoutAction>,
    pub open: Vec<Tab>,
    pub layout_preset: Preset,
    pub panels_menu: bool,
    pub sections: Vec<Section>,
    pub selected: (usize, usize),
    pub search_open: bool,
    pub search: String,
    pub templates: Vec<Template>,
    pub template: usize,
    pub preset: usize,
    pub quality: usize,
    pub ratio: usize,
    pub format: usize,
    pub files: Vec<FileRow>,
    pub file: usize,
    pub code: String,
    pub zoom: f32,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let st = State::new();
        theme::set_dark(&cc.egui_ctx, st.dark);
        Self { st, dock: Preset::Studio.build() }
    }
}

impl State {
    fn new() -> Self {
        use egui_phosphor::regular as ic;
        let c = egui::Color32::from_rgb;
        Self {
            dark: true,
            fmt: Default::default(),
            layout_actions: Vec::new(),
            open: Vec::new(),
            layout_preset: Preset::Studio,
            panels_menu: false,
            sections: vec![
                Section {
                    title: "Portada",
                    icon: ic::FRAME_CORNERS,
                    open: true,
                    blocks: vec![
                        Block { icon: ic::TEXT_H, title: "La regla de los 90 minutos", meta: "Título · 64 pt" },
                        Block { icon: ic::TEXT_ALIGN_LEFT, title: "Cómo organizo mi día de trabajo", meta: "Subtítulo · 28 pt" },
                        Block { icon: ic::IMAGE, title: "fondo-papel.jpg", meta: "Imagen · cubrir" },
                    ],
                },
                Section {
                    title: "Contenido",
                    icon: ic::ARTICLE,
                    open: true,
                    blocks: vec![
                        Block { icon: ic::QUOTES, title: "Elige una tarea, fija 90 minutos…", meta: "Cita · 36 pt" },
                        Block { icon: ic::LIST_BULLETS, title: "Tres reglas para enfocarte", meta: "Lista · 3 ítems" },
                        Block { icon: ic::TABLE, title: "ritmo-semanal.csv", meta: "Tabla · 5×4" },
                    ],
                },
            ],
            selected: (0, 0),
            search_open: false,
            search: String::new(),
            templates: vec![
                Template { name: "Cartel — Cálido", desc: "Título grande, mucho aire", colors: [c(0xf4, 0xef, 0xe6), c(0x1b, 0x1b, 0x1b), c(0xf2, 0xa1, 0x4e)] },
                Template { name: "Tarjeta — Editorial", desc: "Serif, regla superior", colors: [c(0xff, 0xff, 0xff), c(0x11, 0x11, 0x11), c(0x2b, 0x59, 0xff)] },
                Template { name: "Cita — Noche", desc: "Contraste alto, centrado", colors: [c(0x10, 0x10, 0x10), c(0xf2, 0xf2, 0xf2), c(0xe5, 0x48, 0x4d)] },
            ],
            template: 0,
            preset: 0,
            quality: 1,
            ratio: 1,
            format: 0,
            files: vec![
                FileRow { icon: ic::FILE_CODE, name: "main.typ", visible: true, locked: false },
                FileRow { icon: ic::PAINT_BRUSH, name: "estilo.typ", visible: true, locked: false },
                FileRow { icon: ic::DATABASE, name: "datos.json", visible: true, locked: true },
            ],
            file: 0,
            code: SAMPLE.into(),
            zoom: 0.6,
        }
    }
}

const SAMPLE: &str = r##"// Cartel 4:5 — Enfoca tu día
#import "estilo.typ": acento, tinta

#set page(width: 1080pt, height: 1350pt, margin: 96pt, fill: rgb("#f4efe6"))
#set text(font: "Inter", size: 30pt, fill: tinta)

#text(size: 22pt, tracking: 4pt, fill: acento)[ENFOQUE · 90 MIN]

= La regla de los 90 minutos

Elige una tarea, fija un temporizador y
trabaja sin interrupciones hasta que suene.

#v(1fr)
#align(right)[@usuario]
"##;

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from(theme::pal().bg).to_array()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        use theme::dims::{GUTTER, TOPBAR_H};
        let m = |l, r, t, b| Margin { left: l, right: r, top: t, bottom: b };

        self.st.open = Tab::ALL.into_iter().filter(|t| layout::is_open(&self.dock, *t)).collect();

        egui::Panel::top("top")
            .exact_size(TOPBAR_H)
            .resizable(false)
            .frame(Frame::new().inner_margin(m(GUTTER, GUTTER, GUTTER, 0)))
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
