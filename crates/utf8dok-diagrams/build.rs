//! Build script for utf8dok-diagrams
//!
//! Downloads mermaid.min.js during build when the `js` feature is enabled.

fn main() {
    // Only download mermaid.js when js feature is enabled
    #[cfg(feature = "js")]
    download_mermaid();

    // Re-run if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");
}

#[cfg(feature = "js")]
fn download_mermaid() {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    /// Mermaid.js CDN URL (pinned version for reproducibility)
    const MERMAID_VERSION: &str = "10.6.1";
    const MERMAID_CDN_URL: &str =
        "https://cdn.jsdelivr.net/npm/mermaid@{VERSION}/dist/mermaid.min.js";

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let mermaid_path = out_dir.join("mermaid.min.js");

    // Skip download if already exists and not stale
    if mermaid_path.exists() {
        println!(
            "cargo:warning=mermaid.min.js already exists at {}",
            mermaid_path.display()
        );
        return;
    }

    let url = MERMAID_CDN_URL.replace("{VERSION}", MERMAID_VERSION);
    println!(
        "cargo:warning=Downloading mermaid.js v{} from CDN...",
        MERMAID_VERSION
    );

    // Try to download using reqwest (blocking)
    match download_file(&url, &mermaid_path) {
        Ok(size) => {
            println!(
                "cargo:warning=Downloaded mermaid.min.js ({} bytes) to {}",
                size,
                mermaid_path.display()
            );
        }
        Err(e) => {
            // Fall back to bundled version if download fails
            println!("cargo:warning=Failed to download mermaid.js: {}", e);
            println!("cargo:warning=Build will fail if js feature is used without mermaid.js");

            // Create a placeholder that will error at runtime
            fs::write(
                &mermaid_path,
                "// Placeholder - mermaid.js download failed during build\nthrow new Error('mermaid.js not available - download failed during build');\n",
            )
            .expect("Failed to write placeholder mermaid.js");
        }
    }

    // Emit path for inclusion
    println!("cargo:rustc-env=MERMAID_JS_PATH={}", mermaid_path.display());
}

#[cfg(feature = "js")]
fn download_file(url: &str, path: &std::path::PathBuf) -> Result<usize, String> {
    use std::fs;

    // Use std::process::Command to curl/wget as fallback
    // This avoids adding a build dependency on reqwest

    // Try curl first
    let output = std::process::Command::new("curl")
        .args(["-fsSL", "-o", path.to_str().unwrap(), url])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
            return Ok(metadata.len() as usize);
        }
    }

    // Try wget as fallback
    let output = std::process::Command::new("wget")
        .args(["-q", "-O", path.to_str().unwrap(), url])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
            return Ok(metadata.len() as usize);
        }
    }

    // Neither curl nor wget works
    Err("Neither curl nor wget available for download".to_string())
}
