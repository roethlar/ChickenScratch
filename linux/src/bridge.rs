use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::models::{Project, TreeNode};
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
use std::collections::HashSet;
use std::path::PathBuf;
use std::pin::Pin;

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, project_title)]
        #[qproperty(QString, project_path)]
        #[qproperty(QString, active_doc_id)]
        #[qproperty(QString, active_doc_name)]
        #[qproperty(QString, active_doc_content)]
        #[qproperty(QString, save_label)]
        #[qproperty(bool, dirty)]
        #[qproperty(QStringList, binder_ids)]
        #[qproperty(QStringList, binder_names)]
        #[qproperty(QStringList, binder_kinds)]
        #[qproperty(QStringList, binder_depths)]
        #[qproperty(QStringList, binder_has_children)]
        #[qproperty(QStringList, binder_expanded)]
        #[qproperty(QString, doc_synopsis)]
        #[qproperty(QString, doc_label)]
        #[qproperty(QString, doc_status)]
        #[qproperty(QString, doc_keywords)]
        #[qproperty(bool, doc_include_in_compile)]
        #[qproperty(i32, doc_word_count_target)]
        #[qproperty(QString, doc_modified)]
        type AppController = super::AppControllerRust;

        #[qinvokable]
        fn open_project(self: Pin<&mut AppController>, path: QString) -> QString;

        #[qinvokable]
        fn select_document(self: Pin<&mut AppController>, id: QString);

        #[qinvokable]
        fn update_content(self: Pin<&mut AppController>, text: QString);

        #[qinvokable]
        fn save(self: Pin<&mut AppController>) -> QString;

        #[qinvokable]
        fn rename_node(self: Pin<&mut AppController>, id: QString, new_name: QString) -> QString;

        #[qinvokable]
        fn save_metadata(
            self: Pin<&mut AppController>,
            synopsis: QString,
            label: QString,
            status: QString,
            keywords: QString,
            include_in_compile: bool,
            word_count_target: i32,
        ) -> QString;

        #[qinvokable]
        fn toggle_folder(self: Pin<&mut AppController>, id: QString);

        #[qinvokable]
        fn home_dir(self: &AppController) -> QString;
    }
}

#[derive(Default)]
pub struct AppControllerRust {
    project_title: QString,
    project_path: QString,
    active_doc_id: QString,
    active_doc_name: QString,
    active_doc_content: QString,
    save_label: QString,
    dirty: bool,
    binder_ids: QStringList,
    binder_names: QStringList,
    binder_kinds: QStringList,
    binder_depths: QStringList,
    binder_has_children: QStringList,
    binder_expanded: QStringList,
    doc_synopsis: QString,
    doc_label: QString,
    doc_status: QString,
    doc_keywords: QString,
    doc_include_in_compile: bool,
    doc_word_count_target: i32,
    doc_modified: QString,

    project: Option<Project>,
    collapsed: HashSet<String>,
}

impl ffi::AppController {
    pub fn open_project(mut self: Pin<&mut Self>, path: QString) -> QString {
        let path_str = path.to_string();
        let pb = PathBuf::from(&path_str);
        match reader::read_project(&pb) {
            Ok(project) => {
                let title = project
                    .metadata
                    .title
                    .clone()
                    .unwrap_or_else(|| project.name.clone());

                self.as_mut().set_project_title(QString::from(&title));
                self.as_mut().set_project_path(QString::from(&path_str));
                self.as_mut().set_active_doc_id(QString::default());
                self.as_mut().set_active_doc_name(QString::default());
                self.as_mut().set_active_doc_content(QString::default());
                self.as_mut().set_dirty(false);
                self.as_mut().set_save_label(QString::from("Ready"));
                self.as_mut().rust_mut().collapsed.clear();
                self.as_mut().rust_mut().project = Some(project);
                self.as_mut().refresh_binder();
                self.as_mut().clear_doc_fields();
                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn select_document(mut self: Pin<&mut Self>, id: QString) {
        let id_str = id.to_string();
        let doc_snapshot = self
            .as_ref()
            .rust()
            .project
            .as_ref()
            .and_then(|p| p.documents.get(&id_str))
            .cloned();
        let doc = match doc_snapshot {
            Some(d) => d,
            None => return,
        };
        let keywords_csv = doc
            .keywords
            .as_ref()
            .map(|k| k.join(", "))
            .unwrap_or_default();

        self.as_mut().set_active_doc_id(QString::from(&id_str));
        self.as_mut().set_active_doc_name(QString::from(&doc.name));
        self.as_mut().set_active_doc_content(QString::from(&doc.content));
        self.as_mut()
            .set_doc_synopsis(QString::from(doc.synopsis.as_deref().unwrap_or("")));
        self.as_mut()
            .set_doc_label(QString::from(doc.label.as_deref().unwrap_or("")));
        self.as_mut()
            .set_doc_status(QString::from(doc.status.as_deref().unwrap_or("")));
        self.as_mut().set_doc_keywords(QString::from(&keywords_csv));
        self.as_mut().set_doc_include_in_compile(doc.include_in_compile);
        self.as_mut()
            .set_doc_word_count_target(doc.word_count_target as i32);
        self.as_mut().set_doc_modified(QString::from(&doc.modified));
        self.as_mut().set_dirty(false);
        self.as_mut().set_save_label(QString::from("Saved"));
    }

    pub fn update_content(mut self: Pin<&mut Self>, text: QString) {
        let s = text.to_string();
        let current = self.as_ref().active_doc_content().to_string();
        if current == s {
            return;
        }
        self.as_mut().set_active_doc_content(QString::from(&s));
        self.as_mut().set_dirty(true);
        self.as_mut().set_save_label(QString::from("Modified"));
    }

    pub fn save(mut self: Pin<&mut Self>) -> QString {
        let id = self.as_ref().active_doc_id().to_string();
        if id.is_empty() {
            return QString::from("No document selected");
        }
        let content = self.as_ref().active_doc_content().to_string();
        self.as_mut().set_save_label(QString::from("Saving..."));

        let write_result = {
            let mut rust_mut = self.as_mut().rust_mut();
            let project = match rust_mut.project.as_mut() {
                Some(p) => p,
                None => return QString::from("No project loaded"),
            };
            match project.documents.get_mut(&id) {
                Some(doc) => {
                    doc.content = content;
                    doc.modified = chrono_now();
                }
                None => return QString::from("Document not found"),
            }
            writer::write_project(project)
        };

        match write_result {
            Ok(()) => {
                let modified = self
                    .as_ref()
                    .rust()
                    .project
                    .as_ref()
                    .and_then(|p| p.documents.get(&id))
                    .map(|d| d.modified.clone())
                    .unwrap_or_default();
                self.as_mut().set_doc_modified(QString::from(&modified));
                self.as_mut().set_dirty(false);
                self.as_mut().set_save_label(QString::from("Saved"));
                QString::default()
            }
            Err(e) => {
                let msg = format!("{}", e);
                self.as_mut()
                    .set_save_label(QString::from(&format!("Error: {}", msg)));
                QString::from(&msg)
            }
        }
    }

    pub fn rename_node(mut self: Pin<&mut Self>, id: QString, new_name: QString) -> QString {
        let id_str = id.to_string();
        let name = new_name.to_string();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return QString::from("Name cannot be empty");
        }

        let write_result = {
            let mut rust_mut = self.as_mut().rust_mut();
            let project = match rust_mut.project.as_mut() {
                Some(p) => p,
                None => return QString::from("No project loaded"),
            };
            rename_in_hierarchy(&mut project.hierarchy, &id_str, trimmed);
            if let Some(doc) = project.documents.get_mut(&id_str) {
                doc.name = trimmed.to_string();
                doc.modified = chrono_now();
            }
            writer::write_project(project)
        };

        match write_result {
            Ok(()) => {
                if self.as_ref().active_doc_id().to_string() == id_str {
                    self.as_mut().set_active_doc_name(QString::from(trimmed));
                }
                self.as_mut().refresh_binder();
                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn save_metadata(
        mut self: Pin<&mut Self>,
        synopsis: QString,
        label: QString,
        status: QString,
        keywords: QString,
        include_in_compile: bool,
        word_count_target: i32,
    ) -> QString {
        let id = self.as_ref().active_doc_id().to_string();
        if id.is_empty() {
            return QString::from("No document selected");
        }

        let synopsis = non_empty(synopsis.to_string());
        let label = non_empty(label.to_string());
        let status = non_empty(status.to_string());
        let keywords_vec: Vec<String> = keywords
            .to_string()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let keywords_opt = if keywords_vec.is_empty() {
            None
        } else {
            Some(keywords_vec)
        };
        let target = word_count_target.max(0) as u32;

        let write_result = {
            let mut rust_mut = self.as_mut().rust_mut();
            let project = match rust_mut.project.as_mut() {
                Some(p) => p,
                None => return QString::from("No project loaded"),
            };
            match project.documents.get_mut(&id) {
                Some(doc) => {
                    doc.synopsis = synopsis;
                    doc.label = label;
                    doc.status = status;
                    doc.keywords = keywords_opt;
                    doc.include_in_compile = include_in_compile;
                    doc.word_count_target = target;
                    doc.modified = chrono_now();
                }
                None => return QString::from("Document not found"),
            }
            writer::write_project(project)
        };

        match write_result {
            Ok(()) => {
                let modified = self
                    .as_ref()
                    .rust()
                    .project
                    .as_ref()
                    .and_then(|p| p.documents.get(&id))
                    .map(|d| d.modified.clone())
                    .unwrap_or_default();
                self.as_mut().set_doc_modified(QString::from(&modified));
                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn toggle_folder(mut self: Pin<&mut Self>, id: QString) {
        let id_str = id.to_string();
        {
            let mut rust_mut = self.as_mut().rust_mut();
            if rust_mut.collapsed.contains(&id_str) {
                rust_mut.collapsed.remove(&id_str);
            } else {
                rust_mut.collapsed.insert(id_str);
            }
        }
        self.as_mut().refresh_binder();
    }

    pub fn home_dir(self: &Self) -> QString {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        QString::from(&home)
    }

    fn refresh_binder(mut self: Pin<&mut Self>) {
        let (ids, names, kinds, depths, has_children, expanded) = {
            let pinned = self.as_ref();
            let rust_ref = pinned.rust();
            match rust_ref.project.as_ref() {
                Some(p) => flatten_hierarchy(&p.hierarchy, &rust_ref.collapsed),
                None => (
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ),
            }
        };
        self.as_mut().set_binder_ids(to_qstring_list(&ids));
        self.as_mut().set_binder_names(to_qstring_list(&names));
        self.as_mut().set_binder_kinds(to_qstring_list(&kinds));
        self.as_mut().set_binder_depths(to_qstring_list(&depths));
        self.as_mut()
            .set_binder_has_children(to_qstring_list(&has_children));
        self.as_mut().set_binder_expanded(to_qstring_list(&expanded));
    }

    fn clear_doc_fields(mut self: Pin<&mut Self>) {
        self.as_mut().set_doc_synopsis(QString::default());
        self.as_mut().set_doc_label(QString::default());
        self.as_mut().set_doc_status(QString::default());
        self.as_mut().set_doc_keywords(QString::default());
        self.as_mut().set_doc_include_in_compile(true);
        self.as_mut().set_doc_word_count_target(0);
        self.as_mut().set_doc_modified(QString::default());
    }
}

fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn chrono_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn rename_in_hierarchy(nodes: &mut Vec<TreeNode>, node_id: &str, new_name: &str) {
    for node in nodes {
        match node {
            TreeNode::Document { id, name, .. } if id == node_id => {
                *name = new_name.to_string();
                return;
            }
            TreeNode::Folder { id, name, children } => {
                if id == node_id {
                    *name = new_name.to_string();
                    return;
                }
                rename_in_hierarchy(children, node_id, new_name);
            }
            _ => {}
        }
    }
}

type FlattenResult = (
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
);

fn flatten_hierarchy(nodes: &[TreeNode], collapsed: &HashSet<String>) -> FlattenResult {
    let mut ids = Vec::new();
    let mut names = Vec::new();
    let mut kinds = Vec::new();
    let mut depths = Vec::new();
    let mut has_children = Vec::new();
    let mut expanded = Vec::new();

    fn walk(
        nodes: &[TreeNode],
        depth: usize,
        collapsed: &HashSet<String>,
        ids: &mut Vec<String>,
        names: &mut Vec<String>,
        kinds: &mut Vec<String>,
        depths: &mut Vec<String>,
        has_children: &mut Vec<String>,
        expanded: &mut Vec<String>,
    ) {
        for node in nodes {
            match node {
                TreeNode::Folder {
                    id,
                    name,
                    children,
                } => {
                    let is_collapsed = collapsed.contains(id);
                    ids.push(id.clone());
                    names.push(name.clone());
                    kinds.push("Folder".to_string());
                    depths.push(depth.to_string());
                    has_children.push(if children.is_empty() { "0" } else { "1" }.to_string());
                    expanded.push(if is_collapsed { "0" } else { "1" }.to_string());
                    if !is_collapsed {
                        walk(
                            children,
                            depth + 1,
                            collapsed,
                            ids,
                            names,
                            kinds,
                            depths,
                            has_children,
                            expanded,
                        );
                    }
                }
                TreeNode::Document { id, name, .. } => {
                    ids.push(id.clone());
                    names.push(name.clone());
                    kinds.push("Document".to_string());
                    depths.push(depth.to_string());
                    has_children.push("0".to_string());
                    expanded.push("0".to_string());
                }
            }
        }
    }

    walk(
        nodes,
        0,
        collapsed,
        &mut ids,
        &mut names,
        &mut kinds,
        &mut depths,
        &mut has_children,
        &mut expanded,
    );
    (ids, names, kinds, depths, has_children, expanded)
}

fn to_qstring_list(v: &[String]) -> QStringList {
    let mut list: QList<QString> = QList::default();
    for s in v {
        list.append(QString::from(s));
    }
    QStringList::from(&list)
}
