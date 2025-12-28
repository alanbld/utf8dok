//! Archive handling for DOCX/DOTX files
//!
//! DOCX and DOTX files are ZIP archives containing XML files and resources.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

use zip::read::ZipArchive;
use zip::write::ZipWriter;
use zip::CompressionMethod;

use crate::error::{OoxmlError, Result};

/// Represents an unpacked OOXML document
#[derive(Debug)]
pub struct OoxmlArchive {
    /// All files in the archive, keyed by path
    files: HashMap<String, Vec<u8>>,
}

impl OoxmlArchive {
    /// Open and unpack a DOCX/DOTX file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }

    /// Create from any reader that implements Read + Seek
    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut archive = ZipArchive::new(reader)?;
        let mut files = HashMap::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            // Skip directories
            if name.ends_with('/') {
                continue;
            }

            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            files.insert(name, contents);
        }

        Ok(Self { files })
    }

    /// Get a file's contents by path
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(|v| v.as_slice())
    }

    /// Get a file's contents as a string
    pub fn get_string(&self, path: &str) -> Result<Option<String>> {
        match self.files.get(path) {
            Some(bytes) => {
                let s = String::from_utf8_lossy(bytes).into_owned();
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }

    /// Get the main document content (word/document.xml)
    pub fn document_xml(&self) -> Result<&[u8]> {
        self.get("word/document.xml")
            .ok_or_else(|| OoxmlError::MissingFile("word/document.xml".to_string()))
    }

    /// Get the styles definition (word/styles.xml)
    pub fn styles_xml(&self) -> Result<&[u8]> {
        self.get("word/styles.xml")
            .ok_or_else(|| OoxmlError::MissingFile("word/styles.xml".to_string()))
    }

    /// Get the numbering definitions (word/numbering.xml)
    pub fn numbering_xml(&self) -> Option<&[u8]> {
        self.get("word/numbering.xml")
    }

    /// Get the document relationships (word/_rels/document.xml.rels)
    pub fn document_rels_xml(&self) -> Option<&[u8]> {
        self.get("word/_rels/document.xml.rels")
    }

    /// Get a header file
    pub fn header_xml(&self, index: u32) -> Option<&[u8]> {
        self.get(&format!("word/header{}.xml", index))
    }

    /// Get a footer file
    pub fn footer_xml(&self, index: u32) -> Option<&[u8]> {
        self.get(&format!("word/footer{}.xml", index))
    }

    /// Check if a file exists in the archive
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// List all files in the archive
    pub fn file_list(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(|s| s.as_str())
    }

    /// Set or update a file's contents
    pub fn set(&mut self, path: impl Into<String>, contents: Vec<u8>) {
        self.files.insert(path.into(), contents);
    }

    /// Set a file's contents from a string
    pub fn set_string(&mut self, path: impl Into<String>, contents: impl Into<String>) {
        self.files.insert(path.into(), contents.into().into_bytes());
    }

    /// Remove a file from the archive
    pub fn remove(&mut self, path: &str) -> Option<Vec<u8>> {
        self.files.remove(path)
    }

    // =========================================================================
    // utf8dok container methods
    // =========================================================================

    /// Read a file from the utf8dok/ folder
    ///
    /// # Arguments
    /// * `path` - Relative path within utf8dok/ folder (e.g., "manifest.json")
    ///
    /// # Returns
    /// The file contents if found, None otherwise
    pub fn read_utf8dok_file(&self, path: &str) -> Option<&[u8]> {
        let full_path = format!("utf8dok/{}", path);
        self.get(&full_path)
    }

    /// Read a file from the utf8dok/ folder as a string
    pub fn read_utf8dok_string(&self, path: &str) -> Result<Option<String>> {
        let full_path = format!("utf8dok/{}", path);
        self.get_string(&full_path)
    }

    /// Write a file to the utf8dok/ folder
    ///
    /// # Arguments
    /// * `path` - Relative path within utf8dok/ folder (e.g., "manifest.json")
    /// * `contents` - File contents as bytes
    pub fn write_utf8dok_file(&mut self, path: &str, contents: Vec<u8>) {
        let full_path = format!("utf8dok/{}", path);
        self.files.insert(full_path, contents);
    }

    /// Write a string file to the utf8dok/ folder
    pub fn write_utf8dok_string(&mut self, path: &str, contents: impl Into<String>) {
        let full_path = format!("utf8dok/{}", path);
        self.files.insert(full_path, contents.into().into_bytes());
    }

    /// Check if a utf8dok file exists
    pub fn has_utf8dok_file(&self, path: &str) -> bool {
        let full_path = format!("utf8dok/{}", path);
        self.files.contains_key(&full_path)
    }

    /// List all files in the utf8dok/ folder
    pub fn list_utf8dok_files(&self) -> Vec<&str> {
        self.files
            .keys()
            .filter_map(|k| k.strip_prefix("utf8dok/"))
            .collect()
    }

    /// Check if this archive has any utf8dok content
    pub fn has_utf8dok_content(&self) -> bool {
        self.files.keys().any(|k| k.starts_with("utf8dok/"))
    }

    /// Get the manifest if it exists
    pub fn get_manifest(&self) -> Result<Option<crate::manifest::Manifest>> {
        match self.read_utf8dok_file("manifest.json") {
            Some(bytes) => {
                let manifest = crate::manifest::Manifest::from_json_bytes(bytes)?;
                Ok(Some(manifest))
            }
            None => Ok(None),
        }
    }

    /// Set the manifest
    pub fn set_manifest(&mut self, manifest: &crate::manifest::Manifest) -> Result<()> {
        let json = manifest.to_json_bytes()?;
        self.write_utf8dok_file("manifest.json", json);
        Ok(())
    }

    /// Write the archive to a file
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        self.write_to(file)
    }

    /// Write the archive to any writer
    pub fn write_to<W: Write + Seek>(&self, writer: W) -> Result<()> {
        let mut zip = ZipWriter::new(writer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        // Sort keys for deterministic output
        let mut paths: Vec<_> = self.files.keys().collect();
        paths.sort();

        for path in paths {
            let contents = &self.files[path];
            zip.start_file(path, options)?;
            zip.write_all(contents)?;
        }

        zip.finish()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ElementMeta, Manifest};

    #[test]
    fn test_file_operations() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Test set and get
        archive.set_string("test.xml", "<root/>");
        assert!(archive.contains("test.xml"));
        assert_eq!(
            archive.get_string("test.xml").unwrap(),
            Some("<root/>".to_string())
        );

        // Test remove
        archive.remove("test.xml");
        assert!(!archive.contains("test.xml"));
    }

    #[test]
    fn test_utf8dok_file_operations() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Initially no utf8dok content
        assert!(!archive.has_utf8dok_content());
        assert!(archive.list_utf8dok_files().is_empty());

        // Write a file to utf8dok folder
        archive.write_utf8dok_string("test.txt", "Hello, utf8dok!");

        // Verify it exists
        assert!(archive.has_utf8dok_file("test.txt"));
        assert!(archive.has_utf8dok_content());

        // Read it back
        let content = archive.read_utf8dok_string("test.txt").unwrap();
        assert_eq!(content, Some("Hello, utf8dok!".to_string()));

        // Verify it's in the full path
        assert!(archive.contains("utf8dok/test.txt"));

        // List files
        let files = archive.list_utf8dok_files();
        assert_eq!(files.len(), 1);
        assert!(files.contains(&"test.txt"));
    }

    #[test]
    fn test_manifest_integration() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Create and set manifest
        let mut manifest = Manifest::new();
        manifest.add_element(
            "fig1",
            ElementMeta::new("figure").with_source("utf8dok/diagrams/fig1.mmd"),
        );

        archive.set_manifest(&manifest).unwrap();

        // Verify manifest file exists
        assert!(archive.has_utf8dok_file("manifest.json"));

        // Read it back
        let restored = archive.get_manifest().unwrap().unwrap();
        assert_eq!(restored.version, "1.0");
        assert_eq!(restored.len(), 1);

        let elem = restored.get_element("fig1").unwrap();
        assert_eq!(elem.type_, "figure");
    }

    #[test]
    fn test_utf8dok_roundtrip_to_file() {
        use std::io::Cursor;

        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Add minimal DOCX structure
        archive.set_string("[Content_Types].xml", r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#);
        archive.set_string("word/document.xml", r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#);

        // Add utf8dok content
        let mut manifest = Manifest::new();
        manifest.add_element("test", ElementMeta::new("test"));
        archive.set_manifest(&manifest).unwrap();
        archive.write_utf8dok_string("data/test.json", r#"{"key": "value"}"#);

        // Write to buffer
        let mut buffer = Cursor::new(Vec::new());
        archive.write_to(&mut buffer).unwrap();

        // Read back
        buffer.set_position(0);
        let restored = OoxmlArchive::from_reader(buffer).unwrap();

        // Verify utf8dok content survived
        assert!(restored.has_utf8dok_content());
        assert!(restored.has_utf8dok_file("manifest.json"));
        assert!(restored.has_utf8dok_file("data/test.json"));

        let restored_manifest = restored.get_manifest().unwrap().unwrap();
        assert_eq!(restored_manifest.len(), 1);
    }
}
