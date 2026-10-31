//! Native Rust PDF parser and document text extractor using lopdf.
use crate::security::{verify_file_size, MAX_FILE_SIZE_BYTES};
use lopdf::Document;
use std::fs;
use std::path::Path;

const MAX_PAGES: usize = 100;

#[derive(Debug, Clone)]
pub struct ExtractedPage {
    pub page_number: u32,
    pub text: String,
    pub is_scanned_only: bool,
}

#[derive(Debug, Clone)]
pub struct PdfExtractionResult {
    pub total_pages: usize,
    pub pages: Vec<ExtractedPage>,
    pub combined_summary: String,
}

/// Safely inspects and extracts text from a PDF document.
pub fn extract_pdf_document(path: &Path) -> Result<PdfExtractionResult, String> {
    // 1. File size gate check using security module
    let metadata = fs::metadata(path).map_err(|e| format!("Failed to read file metadata: {e}"))?;
    verify_file_size(metadata.len(), MAX_FILE_SIZE_BYTES)
        .map_err(|e| format!("Security check failed: {e}"))?;

    let bytes = fs::read(path).map_err(|e| format!("Failed to read PDF file: {e}"))?;

    // 2. Load document into memory
    let doc =
        Document::load_mem(&bytes).map_err(|e| format!("Corrupt or invalid PDF structure: {e}"))?;
    let page_map = doc.get_pages();
    let total_pages = page_map.len();

    if total_pages == 0 {
        return Err("PDF document contains 0 pages".to_owned());
    }

    let mut pages = Vec::new();
    let mut combined_summary = String::new();

    // Sort page numbers to ensure strict reading order
    let mut page_nums: Vec<u32> = page_map.keys().copied().collect();
    page_nums.sort_unstable();

    for &page_num in page_nums.iter().take(MAX_PAGES) {
        let extracted_text = doc.extract_text(&[page_num]).unwrap_or_default();
        let trimmed = extracted_text.trim();

        let is_scanned_only = trimmed.is_empty();
        let text_content = if is_scanned_only {
            "[Scanned image only — native OCR unsupported offline]".to_owned()
        } else {
            trimmed.to_owned()
        };

        let page_entry = ExtractedPage {
            page_number: page_num,
            text: text_content.clone(),
            is_scanned_only,
        };

        combined_summary.push_str(&format!("--- Page {} ---\n{}\n\n", page_num, text_content));
        pages.push(page_entry);
    }

    Ok(PdfExtractionResult {
        total_pages,
        pages,
        combined_summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_extraction_non_existent() {
        let res = extract_pdf_document(Path::new("non_existent_file.pdf"));
        assert!(res.is_err());
    }
}
