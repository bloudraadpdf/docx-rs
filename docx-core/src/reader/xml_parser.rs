use std::collections::VecDeque;
use std::io::{BufReader, Read};

use quick_xml::encoding::Decoder;
use quick_xml::events::{BytesEnd, BytesRef, BytesStart, Event};
use quick_xml::Reader;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedName {
    pub local_name: String,
    pub namespace: Option<String>,
    pub prefix: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedAttribute {
    pub name: OwnedName,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Namespace {
    mappings: Vec<(String, String)>,
}

impl Namespace {
    pub fn empty() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }
}

impl IntoIterator for Namespace {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<(String, String)>;

    fn into_iter(self) -> Self::IntoIter {
        self.mappings.into_iter()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlEvent {
    StartElement {
        name: OwnedName,
        attributes: Vec<OwnedAttribute>,
        namespace: Namespace,
    },
    EndElement {
        name: OwnedName,
    },
    Characters(String),
    Whitespace(String),
    EndDocument,
}

pub struct EventReader<R: Read> {
    reader: Reader<BufReader<R>>,
    buf: Vec<u8>,
    pending: VecDeque<XmlEvent>,
    finished: bool,
}

impl<R: Read> EventReader<R> {
    pub fn new(reader: R) -> Self {
        let mut reader = Reader::from_reader(BufReader::new(reader));
        {
            let config = reader.config_mut();
            config.trim_text(false);
            config.check_end_names = true;
            config.expand_empty_elements = false;
        }
        Self {
            reader,
            buf: Vec::new(),
            pending: VecDeque::new(),
            finished: false,
        }
    }

    pub fn next(&mut self) -> Result<XmlEvent, quick_xml::Error> {
        self.read_next()
    }

    fn read_next(&mut self) -> Result<XmlEvent, quick_xml::Error> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }

        // Coalesce a contiguous run of textual data into a single event.
        // quick-xml 0.37+ splits entity and character references out of the
        // text stream as `GeneralRef` events; the readers, however, rely on
        // one `Characters`/`Whitespace` event per run (several take only the
        // first such event, e.g. field instructions), so text and references
        // are merged back together here, matching the previous contract.
        let mut text: Option<String> = None;

        loop {
            self.buf.clear();
            match self.reader.read_event_into(&mut self.buf)? {
                Event::Text(chunk) => {
                    let decoded = chunk.decode()?;
                    let unescaped = quick_xml::escape::unescape(&decoded)?;
                    text.get_or_insert_with(String::new).push_str(&unescaped);
                }
                Event::GeneralRef(reference) => {
                    append_reference(text.get_or_insert_with(String::new), &reference)?;
                }
                Event::Start(element) => {
                    let decoder = self.reader.decoder();
                    let start = Self::build_start_event(element, decoder)?;
                    return Ok(self.emit(text, start));
                }
                Event::Empty(element) => {
                    let decoder = self.reader.decoder();
                    let start = Self::build_start_event(element, decoder)?;
                    if let XmlEvent::StartElement { name, .. } = &start {
                        self.pending
                            .push_back(XmlEvent::EndElement { name: name.clone() });
                    }
                    return Ok(self.emit(text, start));
                }
                Event::End(element) => {
                    let name = build_name_from_end(&element)?;
                    return Ok(self.emit(text, XmlEvent::EndElement { name }));
                }
                Event::CData(chunk) => {
                    let decoded = self.reader.decoder().decode(chunk.as_ref())?.into_owned();
                    return Ok(self.emit(text, XmlEvent::Characters(decoded)));
                }
                Event::Eof => {
                    return Ok(match text {
                        Some(text) => {
                            self.pending.push_front(XmlEvent::EndDocument);
                            classify_text(text)
                        }
                        None => {
                            self.finished = true;
                            XmlEvent::EndDocument
                        }
                    });
                }
                Event::Decl(_) | Event::PI(_) | Event::Comment(_) | Event::DocType(_) => {
                    // Skip non-structural events; keep accumulating textual data.
                }
            }
        }
    }

    /// Emits any accumulated textual data as a single event, deferring
    /// `boundary` (the structural event that terminated the run) until the
    /// next read. When no text was accumulated, `boundary` is returned
    /// directly.
    fn emit(&mut self, text: Option<String>, boundary: XmlEvent) -> XmlEvent {
        match text {
            Some(text) => {
                self.pending.push_front(boundary);
                classify_text(text)
            }
            None => boundary,
        }
    }

    fn build_start_event(
        element: BytesStart<'_>,
        decoder: Decoder,
    ) -> Result<XmlEvent, quick_xml::Error> {
        let name = build_name_from_start(&element)?;
        let attributes = build_attributes(&element, decoder)?;
        Ok(XmlEvent::StartElement {
            name,
            attributes,
            namespace: Namespace::empty(),
        })
    }
}

impl<R: Read> Iterator for EventReader<R> {
    type Item = Result<XmlEvent, quick_xml::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.read_next() {
            Ok(XmlEvent::EndDocument) => {
                self.finished = true;
                Some(Ok(XmlEvent::EndDocument))
            }
            Ok(event) => Some(Ok(event)),
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}

fn classify_text(text: String) -> XmlEvent {
    if text.chars().all(char::is_whitespace) {
        XmlEvent::Whitespace(text)
    } else {
        XmlEvent::Characters(text)
    }
}

/// Resolves an entity or character reference (`&amp;`, `&#9;`, ...) into its
/// textual value and appends it to `out`. Predefined XML entities and
/// character references are resolved; an unknown named entity is kept verbatim
/// (as `&name;`) so downstream escape handling can resolve DOCX extensions such
/// as `&nbsp;`.
fn append_reference(out: &mut String, reference: &BytesRef<'_>) -> Result<(), quick_xml::Error> {
    let name = reference.decode()?;
    let entity = format!("&{name};");
    match quick_xml::escape::unescape(&entity) {
        Ok(resolved) => out.push_str(&resolved),
        Err(_) => out.push_str(&entity),
    }
    Ok(())
}

fn build_name_from_start(element: &BytesStart<'_>) -> Result<OwnedName, quick_xml::Error> {
    let name = element.name();
    Ok(split_qname(name.as_ref()))
}

fn build_name_from_end(element: &BytesEnd<'_>) -> Result<OwnedName, quick_xml::Error> {
    let name = element.name();
    Ok(split_qname(name.as_ref()))
}

fn split_qname(raw: &[u8]) -> OwnedName {
    let text = String::from_utf8_lossy(raw).into_owned();
    if let Some(idx) = text.find(':') {
        let prefix = text[..idx].to_string();
        let local = text[idx + 1..].to_string();
        OwnedName {
            local_name: local,
            namespace: None,
            prefix: Some(prefix),
        }
    } else {
        OwnedName {
            local_name: text,
            namespace: None,
            prefix: None,
        }
    }
}

fn build_attributes(
    element: &BytesStart<'_>,
    decoder: Decoder,
) -> Result<Vec<OwnedAttribute>, quick_xml::Error> {
    let mut attributes = Vec::new();
    for attr_result in element.attributes().with_checks(false) {
        let attr = attr_result.map_err(quick_xml::Error::from)?;
        let decoded = decoder.decode(attr.value.as_ref())?;
        let value = quick_xml::escape::unescape(&decoded)?.into_owned();
        let name = split_qname(attr.key.as_ref());
        attributes.push(OwnedAttribute { name, value });
    }
    Ok(attributes)
}
