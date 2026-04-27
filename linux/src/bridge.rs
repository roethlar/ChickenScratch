use chickenscratch_core::core::git;
use chickenscratch_core::core::project::{reader, writer};
use chickenscratch_core::models::{Document, Project, TreeNode};
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
        #[qproperty(QString, recent_projects_json)]
        #[qproperty(bool, show_welcome)]
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

        // New invokables
        #[qinvokable]
        fn create_project(self: Pin<&mut AppController>, path: QString, name: QString) -> QString;

        #[qinvokable]
        fn new_document(self: Pin<&mut AppController>, name: QString, parent_id: QString) -> QString;

        #[qinvokable]
        fn new_folder(self: Pin<&mut AppController>, name: QString, parent_id: QString) -> QString;

        #[qinvokable]
        fn delete_node(self: Pin<&mut AppController>, id: QString) -> QString;

        #[qinvokable]
        fn list_revisions_json(self: Pin<&mut AppController>) -> QString;

        #[qinvokable]
        fn save_revision_from_msg(self: Pin<&mut AppController>, msg: QString) -> QString;

        #[qinvokable]
        fn restore_revision_by_id(self: Pin<&mut AppController>, commit_id: QString) -> QString;

        #[qinvokable]
        fn list_drafts_json(self: Pin<&mut AppController>) -> QString;

        #[qinvokable]
        fn create_draft_by_name(self: Pin<&mut AppController>, name: QString) -> QString;

        #[qinvokable]
        fn switch_draft_by_name(self: Pin<&mut AppController>, name: QString) -> QString;

        #[qinvokable]
        fn get_stats_json(self: Pin<&mut AppController>) -> QString;
    }
}

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
    recent_projects_json: QString,
    show_welcome: bool,

    project: Option<Project>,
    collapsed: HashSet<String>,
}

impl Default for AppControllerRust {
    fn default() -> Self {
        let recents = load_recents();
        let recent_json = serde_json::to_string(&recents).unwrap_or_else(|_| "[]".to_string());
        Self {
            project_title: QString::default(),
            project_path: QString::default(),
            active_doc_id: QString::default(),
            active_doc_name: QString::default(),
            active_doc_content: QString::default(),
            save_label: QString::default(),
            dirty: false,
            binder_ids: QStringList::default(),
            binder_names: QStringList::default(),
            binder_kinds: QStringList::default(),
            binder_depths: QStringList::default(),
            binder_has_children: QStringList::default(),
            binder_expanded: QStringList::default(),
            doc_synopsis: QString::default(),
            doc_label: QString::default(),
            doc_status: QString::default(),
            doc_keywords: QString::default(),
            doc_include_in_compile: false,
            doc_word_count_target: 0,
            doc_modified: QString::default(),
            recent_projects_json: QString::from(&recent_json),
            show_welcome: true,
            project: None,
            collapsed: HashSet::new(),
        }
    }
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
                self.as_mut().rust_mut().project = Some(project.clone());
                self.as_mut().refresh_binder();
                self.as_mut().clear_doc_fields();
                self.as_mut().set_show_welcome(false);

                // Update recents
                let name = project.name.clone();
                update_recents(&name, &path_str);
                let recents = load_recents();
                let json = serde_json::to_string(&recents).unwrap_or_else(|_| "[]".to_string());
                self.as_mut().set_recent_projects_json(QString::from(&json));

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

    pub fn create_project(mut self: Pin<&mut Self>, path: QString, name: QString) -> QString {
        let path_str = path.to_string();
        let name_str = name.to_string();
        let name_trimmed = name_str.trim();
        let path_trimmed = path_str.trim();

        if name_trimmed.is_empty() {
            return QString::from("Project name cannot be empty");
        }
        if path_trimmed.is_empty() {
            return QString::from("Project path cannot be empty");
        }

        let pb = PathBuf::from(path_trimmed);
        match writer::create_project(&pb, name_trimmed) {
            Ok(project) => {
                let title = project
                    .metadata
                    .title
                    .clone()
                    .unwrap_or_else(|| project.name.clone());

                let project_path_str = path_trimmed.to_string();
                self.as_mut().set_project_title(QString::from(&title));
                self.as_mut().set_project_path(QString::from(&project_path_str));
                self.as_mut().set_active_doc_id(QString::default());
                self.as_mut().set_active_doc_name(QString::default());
                self.as_mut().set_active_doc_content(QString::default());
                self.as_mut().set_dirty(false);
                self.as_mut().set_save_label(QString::from("Ready"));
                self.as_mut().rust_mut().collapsed.clear();
                self.as_mut().rust_mut().project = Some(project.clone());
                self.as_mut().refresh_binder();
                self.as_mut().clear_doc_fields();
                self.as_mut().set_show_welcome(false);

                // Update recents
                let proj_name = project.name.clone();
                update_recents(&proj_name, &project_path_str);
                let recents = load_recents();
                let json = serde_json::to_string(&recents).unwrap_or_else(|_| "[]".to_string());
                self.as_mut().set_recent_projects_json(QString::from(&json));

                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn new_document(mut self: Pin<&mut Self>, name: QString, parent_id: QString) -> QString {
        let name_str = name.to_string();
        let name_trimmed = name_str.trim();
        if name_trimmed.is_empty() {
            return QString::from("Document name cannot be empty");
        }
        let parent_id_str = parent_id.to_string();

        let now = chrono_now();
        let new_id = make_id();
        let slug = make_slug(name_trimmed);
        let rel_path = format!("manuscript/{}.md", slug);

        let write_result = {
            let mut rust_mut = self.as_mut().rust_mut();
            let project = match rust_mut.project.as_mut() {
                Some(p) => p,
                None => return QString::from("No project loaded"),
            };

            let new_node = TreeNode::Document {
                id: new_id.clone(),
                name: name_trimmed.to_string(),
                path: rel_path.clone(),
            };

            if parent_id_str.is_empty() {
                project.hierarchy.push(new_node);
            } else {
                if !add_to_hierarchy(&mut project.hierarchy, &parent_id_str, new_node.clone()) {
                    project.hierarchy.push(new_node);
                }
            }

            let doc = Document {
                id: new_id.clone(),
                name: name_trimmed.to_string(),
                path: rel_path.clone(),
                content: String::new(),
                parent_id: non_empty(parent_id_str.clone()),
                created: now.clone(),
                modified: now.clone(),
                include_in_compile: true,
                ..Default::default()
            };
            project.documents.insert(new_id.clone(), doc);

            writer::write_project(project)
        };

        match write_result {
            Ok(()) => {
                self.as_mut().refresh_binder();
                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn new_folder(mut self: Pin<&mut Self>, name: QString, parent_id: QString) -> QString {
        let name_str = name.to_string();
        let name_trimmed = name_str.trim();
        if name_trimmed.is_empty() {
            return QString::from("Folder name cannot be empty");
        }
        let parent_id_str = parent_id.to_string();

        let new_id = make_id();

        let write_result = {
            let mut rust_mut = self.as_mut().rust_mut();
            let project = match rust_mut.project.as_mut() {
                Some(p) => p,
                None => return QString::from("No project loaded"),
            };

            let new_node = TreeNode::Folder {
                id: new_id.clone(),
                name: name_trimmed.to_string(),
                children: Vec::new(),
            };

            if parent_id_str.is_empty() {
                project.hierarchy.push(new_node);
            } else {
                if !add_to_hierarchy(&mut project.hierarchy, &parent_id_str, new_node.clone()) {
                    project.hierarchy.push(new_node);
                }
            }

            writer::write_project(project)
        };

        match write_result {
            Ok(()) => {
                self.as_mut().refresh_binder();
                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn delete_node(mut self: Pin<&mut Self>, id: QString) -> QString {
        let id_str = id.to_string();
        if id_str.is_empty() {
            return QString::from("No node ID provided");
        }

        let write_result = {
            let mut rust_mut = self.as_mut().rust_mut();
            let project = match rust_mut.project.as_mut() {
                Some(p) => p,
                None => return QString::from("No project loaded"),
            };

            remove_from_hierarchy(&mut project.hierarchy, &id_str);
            project.documents.remove(&id_str);

            writer::write_project(project)
        };

        match write_result {
            Ok(()) => {
                let active = self.as_ref().active_doc_id().to_string();
                if active == id_str {
                    self.as_mut().set_active_doc_id(QString::default());
                    self.as_mut().set_active_doc_name(QString::default());
                    self.as_mut().set_active_doc_content(QString::default());
                    self.as_mut().set_dirty(false);
                    self.as_mut().clear_doc_fields();
                }
                self.as_mut().refresh_binder();
                QString::default()
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn list_revisions_json(mut self: Pin<&mut Self>) -> QString {
        let path_str = self.as_ref().project_path().to_string();
        if path_str.is_empty() {
            return QString::from("[]");
        }
        let pb = PathBuf::from(&path_str);
        match git::list_revisions(&pb) {
            Ok(revisions) => {
                let json = serde_json::to_string(&revisions).unwrap_or_else(|_| "[]".to_string());
                QString::from(&json)
            }
            Err(_) => QString::from("[]"),
        }
    }

    pub fn save_revision_from_msg(mut self: Pin<&mut Self>, msg: QString) -> QString {
        let path_str = self.as_ref().project_path().to_string();
        if path_str.is_empty() {
            return QString::from("No project loaded");
        }
        let message = msg.to_string();
        let message_trimmed = if message.trim().is_empty() {
            "Manual save".to_string()
        } else {
            message.trim().to_string()
        };

        let pb = PathBuf::from(&path_str);
        match git::save_revision(&pb, &message_trimmed) {
            Ok(_) => QString::default(),
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn restore_revision_by_id(mut self: Pin<&mut Self>, commit_id: QString) -> QString {
        let path_str = self.as_ref().project_path().to_string();
        if path_str.is_empty() {
            return QString::from("No project loaded");
        }
        let id_str = commit_id.to_string();
        let pb = PathBuf::from(&path_str);

        match git::restore_revision(&pb, &id_str) {
            Ok(_) => {
                // Reload the project after restoring
                match reader::read_project(&pb) {
                    Ok(project) => {
                        let title = project
                            .metadata
                            .title
                            .clone()
                            .unwrap_or_else(|| project.name.clone());
                        self.as_mut().set_project_title(QString::from(&title));
                        self.as_mut().set_active_doc_id(QString::default());
                        self.as_mut().set_active_doc_name(QString::default());
                        self.as_mut().set_active_doc_content(QString::default());
                        self.as_mut().set_dirty(false);
                        self.as_mut().set_save_label(QString::from("Restored"));
                        self.as_mut().rust_mut().collapsed.clear();
                        self.as_mut().rust_mut().project = Some(project);
                        self.as_mut().refresh_binder();
                        self.as_mut().clear_doc_fields();
                        QString::default()
                    }
                    Err(e) => QString::from(&format!("Restored but reload failed: {}", e)),
                }
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn list_drafts_json(mut self: Pin<&mut Self>) -> QString {
        let path_str = self.as_ref().project_path().to_string();
        if path_str.is_empty() {
            return QString::from("[]");
        }
        let pb = PathBuf::from(&path_str);
        match git::list_drafts(&pb) {
            Ok(drafts) => {
                let json = serde_json::to_string(&drafts).unwrap_or_else(|_| "[]".to_string());
                QString::from(&json)
            }
            Err(_) => QString::from("[]"),
        }
    }

    pub fn create_draft_by_name(mut self: Pin<&mut Self>, name: QString) -> QString {
        let path_str = self.as_ref().project_path().to_string();
        if path_str.is_empty() {
            return QString::from("No project loaded");
        }
        let name_str = name.to_string();
        let name_trimmed = name_str.trim();
        if name_trimmed.is_empty() {
            return QString::from("Draft name cannot be empty");
        }
        let pb = PathBuf::from(&path_str);
        match git::create_draft(&pb, name_trimmed) {
            Ok(()) => QString::default(),
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn switch_draft_by_name(mut self: Pin<&mut Self>, name: QString) -> QString {
        let path_str = self.as_ref().project_path().to_string();
        if path_str.is_empty() {
            return QString::from("No project loaded");
        }
        let name_str = name.to_string();
        let name_trimmed = name_str.trim();
        if name_trimmed.is_empty() {
            return QString::from("Draft name cannot be empty");
        }
        let pb = PathBuf::from(&path_str);

        match git::switch_draft(&pb, name_trimmed) {
            Ok(()) => {
                // Reload project after switching draft
                match reader::read_project(&pb) {
                    Ok(project) => {
                        let title = project
                            .metadata
                            .title
                            .clone()
                            .unwrap_or_else(|| project.name.clone());
                        self.as_mut().set_project_title(QString::from(&title));
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
                    Err(e) => QString::from(&format!("Switched but reload failed: {}", e)),
                }
            }
            Err(e) => QString::from(&format!("{}", e)),
        }
    }

    pub fn get_stats_json(mut self: Pin<&mut Self>) -> QString {
        let rust = self.as_ref().rust();
        let project = match rust.project.as_ref() {
            Some(p) => p,
            None => return QString::from("{}"),
        };

        let mut total_words: usize = 0;
        let mut doc_entries: Vec<serde_json::Value> = Vec::new();

        for doc in project.documents.values() {
            let words = doc.content.split_whitespace().count();
            total_words += words;
            doc_entries.push(serde_json::json!({
                "id": doc.id,
                "name": doc.name,
                "words": words,
            }));
        }

        let doc_count = project.documents.len();
        let page_count = (total_words as f64 / 250.0).ceil() as usize;
        let reading_minutes = (total_words as f64 / 238.0).ceil() as usize;

        let stats = serde_json::json!({
            "total_words": total_words,
            "doc_count": doc_count,
            "page_count": page_count,
            "reading_minutes": reading_minutes,
            "docs": doc_entries,
        });

        let json = serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string());
        QString::from(&json)
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

// ── Recent projects ───────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct RecentProject {
    name: String,
    path: String,
}

fn recents_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("chickenscratch")
        .join("recents.json")
}

fn load_recents() -> Vec<RecentProject> {
    let path = recents_path();
    if !path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<RecentProject>>(&s).ok())
        .unwrap_or_default()
}

fn update_recents(name: &str, path: &str) {
    let mut recents = load_recents();
    // Remove duplicates by path
    recents.retain(|r| r.path != path);
    // Insert at front
    recents.insert(
        0,
        RecentProject {
            name: name.to_string(),
            path: path.to_string(),
        },
    );
    // Keep at most 10
    recents.truncate(10);

    let recents_file = recents_path();
    if let Some(parent) = recents_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&recents) {
        let _ = std::fs::write(&recents_file, json);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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

fn make_id() -> String {
    format!(
        "{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

fn make_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Recursively add `new_node` as a child of the folder with the given `parent_id`.
/// Returns `true` if the parent was found and the node was added.
fn add_to_hierarchy(nodes: &mut Vec<TreeNode>, parent_id: &str, new_node: TreeNode) -> bool {
    for node in nodes.iter_mut() {
        match node {
            TreeNode::Folder { id, children, .. } => {
                if id == parent_id {
                    children.push(new_node);
                    return true;
                }
                if add_to_hierarchy(children, parent_id, new_node.clone()) {
                    return true;
                }
            }
            TreeNode::Document { .. } => {}
        }
    }
    false
}

/// Recursively remove the node with the given `node_id` from the hierarchy.
fn remove_from_hierarchy(nodes: &mut Vec<TreeNode>, node_id: &str) {
    nodes.retain(|n| n.id() != node_id);
    for node in nodes.iter_mut() {
        if let TreeNode::Folder { children, .. } = node {
            remove_from_hierarchy(children, node_id);
        }
    }
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
