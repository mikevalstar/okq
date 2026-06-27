//! `okq index` — (re)generate the `index.md` directory listings from the
//! concepts okq sees. See `docs/features/index-command.md`. Like `init`/`new`,
//! this writes into the bundle, but it manages only a fenced block
//! (`okq:index:begin`/`end`), preserving surrounding prose and the root
//! `index.md`'s `okf_version` frontmatter.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::IndexArgs;
use crate::error::AppError;
use crate::model::ConceptRecord;
use crate::templates::{INDEX_BEGIN, INDEX_END};
use crate::view::Corpus;

/// Schema tag stamped on every `index` JSON document.
pub const SCHEMA: &str = "okq.index/v1";

/// The `okq.index/v1` envelope.
#[derive(Debug, Serialize, JsonSchema)]
pub struct IndexOutput {
    /// Schema tag (`okq.index/v1`).
    pub schema: &'static str,
    /// Per-`index.md` result, in path order.
    pub files: Vec<IndexFile>,
}

/// One `index.md` touched (or that would be touched under `--check`).
#[derive(Debug, Serialize, JsonSchema)]
pub struct IndexFile {
    /// Path of the `index.md`, relative to the bundle root.
    pub path: String,
    /// `created` | `updated` | `unchanged`.
    pub verb: &'static str,
    /// Number of direct concepts listed in this `index.md`.
    pub concepts: usize,
}

/// Runs `index` against the bundle at `bundle_dir`. With `args.check`, computes
/// what would change but writes nothing.
pub fn run(bundle_dir: &Path, args: &IndexArgs, no_ignore: bool) -> Result<IndexOutput, AppError> {
    let corpus = Corpus::load(bundle_dir, no_ignore)?;
    let bundle = corpus.bundle();

    // Direct concepts per directory (relative dir -> records), and the set of
    // directories that contain concepts anywhere below them (so parents get a
    // navigable index too).
    let mut by_dir: BTreeMap<String, Vec<ConceptRecord>> = BTreeMap::new();
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    dirs.insert(String::new()); // the root always gets an index

    for concept in corpus.concepts() {
        let rec = ConceptRecord::from_concept(bundle, concept);
        let dir = parent_dir(&rec.path);
        by_dir.entry(dir.clone()).or_default().push(rec);
        for ancestor in ancestors(&dir) {
            dirs.insert(ancestor);
        }
    }
    for list in by_dir.values_mut() {
        list.sort_by(|a, b| a.path.cmp(&b.path));
    }

    let mut files = Vec::new();
    for dir in &dirs {
        let direct = by_dir.get(dir).map(Vec::as_slice).unwrap_or(&[]);
        let children: Vec<&String> = dirs
            .iter()
            .filter(|d| !d.is_empty() && &parent_dir(d) == dir)
            .collect();

        let block = build_block(&children, direct);
        let rel = if dir.is_empty() {
            "index.md".to_string()
        } else {
            format!("{dir}/index.md")
        };
        let target = bundle.root().join(&rel);

        let existed = target.exists();
        let new_content = if existed {
            let current = fs::read_to_string(&target)
                .map_err(|e| AppError::Io(format!("reading {}: {e}", target.display())))?;
            inject(&current, &block)
        } else {
            scaffold(dir, bundle.root(), &block)
        };

        let verb = if !existed {
            "created"
        } else {
            // Re-read for the comparison is cheap and avoids holding the string.
            let current = fs::read_to_string(&target).unwrap_or_default();
            if new_content != current {
                "updated"
            } else {
                "unchanged"
            }
        };

        if !args.check && verb != "unchanged" {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| AppError::Io(format!("creating {}: {e}", parent.display())))?;
            }
            fs::write(&target, &new_content)
                .map_err(|e| AppError::Io(format!("writing {}: {e}", target.display())))?;
        }

        files.push(IndexFile {
            path: rel,
            verb,
            concepts: direct.len(),
        });
    }

    Ok(IndexOutput {
        schema: SCHEMA,
        files,
    })
}

/// The parent directory of a forward-slash relative path (`""` for root-level).
fn parent_dir(rel: &str) -> String {
    match rel.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    }
}

/// All ancestor directories of `dir`, including the root (`""`).
fn ancestors(dir: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    if dir.is_empty() {
        return out;
    }
    let mut acc = String::new();
    for seg in dir.split('/') {
        if acc.is_empty() {
            acc = seg.to_string();
        } else {
            acc = format!("{acc}/{seg}");
        }
        out.push(acc.clone());
    }
    out
}

/// The last path segment (directory or file name).
fn last_seg(path: &str) -> &str {
    path.rsplit_once('/').map(|(_, s)| s).unwrap_or(path)
}

/// Builds the fenced listing block: child folders, then a concept table.
fn build_block(children: &[&String], direct: &[ConceptRecord]) -> String {
    let mut s = String::new();
    s.push_str(INDEX_BEGIN);
    s.push('\n');

    if !children.is_empty() {
        s.push_str("### Folders\n\n");
        for child in children {
            let name = last_seg(child);
            s.push_str(&format!("- [{name}/]({name}/)\n"));
        }
        s.push('\n');
    }

    if !direct.is_empty() {
        s.push_str("### Concepts\n\n| Title | File |\n|-------|------|\n");
        for rec in direct {
            let file = last_seg(&rec.path);
            let title = rec
                .title
                .as_deref()
                .filter(|t| !t.is_empty())
                .unwrap_or(&rec.id);
            s.push_str(&format!("| {} | [{file}]({file}) |\n", escape_cell(title)));
        }
    }

    s.push_str(INDEX_END);
    s
}

/// Escapes `|` so a title can't break the Markdown table.
fn escape_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Replaces the listing block in `existing`, or appends it if no markers exist.
fn inject(existing: &str, block: &str) -> String {
    match (existing.find(INDEX_BEGIN), existing.find(INDEX_END)) {
        (Some(b), Some(e)) if e >= b => {
            let end = e + INDEX_END.len();
            format!("{}{}{}", &existing[..b], block, &existing[end..])
        }
        _ => format!("{}\n\n{}\n", existing.trim_end(), block),
    }
}

/// A fresh `index.md` for a directory that has none. The root carries
/// `okf_version` (OKF §11); subdirectory indexes carry no frontmatter (§6).
fn scaffold(dir: &str, root: &Path, block: &str) -> String {
    if dir.is_empty() {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Knowledge base".to_string());
        format!("---\nokf_version: \"0.1\"\n---\n\n# {name}\n\n{block}\n")
    } else {
        format!("# {}\n\n{block}\n", last_seg(dir))
    }
}

/// Serializes the envelope as pretty JSON.
pub fn to_json(out: &IndexOutput) -> String {
    serde_json::to_string_pretty(out).expect("IndexOutput is always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_and_ancestors() {
        assert_eq!(parent_dir("adrs/0001.md"), "adrs");
        assert_eq!(parent_dir("top.md"), "");
        assert_eq!(ancestors("a/b/c"), vec!["", "a", "a/b", "a/b/c"]);
        assert_eq!(ancestors(""), vec![""]);
    }

    #[test]
    fn inject_replaces_between_markers() {
        let existing = format!("# Hi\n\n{INDEX_BEGIN}\nold\n{INDEX_END}\n\n## Keep\n");
        let out = inject(&existing, &format!("{INDEX_BEGIN}\nnew\n{INDEX_END}"));
        assert!(out.contains("## Keep"));
        assert!(out.contains("new"));
        assert!(!out.contains("old"));
        assert_eq!(out.matches(INDEX_BEGIN).count(), 1);
    }

    #[test]
    fn inject_appends_when_no_markers() {
        let out = inject("# Title\n\nprose\n", "BLOCK");
        assert!(out.starts_with("# Title"));
        assert!(out.trim_end().ends_with("BLOCK"));
    }
}
