use serde::Serialize;
use std::io::Write;

use crate::documents::BuildXML;
use crate::xml_builder::*;

/// `<w:altChunk>` — an "alternative format import" that embeds an external
/// part (HTML, XHTML, MHTML, ...) into the document body at this position.
///
/// The element itself only carries the relationship id (`r:id`). The
/// referenced part's raw bytes and its MIME type (inferred from the part
/// extension) are resolved from the package while reading; see the
/// `aFChunk` handling in `read_docx`.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AltChunk {
    /// Relationship id (`r:id`) of the embedded part.
    pub id: String,
    /// MIME type inferred from the part's extension, once resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Raw bytes of the embedded part, once resolved.
    #[serde(skip)]
    pub data: Option<Vec<u8>>,
}

impl AltChunk {
    pub fn new(id: impl Into<String>) -> AltChunk {
        AltChunk {
            id: id.into(),
            content_type: None,
            data: None,
        }
    }

    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }
}

impl BuildXML for AltChunk {
    fn build_to<W: Write>(
        &self,
        stream: crate::xml::writer::EventWriter<W>,
    ) -> crate::xml::writer::Result<crate::xml::writer::EventWriter<W>> {
        XMLBuilder::from(stream).alt_chunk(&self.id)?.into_inner()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[cfg(test)]
    use pretty_assertions::assert_eq;
    use std::str;

    #[test]
    fn test_alt_chunk() {
        let b = AltChunk::new("rId5").build();
        assert_eq!(
            str::from_utf8(&b).unwrap(),
            r#"<w:altChunk r:id="rId5" />"#
        );
    }
}
