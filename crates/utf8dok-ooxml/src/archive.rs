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

    /// Get the core document properties (docProps/core.xml)
    pub fn core_properties_xml(&self) -> Option<&[u8]> {
        self.get("docProps/core.xml")
    }

    /// Get the comments (word/comments.xml)
    pub fn comments_xml(&self) -> Option<&[u8]> {
        self.get("word/comments.xml")
    }

    /// Check if a file exists in the archive
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Check if a file exists in the archive (alias for `contains`)
    pub fn has_file(&self, path: &str) -> bool {
        self.contains(path)
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

    // ==================== Additional Coverage Tests ====================

    #[test]
    fn test_get_basic() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Test get on non-existent file
        assert!(archive.get("missing.xml").is_none());

        // Add file and verify get works
        archive.set("test.xml", b"<test/>".to_vec());
        assert_eq!(archive.get("test.xml"), Some(b"<test/>".as_slice()));
    }

    #[test]
    fn test_get_string_empty_file() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        archive.set("empty.txt", Vec::new());
        let result = archive.get_string("empty.txt").unwrap();
        assert_eq!(result, Some(String::new()));
    }

    #[test]
    fn test_get_string_non_existent() {
        let archive = OoxmlArchive {
            files: HashMap::new(),
        };

        let result = archive.get_string("missing.txt").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_document_xml_missing() {
        let archive = OoxmlArchive {
            files: HashMap::new(),
        };

        let result = archive.document_xml();
        assert!(result.is_err());
        if let Err(OoxmlError::MissingFile(path)) = result {
            assert_eq!(path, "word/document.xml");
        } else {
            panic!("Expected MissingFile error");
        }
    }

    #[test]
    fn test_styles_xml_missing() {
        let archive = OoxmlArchive {
            files: HashMap::new(),
        };

        let result = archive.styles_xml();
        assert!(result.is_err());
        if let Err(OoxmlError::MissingFile(path)) = result {
            assert_eq!(path, "word/styles.xml");
        } else {
            panic!("Expected MissingFile error");
        }
    }

    #[test]
    fn test_optional_xml_files() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // All optional files should return None when missing
        assert!(archive.numbering_xml().is_none());
        assert!(archive.document_rels_xml().is_none());
        assert!(archive.header_xml(1).is_none());
        assert!(archive.footer_xml(1).is_none());
        assert!(archive.core_properties_xml().is_none());
        assert!(archive.comments_xml().is_none());

        // Add header and footer
        archive.set_string("word/header1.xml", "<header/>");
        archive.set_string("word/footer1.xml", "<footer/>");

        assert!(archive.header_xml(1).is_some());
        assert!(archive.footer_xml(1).is_some());
        // Different index still returns None
        assert!(archive.header_xml(2).is_none());
        assert!(archive.footer_xml(2).is_none());
    }

    #[test]
    fn test_file_list() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        archive.set_string("a.xml", "a");
        archive.set_string("b.xml", "b");
        archive.set_string("c.xml", "c");

        let files: Vec<&str> = archive.file_list().collect();
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"a.xml"));
        assert!(files.contains(&"b.xml"));
        assert!(files.contains(&"c.xml"));
    }

    #[test]
    fn test_has_file_alias() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        archive.set_string("test.xml", "content");

        // Both methods should return the same result
        assert_eq!(archive.contains("test.xml"), archive.has_file("test.xml"));
        assert_eq!(archive.contains("missing.xml"), archive.has_file("missing.xml"));
    }

    #[test]
    fn test_set_binary() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Set binary data
        let binary = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];
        archive.set("binary.bin", binary.clone());

        assert_eq!(archive.get("binary.bin"), Some(binary.as_slice()));
    }

    #[test]
    fn test_remove_returns_content() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        archive.set_string("test.xml", "content");

        let removed = archive.remove("test.xml");
        assert_eq!(removed, Some(b"content".to_vec()));

        // Removing again returns None
        assert_eq!(archive.remove("test.xml"), None);
    }

    #[test]
    fn test_read_utf8dok_file_bytes() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        let binary = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
        archive.write_utf8dok_file("image.png", binary.clone());

        let read = archive.read_utf8dok_file("image.png");
        assert_eq!(read, Some(binary.as_slice()));
    }

    #[test]
    fn test_get_manifest_when_missing() {
        let archive = OoxmlArchive {
            files: HashMap::new(),
        };

        let result = archive.get_manifest().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_utf8dok_files() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        archive.write_utf8dok_string("file1.txt", "one");
        archive.write_utf8dok_string("file2.txt", "two");
        archive.write_utf8dok_string("subdir/file3.txt", "three");

        let files = archive.list_utf8dok_files();
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"file1.txt"));
        assert!(files.contains(&"file2.txt"));
        assert!(files.contains(&"subdir/file3.txt"));
    }

    // ==================== Sprint 11: Archive Round-Trip Tests ====================

    #[test]
    fn test_manifest_roundtrip() {
        use crate::manifest::Manifest;

        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Create a manifest with content
        let mut manifest = Manifest::new();
        manifest.add_element(
            "source",
            crate::manifest::ElementMeta::new("source.adoc")
                .with_hash("abc123"),
        );

        // Set manifest
        archive.set_manifest(&manifest).unwrap();

        // Get manifest back
        let retrieved = archive.get_manifest().unwrap();
        assert!(retrieved.is_some());

        let retrieved_manifest = retrieved.unwrap();
        assert!(retrieved_manifest.get_element("source").is_some());
    }

    #[test]
    fn test_has_utf8dok_content() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Empty archive has no utf8dok content
        assert!(!archive.has_utf8dok_content());

        // Add utf8dok file
        archive.write_utf8dok_string("source.adoc", "= Title");
        assert!(archive.has_utf8dok_content());
    }

    #[test]
    fn test_utf8dok_string_roundtrip_utf8() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // UTF-8 content with special characters
        let content = "= Título con ñ y 日本語 and emoji 🎉";
        archive.write_utf8dok_string("source.adoc", content);

        let retrieved = archive.read_utf8dok_string("source.adoc").unwrap();
        assert_eq!(retrieved, Some(content.to_string()));
    }

    #[test]
    fn test_utf8dok_binary_roundtrip() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Binary content (simulated image)
        let binary = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        archive.write_utf8dok_file("image.png", binary.clone());

        let retrieved = archive.read_utf8dok_file("image.png");
        assert_eq!(retrieved, Some(binary.as_slice()));
    }

    #[test]
    fn test_list_utf8dok_files_empty() {
        let archive = OoxmlArchive {
            files: HashMap::new(),
        };

        let files = archive.list_utf8dok_files();
        assert!(files.is_empty());
    }

    #[test]
    fn test_full_self_contained_structure() {
        use crate::manifest::Manifest;

        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Set up complete self-contained structure
        archive.write_utf8dok_string("source.adoc", "= My Document\n\nContent here.");
        archive.write_utf8dok_string("utf8dok.toml", "[template]\npath = \"t.dotx\"");

        let mut manifest = Manifest::new();
        manifest.add_element(
            "source",
            crate::manifest::ElementMeta::new("source.adoc"),
        );
        manifest.add_element(
            "config",
            crate::manifest::ElementMeta::new("utf8dok.toml"),
        );
        archive.set_manifest(&manifest).unwrap();

        // Verify all parts exist
        assert!(archive.has_utf8dok_content());
        let files = archive.list_utf8dok_files();
        assert!(files.contains(&"source.adoc"));
        assert!(files.contains(&"utf8dok.toml"));
        assert!(files.contains(&"manifest.json"));

        // Verify manifest has correct entries
        let retrieved_manifest = archive.get_manifest().unwrap().unwrap();
        assert!(retrieved_manifest.get_element("source").is_some());
        assert!(retrieved_manifest.get_element("config").is_some());
    }

    #[test]
    fn test_overwrite_utf8dok_file() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        archive.write_utf8dok_string("source.adoc", "version 1");
        assert_eq!(
            archive.read_utf8dok_string("source.adoc").unwrap(),
            Some("version 1".to_string())
        );

        // Overwrite
        archive.write_utf8dok_string("source.adoc", "version 2");
        assert_eq!(
            archive.read_utf8dok_string("source.adoc").unwrap(),
            Some("version 2".to_string())
        );
    }

    #[test]
    fn test_archive_file_paths_case_sensitive() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        archive.set_string("Word/Document.xml", "upper");
        archive.set_string("word/document.xml", "lower");

        // Both should exist as separate files
        assert_eq!(
            archive.get_string("Word/Document.xml").unwrap(),
            Some("upper".to_string())
        );
        assert_eq!(
            archive.get_string("word/document.xml").unwrap(),
            Some("lower".to_string())
        );
    }

    #[test]
    fn test_get_string_invalid_utf8() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Set invalid UTF-8 bytes
        let invalid_utf8 = vec![0xFF, 0xFE, 0x00, 0x01];
        archive.set("invalid.txt", invalid_utf8);

        // get_string uses from_utf8_lossy, so invalid UTF-8 is replaced with replacement chars
        let result = archive.get_string("invalid.txt").unwrap();
        assert!(result.is_some());
        // The lossy conversion replaces invalid bytes with the replacement character
        let content = result.unwrap();
        assert!(content.contains('\u{FFFD}')); // Unicode replacement character
    }
}
