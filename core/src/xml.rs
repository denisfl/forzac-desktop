//! A small event layer over quick-xml for the package and XHTML readers: local names, attributes with entities
//! resolved, text with named HTML entities resolved, and a parse error as a value the readers stop on.

use quick_xml::Reader;
use quick_xml::events::Event;

pub(crate) enum Ev {
    /// Local name (namespace prefix dropped) and attributes by local name.
    Start(String, Vec<(String, String)>),
    End,
    Text(String),
    Eof,
}

/// The document is broken from here on; the reader keeps what it has.
pub(crate) struct Broken;

pub(crate) type Res<T> = Result<T, Broken>;

pub(crate) struct Xml<'a> {
    reader: Reader<&'a [u8]>,
    buf: Vec<u8>,
    /// Only these attributes are read, when set: books carry long attributes nobody needs (`title` on every span).
    wanted: Option<&'static [&'static str]>,
}

impl<'a> Xml<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        let mut reader = Reader::from_reader(data);
        let config = reader.config_mut();
        config.expand_empty_elements = true;
        config.check_end_names = false; // broken books are common; nesting is tracked by the readers
        Xml { reader, buf: Vec::new(), wanted: None }
    }

    /// Reads only the attributes in `names`.
    pub(crate) fn with_attributes(mut self, names: &'static [&'static str]) -> Self {
        self.wanted = Some(names);
        self
    }

    pub(crate) fn next(&mut self) -> Res<Ev> {
        loop {
            self.buf.clear();
            let ev = match self.reader.read_event_into(&mut self.buf).map_err(|_| Broken)? {
                Event::Start(e) => {
                    let name = local(e.name().as_ref()).to_owned();
                    let wanted = self.wanted;
                    let attrs = e
                        .attributes()
                        .with_checks(false) // duplicate attributes are an author's slip, not a reason to stop
                        .flatten()
                        .filter(|a| wanted.is_none_or(|w| w.contains(&local(a.key.as_ref()))))
                        .map(|a| {
                            let raw = a.value.into_owned();
                            let value = quick_xml::escape::unescape(&raw).map(|v| v.into_owned()).unwrap_or(raw);
                            (local(a.key.as_ref()).to_owned(), value)
                        })
                        .collect();
                    Ev::Start(name, attrs)
                }
                Event::End(_) => Ev::End,
                Event::Text(e) => Ev::Text(e.xml10_content().into_owned()),
                Event::CData(e) => Ev::Text(e.into_inner().into_owned()),
                Event::GeneralRef(r) => match r.resolve_char_ref() {
                    Ok(Some(c)) => Ev::Text(c.to_string()),
                    _ => Ev::Text(crate::fb2::entity(&r.into_inner()).to_owned()),
                },
                Event::Eof => Ev::Eof,
                _ => continue,
            };
            return Ok(ev);
        }
    }

    /// Consumes the rest of the current element.
    pub(crate) fn skip(&mut self) -> Res<()> {
        let mut depth = 0;
        loop {
            match self.next()? {
                Ev::Start(..) => depth += 1,
                Ev::End if depth == 0 => return Ok(()),
                Ev::End => depth -= 1,
                Ev::Eof => return Err(Broken),
                Ev::Text(_) => {}
            }
        }
    }

    /// All text of the current element and its descendants, whitespace collapsed; consumes the element.
    pub(crate) fn text(&mut self) -> Res<String> {
        let mut out = String::new();
        let mut depth = 0;
        loop {
            match self.next()? {
                Ev::Text(t) => out.push_str(&t),
                Ev::Start(..) => depth += 1,
                Ev::End if depth == 0 => break,
                Ev::End => depth -= 1,
                Ev::Eof => break,
            }
        }
        Ok(collapse(&out))
    }
}

/// The name without its namespace prefix.
fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

pub(crate) fn attr<'v>(attrs: &'v [(String, String)], name: &str) -> Option<&'v str> {
    attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// Runs of whitespace as one space, trimmed.
pub(crate) fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
