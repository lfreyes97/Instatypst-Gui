//! Pestañas del dock, paneles y diseños predefinidos (como el gestor de Grammata).

use egui_dock::{DockState, Node, NodeIndex, NodePath, Split};
use egui_phosphor::regular as ic;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tab {
    Content,
    Files,
    Editor,
    Preview,
    Template,
    Style,
    Format,
    Color,
    Issues,
}

impl Tab {
    pub const ALL: [Tab; 9] = [
        Tab::Editor,
        Tab::Preview,
        Tab::Content,
        Tab::Files,
        Tab::Template,
        Tab::Style,
        Tab::Format,
        Tab::Color,
        Tab::Issues,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Tab::Content => "Contenido",
            Tab::Files => "Archivos",
            Tab::Editor => "Editor Typst",
            Tab::Preview => "Vista previa",
            Tab::Template => "Plantilla",
            Tab::Style => "Pieza",
            Tab::Format => "Tipografía",
            Tab::Color => "Color",
            Tab::Issues => "Incidencias",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Tab::Content => ic::ARTICLE,
            Tab::Files => ic::FOLDER,
            Tab::Editor => ic::FILE_CODE,
            Tab::Preview => ic::IMAGE,
            Tab::Template => ic::SQUARES_FOUR,
            Tab::Style => ic::SLIDERS_HORIZONTAL,
            Tab::Format => ic::TEXT_AA,
            Tab::Color => ic::PAINT_BUCKET,
            Tab::Issues => ic::WARNING_CIRCLE,
        }
    }

    pub fn title(self) -> String {
        format!("{} {}", self.icon(), self.name())
    }

    fn is_left(self) -> bool {
        matches!(self, Tab::Content | Tab::Files | Tab::Issues)
    }

    fn is_right(self) -> bool {
        matches!(self, Tab::Template | Tab::Style | Tab::Format | Tab::Color)
    }
}

/// Fracciones de los diseños predefinidos.
///
/// Se calculan para la ventana típica (≈1338 px de ancho → dock de ≈1314 px)
/// y escalan de forma proporcional. Objetivos en esa ventana:
/// - Estudio: contenido ≈230 · editor ≈410 · vista previa ≈260 · columna derecha ≈390.
/// - Clásico: contenido ≈260 · editor ≈540 · vista previa ≈500 (una página 4:5 a toda altura).
/// - Vista amplia: editor ≈395 · vista previa ≈520 · columna derecha ≈390.
///
/// La columna derecha apila 4 pestañas que piden ≈430 px; con menos, la barra
/// de pestañas se desplaza (egui_dock no permite estrecharlas más).
///
/// En `split_right` la fracción es la parte del nodo existente (izquierda);
/// en `split_left`, la del panel nuevo (izquierda).
mod frac {
    pub const STUDIO_MAIN: f32 = 0.46;
    pub const STUDIO_PREVIEW: f32 = 0.50;
    pub const CLASSIC_MAIN: f32 = 0.61;
    pub const CLASSIC_LEFT: f32 = 0.33;
    pub const FOCUS_EDITOR: f32 = 0.30;
    pub const FOCUS_PREVIEW: f32 = 0.57;
    /// Al volver a mostrar un panel: parte que ocupa el lateral izquierdo nuevo…
    pub const SHOW_LEFT: f32 = 0.36;
    /// …parte que conserva el editor al abrir la vista previa…
    pub const SHOW_PREVIEW: f32 = 0.55;
    /// …y parte que conserva el vecino al abrir un panel de la columna derecha.
    pub const SHOW_RIGHT: f32 = 0.60;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    /// [editor|contenido|archivos] · vista previa · [estilo|formato|color|plantilla].
    Studio,
    /// [contenido|archivos] · editor · vista previa.
    Classic,
    /// Editor estrecho · vista previa ancha · estilo.
    PreviewFocus,
    /// Solo el editor.
    Writing,
}

impl Preset {
    pub const ALL: [Preset; 4] = [Preset::Studio, Preset::Classic, Preset::PreviewFocus, Preset::Writing];

    pub fn label(self) -> &'static str {
        match self {
            Preset::Studio => "Estudio",
            Preset::Classic => "Clásico",
            Preset::PreviewFocus => "Vista amplia",
            Preset::Writing => "Escritura",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Preset::Studio => "Editor (con contenido y archivos) · vista previa · estilo",
            Preset::Classic => "Contenido · editor · vista previa",
            Preset::PreviewFocus => "Editor estrecho, vista previa ancha",
            Preset::Writing => "Solo el editor",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Preset::Studio => ic::SQUARES_FOUR,
            Preset::Classic => ic::SQUARE_SPLIT_HORIZONTAL,
            Preset::PreviewFocus => ic::IMAGE,
            Preset::Writing => ic::PENCIL_SIMPLE_LINE,
        }
    }

    pub fn build(self) -> DockState<Tab> {
        let first = match self {
            // Tres columnas (pantalla pequeña): contenido y archivos van como
            // pestañas junto al editor, que queda activo.
            Preset::Studio => vec![Tab::Editor, Tab::Content, Tab::Files, Tab::Issues],
            _ => vec![Tab::Editor],
        };
        let mut dock = DockState::new(first);
        let s = dock.main_surface_mut();
        match self {
            Preset::Studio => {
                let [_, right] = s.split_right(NodeIndex::root(), frac::STUDIO_MAIN, vec![Tab::Preview]);
                s.split_right(right, frac::STUDIO_PREVIEW, vec![Tab::Style, Tab::Format, Tab::Color, Tab::Template]);
            }
            Preset::Classic => {
                let [editor, _] = s.split_right(NodeIndex::root(), frac::CLASSIC_MAIN, vec![Tab::Preview]);
                s.split_left(editor, frac::CLASSIC_LEFT, vec![Tab::Content, Tab::Files, Tab::Issues]);
            }
            Preset::PreviewFocus => {
                let [_, right] = s.split_right(NodeIndex::root(), frac::FOCUS_EDITOR, vec![Tab::Preview]);
                s.split_right(right, frac::FOCUS_PREVIEW, vec![Tab::Style, Tab::Format, Tab::Color, Tab::Template]);
            }
            Preset::Writing => {}
        }
        dock
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LayoutAction {
    Preset(Preset),
    Toggle(Tab),
    /// Muestra el panel (si está oculto) y lo activa.
    Focus(Tab),
}

pub fn is_open(dock: &DockState<Tab>, tab: Tab) -> bool {
    dock.find_tab(&tab).is_some()
}

fn path_of(dock: &DockState<Tab>, tab: Tab) -> Option<NodePath> {
    dock.find_tab(&tab).map(|p| NodePath { surface: p.surface, node: p.node })
}

/// Muestra el panel si hace falta y lo deja como pestaña activa.
pub fn focus(dock: &mut DockState<Tab>, tab: Tab) {
    if dock.find_tab(&tab).is_none() {
        show(dock, tab);
    }
    if let Some(path) = dock.find_tab(&tab) {
        let _ = dock.set_active_tab(path);
    }
}

/// Muestra u oculta un panel; al mostrarlo lo coloca en su sitio natural.
pub fn toggle(dock: &mut DockState<Tab>, tab: Tab) {
    if let Some(path) = dock.find_tab(&tab) {
        // Nunca se deja el dock vacío.
        if dock.iter_all_tabs().count() > 1 {
            dock.remove_tab(path);
        }
        return;
    }
    show(dock, tab);
}

fn show(dock: &mut DockState<Tab>, tab: Tab) {
    if dock.iter_all_tabs().next().is_none() {
        dock.main_surface_mut().push_to_first_leaf(tab);
        return;
    }
    // Los paneles de un mismo lado se apilan como pestañas.
    let sibling = Tab::ALL
        .into_iter()
        .filter(|t| *t != tab && ((tab.is_left() && t.is_left()) || (tab.is_right() && t.is_right())))
        .find_map(|t| path_of(dock, t));
    if let Some(path) = sibling
        && let Node::Leaf(leaf) = &mut dock[path.surface][path.node]
    {
        leaf.append_tab(tab);
        let _ = leaf.set_active_tab(leaf.len() - 1);
        return;
    }
    let anchor = |dock: &DockState<Tab>, prefer: &[Tab]| {
        prefer.iter().find_map(|t| path_of(dock, *t)).or_else(|| {
            dock.iter_all_tabs().next().map(|(p, _)| NodePath { surface: p.surface, node: p.node })
        })
    };
    // `fraction` es la parte del hijo izquierdo/superior (en Left, el panel nuevo).
    let (at, split, frac) = match tab {
        t if t.is_left() => (anchor(dock, &[Tab::Editor, Tab::Preview]), Split::Left, frac::SHOW_LEFT),
        Tab::Preview => (anchor(dock, &[Tab::Editor]), Split::Right, frac::SHOW_PREVIEW),
        Tab::Editor => (anchor(dock, &[Tab::Preview]), Split::Left, 0.50),
        _ => (anchor(dock, &[Tab::Preview, Tab::Editor]), Split::Right, frac::SHOW_RIGHT),
    };
    match at {
        Some(at) => {
            dock.split(at, split, frac, Node::leaf(tab));
        }
        None => dock.main_surface_mut().push_to_first_leaf(tab),
    }
}
