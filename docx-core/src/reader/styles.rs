use super::*;
use crate::reader::{FromXML, ReaderError};
use std::io::Read;

use std::str::FromStr;

impl FromXML for Styles {
    fn from_xml<R: Read>(reader: R) -> Result<Self, ReaderError> {
        let mut parser = EventReader::new(reader);
        let mut styles = Self::default();
        loop {
            let e = parser.next();
            match e {
                Ok(XmlEvent::StartElement {
                    attributes, name, ..
                }) => {
                    let e = XMLElement::from_str(&name.local_name).unwrap();
                    match e {
                        XMLElement::Style => {
                            if let Ok(s) = Style::read(&mut parser, &attributes) {
                                styles = styles.add_style(s);
                            }
                            continue;
                        }
                        XMLElement::DocDefaults => {
                            if let Ok(d) = DocDefaults::read(&mut parser, &attributes) {
                                styles = styles.doc_defaults(d);
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
                Ok(XmlEvent::EndElement { name, .. }) => {
                    let e = XMLElement::from_str(&name.local_name).unwrap();
                    if let XMLElement::Styles = e {
                        break;
                    }
                }
                Err(_) => return Err(ReaderError::XMLReadError),
                _ => {}
            }
        }
        Ok(styles)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::types::*;
    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_from_xml() {
        let xml = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:style w:type="character" w:styleId="FootnoteTextChar">
        <w:name w:val="Footnote Text Char"></w:name>
        <w:rPr>
            <w:sz w:val="20"></w:sz>
            <w:szCs w:val="20"></w:szCs>
        </w:rPr>
        <w:uiPriority w:val="99"></w:uiPriority>
        <w:unhideWhenUsed></w:unhideWhenUsed>
        <w:basedOn w:val="DefaultParagraphFont"></w:basedOn>
        <w:link w:val="FootnoteText"></w:link>
        <w:uiPriority w:val="99"></w:uiPriority>
        <w:semiHidden></w:semiHidden>
    </w:style>
</w:styles>"#;
        let s = Styles::from_xml(xml.as_bytes()).unwrap();
        let mut styles = Styles::new();
        styles = styles.add_style(
            Style::new("FootnoteTextChar", StyleType::Character)
                .name("Footnote Text Char")
                .size(20)
                .based_on("DefaultParagraphFont")
                .link("FootnoteText"),
        );
        assert_eq!(s, styles);
    }

    // Real-world documents (e.g. commoncrawl 139072a81c4cc1f8 and
    // e43d5da09ec72595) carry a table style whose `<w:tblPr>` contains a
    // bare `<w:jc/>` with no `w:val`. Word tolerates this; the reader must
    // not index past the empty attribute list, and a missing `w:val` is
    // semantically identical to the element being absent (no alignment
    // override). Comparing the two parses isolates exactly that contract
    // without coupling to unrelated reader defaults.
    #[test]
    fn test_table_style_jc_without_val_attribute_does_not_panic() {
        let with_bare_jc = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:style w:type="table" w:styleId="TableNormal">
        <w:name w:val="Normal Table"></w:name>
        <w:tblPr>
            <w:jc/>
        </w:tblPr>
    </w:style>
</w:styles>"#;
        let without_jc = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:style w:type="table" w:styleId="TableNormal">
        <w:name w:val="Normal Table"></w:name>
        <w:tblPr>
        </w:tblPr>
    </w:style>
</w:styles>"#;
        let bare = Styles::from_xml(with_bare_jc.as_bytes()).expect("bare table-style jc parses");
        let absent = Styles::from_xml(without_jc.as_bytes()).expect("absent jc parses");
        assert_eq!(bare, absent);
    }
}
