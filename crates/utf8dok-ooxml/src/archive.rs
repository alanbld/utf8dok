//! Archive handling for DOCX/DOTX files
//!
//! DOCX and DOTX files are ZIP archives containing XML files and resources.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write, Seek};
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

    #[test]
    fn test_file_operations() {
        let mut archive = OoxmlArchive {
            files: HashMap::new(),
        };

        // Test set and get
        archive.set_string("test.xml", "<root/>");
        assert!(archive.contains("test.xml"));
        assert_eq!(archive.get_string("test.xml").unwrap(), Some("<root/>".to_string()));

        // Test remove
        archive.remove("test.xml");
        assert!(!archive.contains("test.xml"));
    }
}
