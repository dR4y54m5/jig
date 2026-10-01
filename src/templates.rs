//! Embedded document templates and rendering assets.

use include_dir::{Dir, include_dir};

static TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates");
static ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub fn template(path: &str) -> Option<&'static str> {
    TEMPLATES.get_file(path)?.contents_utf8()
}

pub fn doc_template(kind: &str) -> Option<&'static str> {
    template(&format!("docs/{kind}.md"))
}

pub fn asset(name: &str) -> &'static str {
    ASSETS
        .get_file(name)
        .and_then(|f| f.contents_utf8())
        .unwrap_or_else(|| panic!("asset {name} is embedded"))
}

/// Replaces every `{{key}}` with its value.
pub fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Process;

    #[test]
    fn every_kind_has_a_template() {
        for kind in &Process::get().kinds {
            let template =
                doc_template(&kind.key).unwrap_or_else(|| panic!("no template for {}", kind.key));
            assert!(
                template.starts_with("# {{title}}\n"),
                "{} template must start with the title",
                kind.key
            );
        }
    }

    #[test]
    fn fill_replaces_every_occurrence() {
        assert_eq!(
            fill("{{a}} and {{a}}, {{b}}", &[("a", "x"), ("b", "y")]),
            "x and x, y"
        );
    }

    #[test]
    fn rendering_assets_are_embedded() {
        assert!(asset("bench.typ").contains("bench-doc"));
        assert!(asset("bench.lua").contains("doc-ref"));
    }
}
