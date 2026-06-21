use std::io::Read;
use std::str::FromStr;

use super::*;
use crate::types::*;

impl ElementReader for Level {
    fn read<R: Read>(
        r: &mut EventReader<R>,
        attrs: &[OwnedAttribute],
    ) -> Result<Self, ReaderError> {
        let level = read_indent_level(attrs)?;
        let mut style_id = None;
        let mut ppr = ParagraphProperty::new();
        let mut rpr = RunProperty::new();
        let mut start = Start::default();
        let mut num_fmt = NumberFormat::new("decimal");
        let mut level_text = LevelText::new("");
        let mut jc = LevelJc::new("left");

        let mut indent_start = None;
        let mut special_indent = None;
        let mut indent_end = None;
        let mut start_chars = None;
        let mut level_restart = None;
        let mut has_indent = false;
        let mut suffix = LevelSuffixType::Tab;
        let mut is_lgl = None;

        loop {
            let e = r.next();
            match e {
                Ok(XmlEvent::StartElement {
                    attributes, name, ..
                }) => {
                    let e = XMLElement::from_str(&name.local_name).unwrap();
                    match e {
                        XMLElement::ParagraphStyle => {
                            // A bare `<w:pStyle/>` (no `w:val`) names no
                            // style; do not index past the empty list.
                            if let Some(id) = read_val(&attributes) {
                                style_id = Some(id);
                            }
                        }
                        XMLElement::ParagraphProperty => {
                            if let Ok(pr) = ParagraphProperty::read(r, attrs) {
                                ppr = pr;
                            }
                            continue;
                        }
                        XMLElement::RunProperty => {
                            if let Ok(pr) = RunProperty::read(r, attrs) {
                                rpr = pr;
                            }
                            continue;
                        }
                        XMLElement::Start => {
                            // `w:start/@w:val` is the first ordinal. A bare
                            // `<w:start/>` keeps the default; an unparseable
                            // value is ignored rather than aborting the read.
                            if let Some(v) = read_val(&attributes)
                                .and_then(|s| usize::from_str(&s).ok())
                            {
                                start = Start::new(v);
                            }
                        }
                        XMLElement::NumberFormat => {
                            // `w:numFmt/@w:val` is the format token. A bare
                            // `<w:numFmt/>` keeps the `decimal` default.
                            if let Some(v) = read_val(&attributes) {
                                num_fmt = NumberFormat::new(v);
                            }
                        }
                        XMLElement::Suffix => {
                            // `w:suff/@w:val` is the level suffix. A bare
                            // `<w:suff/>` keeps the `tab` default; an unknown
                            // value is ignored rather than aborting the read.
                            if let Some(v) = read_val(&attributes)
                                .and_then(|s| LevelSuffixType::from_str(&s).ok())
                            {
                                suffix = v;
                            }
                        }
                        XMLElement::IsLgl => {
                            is_lgl = Some(IsLgl::new());
                        }
                        XMLElement::LevelText => {
                            // `w:lvlText` carries the numbering text format in
                            // its `w:val` attribute. A bare `<w:lvlText/>` (no
                            // `w:val`), which Word tolerates, must not index
                            // past the empty attribute list; treat it as the
                            // empty format string.
                            level_text = LevelText::new(read_val(&attributes).unwrap_or_default());
                        }
                        XMLElement::LevelRestart => {
                            // `w:lvlRestart/@w:val` is optional; a bare
                            // element or an unparseable value leaves it unset.
                            if let Some(v) = read_val(&attributes)
                                .and_then(|s| u32::from_str(&s).ok())
                            {
                                level_restart = Some(LevelRestart::new(v));
                            }
                        }
                        XMLElement::LevelJustification => {
                            // `w:lvlJc/@w:val` is the alignment. A bare
                            // `<w:lvlJc/>` keeps the `left` default.
                            if let Some(v) = read_val(&attributes) {
                                jc = LevelJc::new(v);
                            }
                        }
                        XMLElement::Indent => {
                            let i = read_indent(&attributes)?;
                            indent_start = i.0;
                            indent_end = i.1;
                            special_indent = i.2;
                            start_chars = i.3;
                            has_indent = true;
                        }
                        _ => {}
                    }
                }
                Ok(XmlEvent::EndElement { name, .. }) => {
                    let e = XMLElement::from_str(&name.local_name).unwrap();
                    if let XMLElement::Level = e {
                        let mut l =
                            Level::new(level, start, num_fmt, level_text, jc).suffix(suffix);
                        if let Some(style_id) = style_id {
                            l = l.paragraph_style(style_id);
                        }
                        if has_indent {
                            l = l.indent(indent_start, special_indent, indent_end, start_chars);
                        }
                        l.paragraph_property = ppr;
                        l.run_property = rpr;
                        l.level_restart = level_restart;
                        l.is_lgl = is_lgl;
                        return Ok(l);
                    }
                }
                Err(_) => return Err(ReaderError::XMLReadError),
                _ => {}
            }
        }
    }
}
