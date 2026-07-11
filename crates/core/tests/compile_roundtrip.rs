use chickenscratch_core::core::compile::{compile, CompileOptions, FORMATS};
use chickenscratch_core::core::project::fidelity::acquire_write_token;
use chickenscratch_core::core::project::writer::{create_project, write_project};
use chickenscratch_core::models::{Document, TreeNode};
use chickenscratch_core::ChiknError;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn compile_html_uses_ordering_title_page_separator_and_pandoc_args() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_compile_project(temp_dir.path(), true);
    let output_path = temp_dir.path().join("out.html");
    let args_path = temp_dir.path().join("pandoc-args.txt");
    let pandoc_path = create_pandoc_shim(temp_dir.path(), &args_path);

    compile(
        &project_path,
        &output_path,
        "html",
        Some("Export Title"),
        Some("Export Author"),
        Some(CompileOptions {
            section_separator: Some("***".to_string()),
            include_title_page: true,
            pandoc_path: Some(pandoc_path.to_string_lossy().into_owned()),
            ..Default::default()
        }),
    )
    .unwrap();

    let output = fs::read_to_string(&output_path).unwrap();
    assert!(!output.trim().is_empty());
    assert!(output.contains("Export Author"));
    assert!(output.contains("Approx."));
    assert!(output.contains("# Export Title"));
    assert!(output.contains("by Export Author"));
    assert!(output.contains("\\newpage"));

    let second = output.find("Second ordered section").unwrap();
    let first = output.find("First ordered section").unwrap();
    assert!(
        second < first,
        "compile_order should sort lower values first"
    );
    assert!(output.contains("Second ordered section\n\n\n***\n\nFirst ordered section"));
    assert!(!output.contains("Excluded section"));

    let args = fs::read_to_string(&args_path).unwrap();
    assert_arg_pair(&args, "-f", "markdown");
    assert_arg_pair(&args, "-t", "html");
    assert_arg_pair(&args, "-o", &output_path.to_string_lossy());
    assert!(args.lines().any(|line| line == "--standalone"));
    assert_metadata_arg(&args, "title=Export Title");
    assert_metadata_arg(&args, "author=Export Author");
}

#[test]
fn compile_advertised_formats_use_expected_pandoc_targets() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_compile_project(temp_dir.path(), true);

    for (format, _) in FORMATS {
        let output_path = temp_dir.path().join(format!("out.{format}"));
        let args_path = temp_dir.path().join(format!("pandoc-args-{format}.txt"));
        let pandoc_path = create_pandoc_shim(temp_dir.path(), &args_path);

        compile(
            &project_path,
            &output_path,
            format,
            None,
            None,
            Some(CompileOptions {
                pandoc_path: Some(pandoc_path.to_string_lossy().into_owned()),
                ..Default::default()
            }),
        )
        .unwrap();

        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("First ordered section"));

        let args = fs::read_to_string(&args_path).unwrap();
        assert_arg_pair(&args, "-t", expected_pandoc_target(format));
    }
}

#[test]
fn compile_without_includable_manuscript_content_fails_cleanly() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = create_compile_project(temp_dir.path(), false);
    let output_path = temp_dir.path().join("empty.html");
    let args_path = temp_dir.path().join("pandoc-args-empty.txt");
    let pandoc_path = create_pandoc_shim(temp_dir.path(), &args_path);

    let err = compile(
        &project_path,
        &output_path,
        "html",
        None,
        None,
        Some(CompileOptions {
            pandoc_path: Some(pandoc_path.to_string_lossy().into_owned()),
            ..Default::default()
        }),
    )
    .unwrap_err();

    match err {
        ChiknError::InvalidFormat(message) => {
            assert!(message.contains("No manuscript content to compile"));
        }
        other => panic!("expected InvalidFormat, got {other:?}"),
    }
    assert!(!output_path.exists());
}

fn create_compile_project(root: &Path, with_includable_content: bool) -> PathBuf {
    let project_path = root.join(format!("CompileFixture-{}.chikn", uuid::Uuid::new_v4()));
    let mut project = create_project(&project_path, "Compile Fixture").unwrap();
    let token = acquire_write_token(&project_path).unwrap();
    project.metadata.author = Some("Project Author".to_string());

    let first = Document {
        id: "doc-first".to_string(),
        name: "First".to_string(),
        path: "manuscript/first.md".to_string(),
        content: if with_includable_content {
            "First ordered section".to_string()
        } else {
            String::new()
        },
        compile_order: 2,
        created: "2026-01-01T00:00:00Z".to_string(),
        modified: "2026-01-01T00:00:00Z".to_string(),
        ..Default::default()
    };
    let second = Document {
        id: "doc-second".to_string(),
        name: "Second".to_string(),
        path: "manuscript/second.md".to_string(),
        content: if with_includable_content {
            "Second ordered section".to_string()
        } else {
            String::new()
        },
        compile_order: 1,
        created: "2026-01-01T00:00:00Z".to_string(),
        modified: "2026-01-01T00:00:00Z".to_string(),
        ..Default::default()
    };
    let excluded = Document {
        id: "doc-excluded".to_string(),
        name: "Excluded".to_string(),
        path: "manuscript/excluded.md".to_string(),
        content: "Excluded section".to_string(),
        include_in_compile: false,
        compile_order: 0,
        created: "2026-01-01T00:00:00Z".to_string(),
        modified: "2026-01-01T00:00:00Z".to_string(),
        ..Default::default()
    };
    let research = Document {
        id: "doc-research".to_string(),
        name: "Research".to_string(),
        path: "research/note.md".to_string(),
        content: "Research should not compile".to_string(),
        created: "2026-01-01T00:00:00Z".to_string(),
        modified: "2026-01-01T00:00:00Z".to_string(),
        ..Default::default()
    };

    let manuscript_docs = [first.clone(), second.clone(), excluded.clone()];
    for doc in [&first, &second, &excluded, &research] {
        project.documents.insert(doc.id.clone(), doc.clone());
    }
    project.hierarchy.push(TreeNode::Folder {
        id: "manuscript-folder".to_string(),
        name: "Manuscript".to_string(),
        children: manuscript_docs
            .iter()
            .map(|doc| TreeNode::Document {
                id: doc.id.clone(),
                name: doc.name.clone(),
                path: doc.path.clone(),
            })
            .collect(),
    });
    project.hierarchy.push(TreeNode::Document {
        id: research.id.clone(),
        name: research.name.clone(),
        path: research.path.clone(),
    });

    write_project(&mut project, &token).unwrap();
    project_path
}

fn create_pandoc_shim(root: &Path, args_path: &Path) -> PathBuf {
    let shim_path = root.join(format!("pandoc-shim-{}", uuid::Uuid::new_v4()));
    let script = format!(
        r#"#!/bin/sh
set -eu
args_file={args_file:?}
out=""
prev=""
input=""
for arg in "$@"; do
  printf '%s\n' "$arg" >> "$args_file"
  if [ "$prev" = "-o" ]; then
    out="$arg"
  fi
  prev="$arg"
  input="$arg"
done
if [ -z "$out" ]; then
  echo "missing -o" >&2
  exit 2
fi
cp "$input" "$out"
"#,
        args_file = args_path.display()
    );
    fs::write(&shim_path, script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&shim_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&shim_path, perms).unwrap();
    }

    shim_path
}

fn expected_pandoc_target(format: &str) -> &str {
    match format {
        "docx" => "docx",
        "pdf" => "pdf",
        "epub" => "epub3",
        "html" => "html",
        "odt" => "odt",
        _ => "docx",
    }
}

fn assert_arg_pair(args: &str, key: &str, value: &str) {
    let lines: Vec<&str> = args.lines().collect();
    assert!(
        lines.windows(2).any(|pair| pair == [key, value]),
        "expected argument pair {key:?} {value:?} in {lines:?}"
    );
}

fn assert_metadata_arg(args: &str, value: &str) {
    let lines: Vec<&str> = args.lines().collect();
    assert!(
        lines
            .windows(2)
            .any(|pair| pair[0] == "--metadata" && pair[1] == value),
        "expected metadata {value:?} in {lines:?}"
    );
}
