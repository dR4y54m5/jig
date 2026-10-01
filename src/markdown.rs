//! Line-level Markdown helpers. They understand fenced code blocks, HTML
//! comments, ATX headings and inline links; nothing more is needed to lint
//! and render engineering documents.

use std::sync::LazyLock;

use regex::{Captures, Regex};

static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(!?)\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)"#).unwrap());
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#{1,6})\s+(.*?)\s*#*\s*$").unwrap());
static INLINE_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`+[^`]*`+").unwrap());

#[derive(Debug, Clone)]
pub struct Line<'a> {
    /// 1-based line number in the whole file.
    pub no: usize,
    pub text: &'a str,
    /// Inside a fenced code block, or one of its fence lines.
    pub code: bool,
}

#[derive(Debug, Clone)]
pub struct Fence {
    pub lang: String,
    /// Line number of the opening fence.
    pub line: usize,
    pub content: String,
    /// Line number of the closing fence, if the block is closed.
    pub end_line: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Heading {
    pub line: usize,
    pub level: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub image: bool,
    pub target: String,
}

fn fence_marker(line: &str) -> Option<(char, usize, &str)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let count = rest.chars().take_while(|c| *c == ch).count();
    (count >= 3).then(|| (ch, count, rest[count..].trim()))
}

/// Every line of `body`, marked as code or not. `first_line` is the file line
/// number of the first body line.
pub fn lines(body: &str, first_line: usize) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    let mut open: Option<(char, usize)> = None;
    for (i, text) in body.lines().enumerate() {
        let no = first_line + i;
        match open {
            Some((ch, count)) => {
                if let Some((c, n, info)) = fence_marker(text)
                    && c == ch
                    && n >= count
                    && info.is_empty()
                {
                    open = None;
                }
                out.push(Line {
                    no,
                    text,
                    code: true,
                });
            }
            None => {
                if let Some((ch, count, _)) = fence_marker(text) {
                    open = Some((ch, count));
                    out.push(Line {
                        no,
                        text,
                        code: true,
                    });
                } else {
                    out.push(Line {
                        no,
                        text,
                        code: false,
                    });
                }
            }
        }
    }
    out
}

pub fn fences(body: &str, first_line: usize) -> Vec<Fence> {
    let mut out: Vec<Fence> = Vec::new();
    let mut open: Option<(char, usize)> = None;
    for (i, text) in body.lines().enumerate() {
        let no = first_line + i;
        match open {
            Some((ch, count)) => {
                let closes = fence_marker(text)
                    .is_some_and(|(c, n, info)| c == ch && n >= count && info.is_empty());
                let fence = out.last_mut().expect("an open fence was pushed");
                if closes {
                    open = None;
                    fence.end_line = Some(no);
                } else {
                    fence.content.push_str(text);
                    fence.content.push('\n');
                }
            }
            None => {
                if let Some((ch, count, info)) = fence_marker(text) {
                    open = Some((ch, count));
                    let lang = info.split_whitespace().next().unwrap_or("").to_string();
                    out.push(Fence {
                        lang,
                        line: no,
                        content: String::new(),
                        end_line: None,
                    });
                }
            }
        }
    }
    out
}

/// Lines of prose: outside code blocks, with HTML comments and inline code
/// spans removed.
pub fn prose(body: &str, first_line: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_comment = false;
    for line in lines(body, first_line) {
        if line.code {
            continue;
        }
        let mut text = String::new();
        let mut rest = line.text;
        loop {
            if in_comment {
                match rest.find("-->") {
                    Some(end) => {
                        in_comment = false;
                        rest = &rest[end + 3..];
                    }
                    None => break,
                }
            } else {
                match rest.find("<!--") {
                    Some(start) => {
                        text.push_str(&rest[..start]);
                        in_comment = true;
                        rest = &rest[start + 4..];
                    }
                    None => {
                        text.push_str(rest);
                        break;
                    }
                }
            }
        }
        let text = INLINE_CODE.replace_all(&text, "").into_owned();
        if !text.trim().is_empty() {
            out.push((line.no, text));
        }
    }
    out
}

/// Line numbers of `<!-- guide: ... -->` authoring comments left in a document.
pub fn guide_comments(body: &str, first_line: usize) -> Vec<usize> {
    lines(body, first_line)
        .into_iter()
        .filter(|l| !l.code && without_code_spans(l.text).contains("<!-- guide:"))
        .map(|l| l.no)
        .collect()
}

pub fn headings(body: &str, first_line: usize) -> Vec<Heading> {
    lines(body, first_line)
        .into_iter()
        .filter(|l| !l.code)
        .filter_map(|l| {
            let caps = HEADING.captures(l.text)?;
            Some(Heading {
                line: l.no,
                level: caps[1].len(),
                text: caps[2].to_string(),
            })
        })
        .collect()
}

/// `text` with inline code spans removed.
pub fn without_code_spans(text: &str) -> String {
    INLINE_CODE.replace_all(text, "").into_owned()
}

pub fn links(text: &str) -> Vec<Link> {
    LINK.captures_iter(text)
        .map(|c| Link {
            image: &c[1] == "!",
            target: c[3].to_string(),
        })
        .collect()
}

/// Rewrites link targets outside code blocks. `rewrite` returns the new
/// target, or `None` to leave the link unchanged.
pub fn rewrite_links(body: &str, rewrite: impl Fn(&Link) -> Option<String>) -> String {
    let mut out = String::with_capacity(body.len());
    for line in lines(body, 1) {
        if line.code {
            out.push_str(line.text);
        } else {
            let replaced = LINK.replace_all(line.text, |c: &Captures| {
                let link = Link {
                    image: &c[1] == "!",
                    target: c[3].to_string(),
                };
                match rewrite(&link) {
                    Some(target) => format!("{}[{}]({})", &c[1], &c[2], target),
                    None => c[0].to_string(),
                }
            });
            out.push_str(&replaced);
        }
        out.push('\n');
    }
    out
}

/// A heading's text reduced to the form used to compare section names.
pub fn section_name(heading: &str) -> String {
    let without_number =
        heading.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
    without_number.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Title\n\nProse `let's` here.\n<!-- guide: hidden -->\n```mermaid\nflowchart LR\n  a --> b\n```\n\n## 2. Next <!-- inline -->\nAfter [link](other.md) and ![img](x.svg).\n";

    #[test]
    fn prose_skips_code_comments_and_inline_code() {
        let prose = prose(DOC, 1);
        let texts: Vec<_> = prose.iter().map(|(_, t)| t.trim().to_string()).collect();
        assert!(texts.contains(&"Prose  here.".to_string()));
        assert!(
            !texts
                .iter()
                .any(|t| t.contains("flowchart") || t.contains("hidden") || t.contains("let's"))
        );
        assert!(texts.contains(&"## 2. Next".to_string()));
    }

    #[test]
    fn fences_capture_language_and_content() {
        let fences = fences(DOC, 10);
        assert_eq!(fences.len(), 1);
        assert_eq!(fences[0].lang, "mermaid");
        assert_eq!(fences[0].line, 14);
        assert_eq!(fences[0].content, "flowchart LR\n  a --> b\n");
        assert_eq!(fences[0].end_line, Some(17));
    }

    #[test]
    fn guide_comments_are_found_outside_code() {
        assert_eq!(guide_comments(DOC, 1), vec![4]);
    }

    #[test]
    fn headings_are_parsed_with_levels() {
        let hs = headings(DOC, 1);
        assert_eq!(hs[0].level, 1);
        assert_eq!(hs[1].text, "2. Next <!-- inline -->");
        assert_eq!(section_name("7. Revision history"), "revision history");
    }

    #[test]
    fn guide_comments_quoted_in_code_spans_are_ignored() {
        assert!(guide_comments("Replace `<!-- guide: -->` comments.\n", 1).is_empty());
    }

    #[test]
    fn code_spans_are_removed() {
        assert_eq!(without_code_spans("report `shall` once"), "report  once");
    }

    #[test]
    fn links_distinguish_images() {
        let found = links("See [a](a.md) and ![b](b.svg \"t\").");
        assert_eq!(found.len(), 2);
        assert!(!found[0].image && found[1].image);
        assert_eq!(found[1].target, "b.svg");
    }

    #[test]
    fn rewrite_links_leaves_code_blocks_alone() {
        let body = "[a](a.md)\n```\n[b](b.md)\n```\n";
        let out = rewrite_links(body, |l| {
            (l.target.ends_with(".md")).then(|| "#doc:X".to_string())
        });
        assert_eq!(out, "[a](#doc:X)\n```\n[b](b.md)\n```\n");
    }
}
