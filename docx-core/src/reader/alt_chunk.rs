use std::io::Read;

use super::*;

impl ElementReader for AltChunk {
    fn read<R: Read>(
        r: &mut EventReader<R>,
        attrs: &[OwnedAttribute],
    ) -> Result<Self, ReaderError> {
        let mut id: Option<String> = None;
        for a in attrs {
            // w:altChunk carries the relationship as r:id (local name "id").
            if a.name.local_name == "id" {
                id = Some(a.value.clone());
            }
        }
        let id = id.ok_or(ReaderError::XMLReadError)?;

        // Drain any children (e.g. w:altChunkPr) up to the matching end
        // element so they are not re-dispatched by the body parser.
        loop {
            match r.next() {
                Ok(XmlEvent::EndElement { name }) if name.local_name == "altChunk" => break,
                Ok(XmlEvent::EndDocument) => break,
                Err(_) => return Err(ReaderError::XMLReadError),
                _ => {}
            }
        }

        Ok(AltChunk::new(id))
    }
}
