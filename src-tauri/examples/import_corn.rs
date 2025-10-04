//! Import Corn.scriv sample to test Scrivener importer

use std::path::Path;
use chicken_scratch::scrivener::converter::import_scriv;

fn main() {
    let scriv_path = Path::new("samples/Corn.scriv");
    let output_path = Path::new("samples/Corn.chikn");

    println!("Importing {} to {}", scriv_path.display(), output_path.display());

    match import_scriv(scriv_path, output_path) {
        Ok(project) => {
            println!("✅ Import successful!");
            println!("Project: {}", project.name);
            println!("Documents: {}", project.documents.len());
            println!("Hierarchy items: {}", project.hierarchy.len());
            println!("\nHierarchy:");
            for (i, node) in project.hierarchy.iter().enumerate() {
                match node {
                    chicken_scratch::TreeNode::Folder { name, children, .. } => {
                        println!("  {}. [Folder] {} ({} children)", i+1, name, children.len());
                    }
                    chicken_scratch::TreeNode::Document { name, .. } => {
                        println!("  {}. [Document] {}", i+1, name);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Import failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
