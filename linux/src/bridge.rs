use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::models::{Project, TreeNode};
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QString, QStringList};
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

    project: Option<Project>,
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
                let (ids, names, kinds, depths) = flatten_hierarchy(&project.hierarchy);

                self.as_mut().set_project_title(QString::from(&title));
                self.as_mut().set_project_path(QString::from(&path_str));
                self.as_mut().set_binder_ids(to_qstring_list(&ids));
                self.as_mut().set_binder_names(to_qstring_list(&names));
                self.as_mut().set_binder_kinds(to_qstring_list(&kinds));
                self.as_mut().set_binder_depths(to_qstring_list(&depths));
                self.as_mut().set_active_doc_id(QString::default());
                self.as_mut().set_active_doc_name(QString::default());
                self.as_mut().set_active_doc_content(QString::default());
                self.as_mut().set_dirty(false);
                self.as_mut().set_save_label(QString::from("Ready"));

                self.as_mut().rust_mut().project = Some(project);
                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn select_document(mut self: Pin<&mut Self>, id: QString) {
        let id_str = id.to_string();
        let (name, content) = match self.as_ref().rust().project.as_ref() {
            Some(p) => match p.documents.get(&id_str) {
                Some(doc) => (doc.name.clone(), doc.content.clone()),
                None => return,
            },
            None => return,
        };
        self.as_mut().set_active_doc_id(QString::from(&id_str));
        self.as_mut().set_active_doc_name(QString::from(&name));
        self.as_mut().set_active_doc_content(QString::from(&content));
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
                Some(doc) => doc.content = content,
                None => return QString::from("Document not found"),
            }
            writer::write_project(project)
        };

        match write_result {
            Ok(()) => {
                self.as_mut().set_dirty(false);
                self.as_mut().set_save_label(QString::from("Saved"));
                QString::default()
            }
            Err(e) => {
                let msg = format!("{}", e);
                self.as_mut().set_save_label(QString::from(&format!("Error: {}", msg)));
                QString::from(&msg)
            }
        }
    }

    pub fn home_dir(self: &Self) -> QString {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        QString::from(&home)
    }
}

fn flatten_hierarchy(
    nodes: &[TreeNode],
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut ids = Vec::new();
    let mut names = Vec::new();
    let mut kinds = Vec::new();
    let mut depths = Vec::new();
    fn walk(
        nodes: &[TreeNode],
        depth: usize,
        ids: &mut Vec<String>,
        names: &mut Vec<String>,
        kinds: &mut Vec<String>,
        depths: &mut Vec<String>,
    ) {
        for node in nodes {
            match node {
                TreeNode::Folder { id, name, children } => {
                    ids.push(id.clone());
                    names.push(name.clone());
                    kinds.push("Folder".to_string());
                    depths.push(depth.to_string());
                    walk(children, depth + 1, ids, names, kinds, depths);
                }
                TreeNode::Document { id, name, .. } => {
                    ids.push(id.clone());
                    names.push(name.clone());
                    kinds.push("Document".to_string());
                    depths.push(depth.to_string());
                }
            }
        }
    }
    walk(nodes, 0, &mut ids, &mut names, &mut kinds, &mut depths);
    (ids, names, kinds, depths)
}

fn to_qstring_list(v: &[String]) -> QStringList {
    let mut list: QList<QString> = QList::default();
    for s in v {
        list.append(QString::from(s));
    }
    QStringList::from(&list)
}
