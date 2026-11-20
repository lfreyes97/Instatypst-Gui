//! Un documento abierto: el modelo del contrato (`instatypst-piece`), su archivo
//! y cómo guardarlo sin destruir lo que la persona escribió a mano.
//!
//! Para TOML, guardar reaprovecha el texto original de cada pieza que no cambió
//! (comentarios, orden, espacios) y solo reescribe las que sí. JSON y YAML se
//! escriben enteros: no guardan comentarios.

use std::path::{Path, PathBuf};

use instatypst_piece::model::{self, Block, Catalog, Content, Document, Issue, PieceSpec, Syntax, SCHEMA};
use toml_edit::{DocumentMut, Item, Table};

pub struct Project {
    pub path: Option<PathBuf>,
    pub syntax: Syntax,
    pub doc: Document,
    /// Lo último que hay en disco (o lo vacío, si es nuevo).
    original_text: String,
    original: Document,
    /// De dónde viene cada pieza actual en `original.posts` (`None` = nueva).
    origin: Vec<Option<usize>>,
    /// Sube cada vez que cambian las piezas por fuera del editor de texto
    /// (abrir, crear, mover, borrar…): el editor sabe así que debe recargar.
    pub generation: u64,
    /// Sube con cada cambio del documento (también al escribir en el editor).
    pub revision: u64,
}

impl Project {
    pub fn new_empty() -> Self {
        let doc = Document {
            schema: Some(SCHEMA.to_string()),
            defaults: None,
            render: None,
            brand: None,
            palettes: Default::default(),
            posts: Vec::new(),
        };
        Self {
            path: None,
            syntax: Syntax::Toml,
            original: doc.clone(),
            original_text: String::new(),
            origin: Vec::new(),
            generation: 0,
            revision: 0,
            doc,
        }
    }

    /// Abre un `.toml`, `.json` o `.yaml`. Solo falla si no se puede leer; un
    /// documento con errores de contrato se abre igual (los muestra el diagnóstico).
    pub fn open(path: &Path) -> Result<Self, String> {
        let syntax = path
            .to_str()
            .and_then(Syntax::from_extension)
            .ok_or_else(|| format!("{}: extensión no reconocida (.toml, .json, .yaml)", path.display()))?;
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let doc = model::parse(&text, syntax).map_err(|e| format!("{}: {e:#}", path.display()))?;
        Ok(Self {
            path: Some(path.to_path_buf()),
            syntax,
            origin: (0..doc.posts.len()).map(Some).collect(),
            original: doc.clone(),
            original_text: text,
            generation: 0,
            revision: 0,
            doc,
        })
    }

    pub fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sin título.toml".to_string())
    }

    pub fn is_modified(&self) -> bool {
        self.doc != self.original
    }

    // ── Piezas ──────────────────────────────────────────────

    /// Título de la lista: `name`, o el primer título/epígrafe/cita, o «Pieza N».
    pub fn label(&self, i: usize) -> String {
        let p = &self.doc.posts[i];
        if let Some(n) = p.name.as_deref().filter(|n| !n.trim().is_empty()) {
            return n.to_string();
        }
        if let Some(Content::Blocks(bs)) = &p.content {
            for b in bs {
                if let Block::Title { text } | Block::Quote { text } | Block::Epigraph { text, .. } = b {
                    return text.lines().next().unwrap_or("").to_string();
                }
            }
        }
        format!("Pieza {}", i + 1)
    }

    /// La pieza `i` tal como se escribe en el archivo: su tabla `[[post]]`.
    /// Es lo que muestra el editor de texto (el contenido en Typst llega en la fase 6).
    pub fn piece_text(&self, i: usize) -> String {
        let solo = Document {
            schema: None,
            defaults: None,
            render: None,
            brand: None,
            palettes: Default::default(),
            posts: vec![self.doc.posts[i].clone()],
        };
        let texto = model::write(&solo, Syntax::Toml).unwrap_or_default();
        let desde = texto.find("[[post]]").unwrap_or(0);
        texto[desde..].trim_end().to_string() + "\n"
    }

    /// Reemplaza la pieza `i` (lo hacen los controles de los ejes). El editor de
    /// texto se recarga con el valor nuevo.
    pub fn set_piece(&mut self, i: usize, spec: PieceSpec) {
        if i < self.doc.posts.len() && self.doc.posts[i] != spec {
            self.doc.posts[i] = spec;
            self.generation += 1;
            self.revision += 1;
        }
    }

    /// Lee el texto de una pieza y, si es válido, reemplaza la pieza `i`.
    /// `Err` lleva el motivo; el documento no cambia.
    pub fn set_piece_text(&mut self, i: usize, texto: &str) -> Result<(), String> {
        if i >= self.doc.posts.len() {
            return Err(format!("la pieza {} no existe", i + 1));
        }
        let doc = model::parse(texto, Syntax::Toml).map_err(|e| format!("{e:#}"))?;
        let mut posts = doc.posts;
        if posts.len() != 1 {
            return Err(format!("debe haber exactamente una pieza ([[post]]), hay {}", posts.len()));
        }
        let nueva = posts.remove(0);
        if self.doc.posts[i] != nueva {
            self.doc.posts[i] = nueva;
            self.revision += 1;
        }
        Ok(())
    }

    /// Las incidencias del contrato: la estructura y la pieza frente a la firma de su estilo.
    /// Son las mismas funciones que usa `instatypst check`.
    pub fn diagnose(&self, cat: &Catalog) -> Vec<Issue> {
        let mut v = model::validate(&self.doc);
        v.extend(model::validate_semantic(&self.doc, cat));
        v
    }

    /// «estilo · formato» de la pieza, ya con lo que hereda de `defaults`.
    pub fn meta(&self, i: usize) -> String {
        use model::{Format, Style};
        let m = self.doc.merged(i);
        let style = match &m.style {
            Some(Style::Catalog(s)) => s.clone(),
            Some(Style::Custom(c)) => format!("{} (propio)", c.function),
            None => "sin estilo".to_string(),
        };
        let format = match &m.format {
            Some(Format::Named(n)) => format!("{n:?}").to_lowercase(),
            Some(Format::Custom(c)) => format!("{}×{}", c.width, c.height),
            None => "formato por defecto".to_string(),
        };
        format!("{style} · {format}")
    }

    pub fn add_piece(&mut self) -> usize {
        self.doc.posts.push(PieceSpec {
            content: Some(Content::Blocks(Vec::new())),
            ..Default::default()
        });
        self.origin.push(None);
        self.generation += 1;
        self.revision += 1;
        self.doc.posts.len() - 1
    }

    /// Duplica la pieza `i` justo debajo; devuelve el índice de la copia.
    pub fn duplicate(&mut self, i: usize) -> usize {
        if i >= self.doc.posts.len() {
            return i;
        }
        let mut copy = self.doc.posts[i].clone();
        if let Some(n) = copy.name.clone() {
            // Los nombres de pieza no pueden repetirse (`name_duplicate`).
            let mut k = 1;
            let nuevo = loop {
                let c = if k == 1 { format!("{n} (copia)") } else { format!("{n} (copia {k})") };
                if !self.doc.posts.iter().any(|p| p.name.as_deref() == Some(&c)) {
                    break c;
                }
                k += 1;
            };
            copy.name = Some(nuevo);
        }
        self.doc.posts.insert(i + 1, copy);
        self.origin.insert(i + 1, None);
        self.generation += 1;
        self.revision += 1;
        i + 1
    }

    pub fn delete(&mut self, i: usize) {
        if i >= self.doc.posts.len() {
            return;
        }
        self.doc.posts.remove(i);
        self.origin.remove(i);
        self.generation += 1;
        self.revision += 1;
    }

    /// Mueve la pieza `i` una posición (`-1` arriba, `1` abajo); devuelve su índice nuevo.
    pub fn move_piece(&mut self, i: usize, dir: isize) -> usize {
        let j = i as isize + dir;
        if i >= self.doc.posts.len() || j < 0 || j as usize >= self.doc.posts.len() {
            return i;
        }
        let j = j as usize;
        self.doc.posts.swap(i, j);
        self.origin.swap(i, j);
        self.generation += 1;
        self.revision += 1;
        j
    }

    // ── Guardar ─────────────────────────────────────────────

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.clone().ok_or("el documento no tiene archivo: usa «Guardar como»")?;
        self.save_as(&path)
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), String> {
        let syntax = path
            .to_str()
            .and_then(Syntax::from_extension)
            .ok_or_else(|| format!("{}: extensión no reconocida", path.display()))?;
        // Cambiar de sintaxis no puede reutilizar el texto original.
        let text = if syntax == Syntax::Toml && self.syntax == Syntax::Toml {
            self.render_toml()?
        } else {
            model::write(&self.doc, syntax).map_err(|e| format!("{e:#}"))? + "\n"
        };
        write_atomic(path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
        self.path = Some(path.to_path_buf());
        self.syntax = syntax;
        self.original = self.doc.clone();
        self.original_text = text;
        self.origin = (0..self.doc.posts.len()).map(Some).collect();
        Ok(())
    }

    /// El TOML del documento actual, conservando el formato de lo que no cambió.
    pub fn render_toml(&self) -> Result<String, String> {
        if self.original_text.trim().is_empty() {
            return Ok(model::write(&self.doc, Syntax::Toml).map_err(|e| format!("{e:#}"))? + "\n");
        }
        if !self.is_modified() {
            return Ok(self.original_text.clone());
        }
        let mut out: DocumentMut = self.original_text.parse().map_err(|e| format!("TOML original: {e}"))?;
        let fresh_text = model::write(&self.doc, Syntax::Toml).map_err(|e| format!("{e:#}"))?;
        let fresh: DocumentMut = fresh_text.parse().map_err(|e| format!("TOML nuevo: {e}"))?;

        // Claves de documento: se reescriben solo si cambiaron.
        let (a, b) = (&self.original, &self.doc);
        let changed = [
            ("schema", a.schema != b.schema),
            ("defaults", a.defaults != b.defaults),
            ("render", a.render != b.render),
            ("brand", a.brand != b.brand),
            ("palettes", a.palettes != b.palettes),
        ];
        for (key, differs) in changed {
            if !differs {
                continue;
            }
            match (fresh.get(key), out.get(key)) {
                // Dos tablas: solo se tocan las claves distintas (conserva comentarios).
                (Some(Item::Table(nueva)), Some(Item::Table(vieja))) => {
                    let parcheada = patch_table(vieja, nueva);
                    out.insert(key, Item::Table(parcheada));
                }
                (Some(item), _) => {
                    out.insert(key, item.clone());
                }
                (None, _) => {
                    out.remove(key);
                }
            }
        }

        // Piezas: la tabla original si no cambió; la nueva si cambió o no existía.
        let originals: Vec<Table> = out
            .get("post")
            .and_then(Item::as_array_of_tables)
            .map(|aot| aot.iter().cloned().collect())
            .unwrap_or_default();
        let fresh_posts: Vec<Table> = fresh
            .get("post")
            .and_then(Item::as_array_of_tables)
            .map(|aot| aot.iter().cloned().collect())
            .unwrap_or_default();
        let inline: Option<Vec<toml_edit::Value>> = match out.get("post") {
            Some(Item::Value(toml_edit::Value::Array(a))) => Some(a.iter().cloned().collect()),
            _ => None,
        };
        if self.doc.posts.is_empty() {
            out.remove("post");
        } else if let Some(originals) = inline {
            // `post = [ {...}, {...} ]`: se rehace el array reutilizando cada elemento intacto.
            let mut arr = toml_edit::Array::new();
            for (i, spec) in self.doc.posts.iter().enumerate() {
                let sin_cambios = self.origin[i].is_some_and(|j| self.original.posts.get(j) == Some(spec));
                let viejo = self.origin[i].and_then(|j| originals.get(j));
                arr.push_formatted(match viejo {
                    Some(v) if sin_cambios => v.clone(),
                    _ => toml_edit::Value::InlineTable(fresh_posts[i].clone().into_inline_table()),
                });
            }
            out.insert("post", Item::Value(toml_edit::Value::Array(arr)));
        } else {
            let mut aot = toml_edit::ArrayOfTables::new();
            for (i, spec) in self.doc.posts.iter().enumerate() {
                let orig_table = self.origin[i].and_then(|j| originals.get(j));
                let sin_cambios = self.origin[i].is_some_and(|j| self.original.posts.get(j) == Some(spec));
                aot.push(match orig_table {
                    Some(t) if sin_cambios => t.clone(),
                    // Cambió: se parte de la tabla original y solo se tocan las claves distintas.
                    Some(t) => patch_table(t, &fresh_posts[i]),
                    None => fresh_posts[i].clone(),
                });
            }
            out.insert("post", Item::ArrayOfTables(aot));
        }
        // Las posiciones de las tablas decidieron el orden al escribir: se
        // renumeran siguiendo el orden de las claves.
        let mut n = 0;
        renumber(out.as_table_mut(), &mut n);
        let texto = out.to_string();
        // Las partes nuevas salen con LF: el archivo conserva los finales de línea que tenía.
        Ok(if self.original_text.contains("\r\n") {
            texto.replace("\r\n", "\n").replace('\n', "\r\n")
        } else {
            texto
        })
    }
}

/// Escribe a un temporal y renombra: un fallo a mitad no trunca el original.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("instatypst-tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// `orig` con solo las claves de `fresh` que difieren; el resto conserva su formato.
fn patch_table(orig: &Table, fresh: &Table) -> Table {
    let mut out = orig.clone();
    let stale: Vec<String> = orig.iter().map(|(k, _)| k.to_string()).filter(|k| !fresh.contains_key(k)).collect();
    for k in stale {
        out.remove(&k);
    }
    for (k, new_item) in fresh.iter() {
        match orig.get(k) {
            Some(old) if same_value(k, old, new_item) => {}
            // Si la persona lo escribió en línea, el valor nuevo también va en línea.
            Some(old) if old.is_value() => {
                out.insert(k, new_item.clone().into_value().map(Item::Value).unwrap_or_else(|_| new_item.clone()));
            }
            _ => {
                out.insert(k, new_item.clone());
            }
        }
    }
    out
}

/// ¿Dicen lo mismo, aunque estén escritos distinto (en línea o en tabla)?
fn same_value(key: &str, a: &Item, b: &Item) -> bool {
    let norm = |item: &Item| -> Option<toml::Table> {
        let mut d = DocumentMut::new();
        d.insert(key, item.clone());
        toml::from_str(&d.to_string()).ok()
    };
    matches!((norm(a), norm(b)), (Some(x), Some(y)) if x == y)
}

/// Índice de pieza de una ruta `post[2].content[1]` (`None` si es del documento entero).
pub fn piece_of(path: &str) -> Option<usize> {
    let rest = path.strip_prefix("post[")?;
    rest.split_once(']')?.0.parse().ok()
}

/// Línea (0-based) del texto de una pieza (`piece_text`) a la que apunta `path`.
/// `post[2].content[1]` → el 2.º `[[post.content]]` si hay, si no la línea de `content`.
pub fn line_of(text: &str, path: &str) -> usize {
    let campo = path.split_once("].").map(|(_, r)| r).unwrap_or("");
    let clave = campo.split(['.', '[']).next().unwrap_or("");
    let indice = campo.split_once('[').and_then(|(_, r)| r.split_once(']')).and_then(|(n, _)| n.parse::<usize>().ok());
    let lineas: Vec<&str> = text.lines().collect();
    if clave.is_empty() {
        return 0;
    }
    let tabla = format!("[[post.{clave}]]");
    if let Some(i) = indice {
        if let Some(n) = lineas.iter().enumerate().filter(|(_, l)| l.trim() == tabla).map(|(n, _)| n).nth(i) {
            return n;
        }
    }
    lineas
        .iter()
        .position(|l| {
            let l = l.trim_start();
            l.strip_prefix(clave).is_some_and(|r| r.trim_start().starts_with('=') || r.starts_with('.'))
                || l == format!("[post.{clave}]")
                || l == tabla
        })
        .unwrap_or(0)
}

fn renumber(t: &mut Table, n: &mut usize) {
    for (_, item) in t.iter_mut() {
        match item {
            Item::Table(sub) => {
                sub.set_position(*n as isize);
                *n += 1;
                renumber(sub, n);
            }
            Item::ArrayOfTables(aot) => {
                for sub in aot.iter_mut() {
                    sub.set_position(*n as isize);
                    *n += 1;
                    renumber(sub, n);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EJEMPLO: &str = r##"# Mi serie
schema = "instatypst-piece/v1"

[defaults]
style = "liso"   # el de siempre

# La primera
[[post]]
name = "uno"
content = [ { type = "title", text = "Uno" } ]

# La segunda, con comentario
[[post]]
name = "dos"
content = [ { type = "title", text = "Dos" } ]

[[post]]
name = "tres"
content = [ { type = "title", text = "Tres" } ]
"##;

    fn abrir(texto: &str) -> (Project, PathBuf) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("insta-gui-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("serie.toml");
        std::fs::write(&ruta, texto).unwrap();
        (Project::open(&ruta).unwrap(), ruta)
    }

    fn titulo(p: &mut PieceSpec, t: &str) {
        p.content = Some(Content::Blocks(vec![Block::Title { text: t.into() }]));
    }

    #[test]
    fn sin_cambios_el_archivo_queda_igual() {
        let (mut p, ruta) = abrir(EJEMPLO);
        assert!(!p.is_modified());
        p.save().unwrap();
        assert_eq!(std::fs::read_to_string(ruta).unwrap(), EJEMPLO);
    }

    #[test]
    fn tocar_una_pieza_solo_cambia_sus_lineas() {
        let (mut p, ruta) = abrir(EJEMPLO);
        titulo(&mut p.doc.posts[1], "Dos, corregido");
        assert!(p.is_modified());
        p.save().unwrap();
        let nuevo = std::fs::read_to_string(ruta).unwrap();
        let distintas: Vec<_> = EJEMPLO.lines().zip(nuevo.lines()).filter(|(a, b)| a != b).collect();
        assert_eq!(EJEMPLO.lines().count(), nuevo.lines().count(), "{nuevo}");
        assert_eq!(distintas.len(), 1, "{distintas:?}\n{nuevo}");
        assert!(nuevo.contains("# Mi serie") && nuevo.contains("# el de siempre") && nuevo.contains("# La primera"));
        assert!(nuevo.contains("Dos, corregido"));
    }

    #[test]
    fn reordenar_borrar_y_duplicar_conservan_lo_que_no_cambia() {
        let (mut p, ruta) = abrir(EJEMPLO);
        p.move_piece(2, -1); // tres sube sobre dos
        p.delete(0); // fuera «uno»
        let copia = p.duplicate(0); // copia de «tres»
        assert_eq!(copia, 1);
        p.save().unwrap();
        let nuevo = std::fs::read_to_string(&ruta).unwrap();
        let de_vuelta = Project::open(&ruta).unwrap();
        let nombres: Vec<_> = de_vuelta.doc.posts.iter().map(|x| x.name.clone().unwrap()).collect();
        assert_eq!(nombres, ["tres", "tres (copia)", "dos"], "{nuevo}");
        assert!(nuevo.contains("# La segunda, con comentario"), "{nuevo}");
        assert!(!nuevo.contains("# La primera"), "{nuevo}");
        assert!(nuevo.contains("# Mi serie"), "{nuevo}");
    }

    #[test]
    fn cambiar_los_defaults_no_toca_las_piezas() {
        let (mut p, ruta) = abrir(EJEMPLO);
        p.doc.defaults.as_mut().unwrap().variant = Some("oscuro".into());
        p.save().unwrap();
        let nuevo = std::fs::read_to_string(ruta).unwrap();
        assert!(nuevo.contains("oscuro"));
        assert!(nuevo.contains("# La segunda, con comentario"));
        assert!(nuevo.contains(r#"content = [ { type = "title", text = "Dos" } ]"#), "{nuevo}");
    }

    #[test]
    fn el_texto_de_una_pieza_va_y_vuelve_y_llega_al_documento() {
        let (mut p, ruta) = abrir(EJEMPLO);
        let texto = p.piece_text(1);
        assert!(texto.starts_with("[[post]]") && texto.contains("Dos"), "{texto}");
        // sin tocar nada, la pieza no cambia
        p.set_piece_text(1, &texto).unwrap();
        assert!(!p.is_modified());
        // editar el texto cambia el modelo y se guarda
        p.set_piece_text(1, &texto.replace("Dos", "Dos editado")).unwrap();
        assert!(p.is_modified());
        p.save().unwrap();
        assert!(std::fs::read_to_string(ruta).unwrap().contains("Dos editado"));
        // un texto inválido no toca el documento
        let antes = p.doc.clone();
        assert!(p.set_piece_text(1, "[[post]]\nstyle = [").is_err());
        assert!(p.set_piece_text(1, "name = \"sin post\"").is_err());
        assert_eq!(p.doc, antes);
    }

    #[test]
    fn un_documento_nuevo_se_guarda_y_se_vuelve_a_abrir() {
        let mut p = Project::new_empty();
        let i = p.add_piece();
        titulo(&mut p.doc.posts[i], "Hola");
        assert_eq!(p.label(i), "Hola");
        let ruta = std::env::temp_dir().join(format!("insta-gui-nuevo-{}.toml", std::process::id()));
        p.save_as(&ruta).unwrap();
        let q = Project::open(&ruta).unwrap();
        assert_eq!(q.doc, p.doc);
        assert!(!q.is_modified());
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn las_fixtures_del_contrato_se_guardan_sin_cambios() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Documentos/GitHub/typst-rrss/instatypst-cli/tests/fixtures/contrato/valid");
        let mut vistas = 0;
        for e in std::fs::read_dir(&dir).expect("fixtures").flatten() {
            let ruta = e.path();
            if ruta.extension().is_none_or(|x| x != "toml") {
                continue;
            }
            let original = std::fs::read_to_string(&ruta).unwrap();
            let p = Project::open(&ruta).unwrap_or_else(|e| panic!("{}: {e}", ruta.display()));
            assert_eq!(p.render_toml().unwrap(), original, "{}", ruta.display());
            vistas += 1;
        }
        assert!(vistas >= 3, "solo {vistas} fixtures");
    }

    #[test]
    fn tocar_una_fixture_real_da_un_diff_minimo() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../Documentos/GitHub/typst-rrss/instatypst-cli/tests/fixtures/contrato/valid");
        let ruta = dir.join("presets.toml");
        let original = std::fs::read_to_string(&ruta).unwrap();
        let mut p = Project::open(&ruta).unwrap();
        let ultima = p.doc.posts.len() - 1;
        p.doc.posts[ultima].name = Some("renombrada".into());
        let nuevo = p.render_toml().unwrap();
        let distintas = original.lines().zip(nuevo.lines()).filter(|(a, b)| a != b).count();
        assert!(distintas <= 2 && original.lines().count().abs_diff(nuevo.lines().count()) <= 1, "{distintas} líneas distintas\n{nuevo}");
    }

    // ── Fallos señalados por la revisión: cada test dice si es real ──

    #[test]
    fn rev1_comentario_dentro_de_defaults_sobrevive_a_un_cambio_en_defaults() {
        let (mut p, ruta) = abrir(EJEMPLO);
        p.doc.defaults.as_mut().unwrap().variant = Some("oscuro".into());
        p.save().unwrap();
        let nuevo = std::fs::read_to_string(ruta).unwrap();
        assert!(nuevo.contains("# el de siempre"), "{nuevo}");
    }

    #[test]
    fn rev2_duplicar_no_deja_dos_piezas_con_el_mismo_nombre() {
        let (mut p, _) = abrir(EJEMPLO);
        let c = p.duplicate(1);
        assert_ne!(p.doc.posts[c].name, p.doc.posts[1].name);
    }

    #[test]
    fn rev3_crlf_no_se_mezcla() {
        let crlf = EJEMPLO.replace('\n', "\r\n");
        let (mut p, ruta) = abrir(&crlf);
        titulo(&mut p.doc.posts[1], "Dos, corregido");
        p.save().unwrap();
        let nuevo = std::fs::read_to_string(ruta).unwrap();
        let lf_sueltos = nuevo.replace("\r\n", "").matches('\n').count();
        assert_eq!(lf_sueltos, 0, "hay LF sin CR:\n{nuevo:?}");
    }

    #[test]
    fn rev4_post_como_array_inline_conserva_las_piezas_sin_tocar() {
        let t = "schema = \"instatypst-piece/v1\"\npost = [\n  # una\n  { name = \"a\", content = [ { type = \"title\", text = \"A\" } ] },\n  # otra\n  { name = \"b\", content = [ { type = \"title\", text = \"B\" } ] },\n]\n";
        let (mut p, ruta) = abrir(t);
        titulo(&mut p.doc.posts[1], "B2");
        p.save().unwrap();
        let nuevo = std::fs::read_to_string(ruta).unwrap();
        assert!(nuevo.contains("# una"), "{nuevo}");
    }

    #[test]
    fn rev5_comentario_en_subtabla_de_pieza_sobrevive_a_cambiar_otra_clave() {
        let t = "schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"liso\"\n\n[[post.content]]\n# el titular\ntype = \"title\"\ntext = \"A\"\n";
        let (mut p, ruta) = abrir(t);
        p.doc.posts[0].variant = Some("x".into());
        p.save().unwrap();
        let nuevo = std::fs::read_to_string(ruta).unwrap();
        assert!(nuevo.contains("# el titular"), "{nuevo}");
    }

    #[test]
    fn rev7_indices_fuera_de_rango_no_hacen_panico() {
        let (mut p, _) = abrir(EJEMPLO);
        assert!(p.set_piece_text(99, "[[post]]\n").is_err());
    }

    #[test]
    fn diagnostico_un_typo_de_estilo_y_un_bloque_que_el_estilo_no_acepta() {
        let cat = Catalog::embedded();
        let t = "schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"lisoo\"\ncontent = [ { type = \"title\", text = \"A\" } ]\n\n[[post]]\nname = \"b\"\nstyle = \"liso\"\ncontent = [ { type = \"body\", text = \"x\" }, { type = \"number\", value = 3 } ]\n";
        let (p, _) = abrir(t);
        let v = p.diagnose(&cat);
        let a = v.iter().find(|i| i.path == "post[0].style").expect("typo de estilo");
        assert_eq!(a.code, "style_unknown");
        let b = v.iter().find(|i| i.path == "post[1].content[1]").expect("bloque no aceptado");
        assert_eq!(b.code, "block_unsupported");
        assert_eq!(piece_of(&b.path), Some(1));
        assert_eq!(piece_of("defaults.style"), None);
        // una pieza válida no da nada
        let (ok, _) = abrir("schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"liso\"\ncontent = [ { type = \"body\", text = \"x\" } ]\n");
        assert!(ok.diagnose(&cat).is_empty(), "{:?}", ok.diagnose(&cat));
    }

    #[test]
    fn la_ruta_de_una_incidencia_lleva_a_su_linea_del_editor() {
        let (p, _) = abrir("schema = \"instatypst-piece/v1\"\n\n[[post]]\nname = \"a\"\nstyle = \"liso\"\n\n[[post.content]]\ntype = \"body\"\ntext = \"x\"\n\n[[post.content]]\ntype = \"number\"\nvalue = 3\n");
        let t = p.piece_text(0);
        let l = |path: &str| t.lines().nth(line_of(&t, path)).unwrap().to_string();
        assert_eq!(l("post[0].style"), "style = \"liso\"");
        assert_eq!(l("post[0].content[1]"), "[[post.content]]");
        assert_eq!(line_of(&t, "post[0].content[1]"), t.lines().enumerate().filter(|(_, x)| *x == "[[post.content]]").nth(1).unwrap().0);
        assert_eq!(line_of(&t, "post[0]"), 0);
    }

    #[test]
    fn rev8_schema_nuevo_no_cae_dentro_de_la_ultima_pieza() {
        let t = "[[post]]\nname = \"a\"\ncontent = [ { type = \"title\", text = \"A\" } ]\n";
        let (mut p, ruta) = abrir(t);
        p.doc.schema = Some(SCHEMA.into());
        p.save().unwrap();
        let q = Project::open(&ruta).unwrap();
        assert_eq!(q.doc.schema.as_deref(), Some(SCHEMA));
    }

    /// La GUI diagnostica con las mismas funciones que `check`: lo válido da cero, lo inválido da algo.
    #[test]
    fn el_diagnostico_coincide_con_las_fixtures_del_contrato() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Documentos/GitHub/typst-rrss/instatypst-cli/tests/fixtures/contrato");
        let cat = Catalog::embedded();
        let toml = |d: &str| -> Vec<PathBuf> {
            std::fs::read_dir(base.join(d)).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect()
        };
        for ruta in toml("valid") {
            let p = Project::open(&ruta).unwrap();
            let v = p.diagnose(&cat);
            assert!(v.is_empty(), "{}: {v:?}", ruta.display());
        }
        let (mut con_incidencias, mut sin_abrir) = (0, 0);
        for ruta in toml("invalid") {
            match Project::open(&ruta) {
                Ok(p) if !p.diagnose(&cat).is_empty() => con_incidencias += 1,
                Ok(_) => panic!("{} no da incidencias", ruta.display()),
                Err(_) => sin_abrir += 1, // no parsea: lo dice `parse`, no el diagnóstico
            }
        }
        assert!(con_incidencias > 0, "ninguna fixture inválida dio incidencias ({sin_abrir} no abren)");
    }
}
