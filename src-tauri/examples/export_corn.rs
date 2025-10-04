//! Export Corn.chikn back to .scriv to test round-trip

use std::path::Path;
use chicken_scratch::core::project::reader::read_project;
use chicken_scratch::scrivener::exporter::export_to_scriv;

fn main() {
    let chikn_path = Path::new("samples/Corn.chikn");
    let output_path = Path::new("samples/Corn-exported.scriv");

    println!("Loading {} ...", chikn_path.display());

    let project = match read_project(chikn_path) {
        Ok(p) => {
            println!("✅ Loaded project: {}", p.name);
            println!("   Documents: {}", p.documents.len());
            p
        }
        Err(e) => {
            eprintln!("❌ Failed to load project: {:?}", e);
            std::process::exit(1);
        }
    };

    println!("\nExporting to {} ...", output_path.display());

    match export_to_scriv(&project, output_path) {
        Ok(_) => {
            println!("✅ Export successful!");
            println!("\nVerifying export:");

            // Check structure
            let scrivx_exists = output_path.join(format!("{}.scrivx", project.name)).exists();
            let files_exist = output_path.join("Files/Data").exists();
            let version_exists = output_path.join("Files/version.txt").exists();

            println!("  .scrivx file: {}", if scrivx_exists { "✅" } else { "❌" });
            println!("  Files/Data: {}", if files_exist { "✅" } else { "❌" });
            println!("  version.txt: {}", if version_exists { "✅" } else { "❌" });
        }
        Err(e) => {
            eprintln!("❌ Export failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
