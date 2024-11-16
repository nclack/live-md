use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Print cargo environment variables
    println!("cargo:warning=Build Script - Environment Variables:");
    println!(
        "cargo:warning=CARGO_MANIFEST_DIR: {:?}",
        env::var("CARGO_MANIFEST_DIR")
    );
    println!("cargo:warning=OUT_DIR: {:?}", env::var("OUT_DIR"));
    println!(
        "cargo:warning=CARGO_PKG_NAME: {:?}",
        env::var("CARGO_PKG_NAME")
    );

    // Print current working directory
    println!(
        "cargo:warning=Current working directory: {:?}",
        env::current_dir().unwrap()
    );

    // Check template directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let template_dir = Path::new(&manifest_dir).join("src").join("templates");
    println!("cargo:warning=Template directory path: {:?}", template_dir);
    println!(
        "cargo:warning=Template directory exists: {}",
        template_dir.exists()
    );

    // List all files in template directory if it exists
    if template_dir.exists() {
        println!("cargo:warning=Files in template directory:");
        match fs::read_dir(&template_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    println!("cargo:warning=  {:?}", entry.path());
                }
            }
            Err(e) => println!("cargo:warning=Error reading template directory: {}", e),
        }
    }

    // Check specific template files
    let template_files = vec![
        "index-start.html",
        "index-end.html",
        "page-start.html",
        "page-end.html",
    ];

    for file in template_files {
        let file_path = template_dir.join(file);
        println!(
            "cargo:warning=Template file {:?} exists: {}",
            file,
            file_path.exists()
        );
        if file_path.exists() {
            match fs::metadata(&file_path) {
                Ok(metadata) => println!(
                    "cargo:warning=  Size: {} bytes, readonly: {}",
                    metadata.len(),
                    metadata.permissions().readonly()
                ),
                Err(e) => println!("cargo:warning=  Error getting metadata: {}", e),
            }
        }
    }

    // Also check parent directories for context
    let src_dir = template_dir.parent().unwrap();
    if src_dir.exists() {
        println!("cargo:warning=Files in src directory:");
        match fs::read_dir(src_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    println!("cargo:warning=  {:?}", entry.path());
                }
            }
            Err(e) => println!("cargo:warning=Error reading src directory: {}", e),
        }
    }

    // Print any custom environment variables that might be relevant
    for (key, value) in env::vars() {
        if key.starts_with("NIX_") || key.starts_with("CARGO_") {
            println!("cargo:warning={}={}", key, value);
        }
    }

    println!("cargo:rerun-if-changed=src/templates");
}
