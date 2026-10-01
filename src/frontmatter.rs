//! Document-control front matter: the flat `key: value` subset of YAML that
//! jig writes, which GitHub renders as a table above the document.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrontMatter {
    entries: Vec<(String, String)>,
}

/// A document split into its front matter and body.
#[derive(Debug)]
pub struct Split<'a> {
    pub front: Option<FrontMatter>,
    pub body: &'a str,
    /// 1-based line number of the first body line.
    pub body_line: usize,
}

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl FrontMatter {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = value,
            None => self.entries.push((key.to_string(), value)),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(k, _)| k.as_str())
    }

    pub fn to_yaml(&self) -> String {
        let mut out = String::from("---\n");
        for (key, value) in &self.entries {
            out.push_str(key);
            out.push_str(": ");
            out.push_str(&yaml_value(value));
            out.push('\n');
        }
        out.push_str("---\n");
        out
    }
}

/// Splits a Markdown document into front matter and body. A document without
/// an opening `---` line has no front matter.
pub fn split(text: &str) -> Result<Split<'_>, ParseError> {
    let mut lines = text.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return Ok(Split {
            front: None,
            body: text,
            body_line: 1,
        });
    };
    if first.trim_end() != "---" {
        return Ok(Split {
            front: None,
            body: text,
            body_line: 1,
        });
    }

    let mut front = FrontMatter::default();
    let mut offset = first.len();
    for (index, line) in lines.enumerate() {
        let line_no = index + 2;
        offset += line.len();
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            return Ok(Split {
                front: Some(front),
                body: &text[offset..],
                body_line: line_no + 1,
            });
        }
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        let (key, value) = parse_line(trimmed).map_err(|message| ParseError {
            line: line_no,
            message,
        })?;
        front.entries.push((key, value));
    }
    Err(ParseError {
        line: 1,
        message: "front matter is not closed with a `---` line".into(),
    })
}

fn parse_line(line: &str) -> Result<(String, String), String> {
    let Some((key, rest)) = line.split_once(':') else {
        return Err(format!("expected `key: value`, found `{line}`"));
    };
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("invalid key `{key}`"));
    }
    Ok((key.to_string(), parse_value(rest.trim())?))
}

fn parse_value(raw: &str) -> Result<String, String> {
    if let Some(inner) = raw.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(other) => out.push(other),
                    None => return Err("unterminated escape in quoted value".into()),
                },
                '"' => {
                    let rest = chars.as_str().trim();
                    if !rest.is_empty() && !rest.starts_with('#') {
                        return Err(format!("unexpected text after quoted value: `{rest}`"));
                    }
                    return Ok(out);
                }
                c => out.push(c),
            }
        }
        return Err("unterminated quoted value".into());
    }
    if let Some(inner) = raw.strip_prefix('\'') {
        let Some(end) = find_single_quote_end(inner) else {
            return Err("unterminated quoted value".into());
        };
        return Ok(inner[..end].replace("''", "'"));
    }
    let value = match raw.find(" #") {
        Some(at) => &raw[..at],
        None => raw,
    };
    Ok(value.trim().to_string())
}

fn find_single_quote_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                i += 2;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn yaml_value(value: &str) -> String {
    let needs_quotes = value.is_empty()
        || value.contains(": ")
        || value.contains(" #")
        || value.ends_with(':')
        || value.starts_with(char::is_whitespace)
        || value.ends_with(char::is_whitespace)
        || value.starts_with([
            '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%',
            '@', '`',
        ])
        || value.contains('\n');
    if !needs_quotes {
        return value.to_string();
    }
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_quoted_values() {
        let text = "---\nid: EM4-SRS\ntitle: \"Scope: the core\"\nnote: 'it''s fine'\nstatus: draft # comment\n---\n# Body\n";
        let split = split(text).unwrap();
        let fm = split.front.unwrap();
        assert_eq!(fm.get("id"), Some("EM4-SRS"));
        assert_eq!(fm.get("title"), Some("Scope: the core"));
        assert_eq!(fm.get("note"), Some("it's fine"));
        assert_eq!(fm.get("status"), Some("draft"));
        assert_eq!(split.body, "# Body\n");
        assert_eq!(split.body_line, 7);
    }

    #[test]
    fn documents_without_front_matter_are_all_body() {
        let split = split("# Title\n\ntext\n").unwrap();
        assert!(split.front.is_none());
        assert_eq!(split.body_line, 1);
    }

    #[test]
    fn unclosed_front_matter_is_an_error() {
        assert!(split("---\nid: X\n# Title\n").is_err());
    }

    #[test]
    fn malformed_lines_report_their_line_number() {
        let err = split("---\nid: X\nnot a pair\n---\n").unwrap_err();
        assert_eq!(err.line, 3);
    }

    #[test]
    fn writing_then_parsing_round_trips() {
        let mut fm = FrontMatter::default();
        fm.set("id", "EM4-ADR-001");
        fm.set("title", "Decision: use a \"no_std\" core");
        fm.set("gate", "");
        fm.set("status", "draft");
        let text = format!("{}body\n", fm.to_yaml());
        assert_eq!(split(&text).unwrap().front.unwrap(), fm);
    }

    #[test]
    fn set_replaces_existing_values_in_place() {
        let mut fm = FrontMatter::default();
        fm.set("status", "draft");
        fm.set("revision", "A");
        fm.set("status", "released");
        assert_eq!(fm.keys().collect::<Vec<_>>(), ["status", "revision"]);
        assert_eq!(fm.get("status"), Some("released"));
    }
}
