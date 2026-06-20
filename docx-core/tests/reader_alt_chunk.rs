use std::io::{Cursor, Write};

use docx_rs::*;

// Build a minimal .docx in memory from the given parts.
fn build_docx(parts: &[(&str, &[u8])]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in parts {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    cursor.into_inner()
}

// A body `<w:altChunk r:id="...">` referencing an embedded HTML part must be
// surfaced as `DocumentChild::AltChunk` with the part bytes and the inferred
// content type attached during reading.
#[test]
fn reads_alt_chunk_with_embedded_html_part() {
    let content_types = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="html" ContentType="text/html"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
    let root_rels = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let document = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:altChunk r:id="rIdChunk"/><w:sectPr/></w:body></w:document>"#;
    let document_rels = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdChunk" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/aFChunk" Target="afchunk.html"/></Relationships>"#;
    let html = br#"<html><body><p>Hello altChunk</p></body></html>"#;

    let bytes = build_docx(&[
        ("[Content_Types].xml", content_types),
        ("_rels/.rels", root_rels),
        ("word/document.xml", document),
        ("word/_rels/document.xml.rels", document_rels),
        ("word/afchunk.html", html),
    ]);

    let docx = read_docx(&bytes).expect("read minimal altChunk docx");
    let alt = docx
        .document
        .children
        .iter()
        .find_map(|child| match child {
            DocumentChild::AltChunk(alt) => Some(alt),
            _ => None,
        })
        .expect("body should contain an AltChunk child");
    assert_eq!(alt.id, "rIdChunk");
    assert_eq!(alt.content_type.as_deref(), Some("text/html"));
    let data = alt.data.as_ref().expect("altChunk part bytes attached");
    assert!(
        std::str::from_utf8(data).unwrap().contains("Hello altChunk"),
        "altChunk should carry the embedded HTML bytes"
    );
}
