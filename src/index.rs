//! The search index: a per-bundle Tantivy BM25 index over heading-delimited
//! sections, cached in the XDG cache directory (never in the bundle — ADR-0003).
//!
//! The index is a derived, rebuildable cache. A manifest of each concept file's
//! `(mtime, size)` plus a schema version decides staleness; any change (or
//! `--reindex`) triggers a full rebuild. `--ephemeral` (or an unwritable cache)
//! builds a transient in-memory index instead.

use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tantivy::schema::{Field, IndexRecordOption, STORED, Schema, TextFieldIndexing, TextOptions};
use tantivy::{Index, TantivyDocument};

use crate::error::AppError;
use crate::sections;
use crate::view::Corpus;

/// Bumped whenever the index schema or analysis changes, forcing a rebuild.
const INDEX_SCHEMA_VERSION: u32 = 1;

/// Field handles into the Tantivy schema.
#[derive(Clone, Copy)]
pub struct Fields {
    /// Section body — the primary ranked field; stored for snippets.
    pub body: Field,
    /// Concept title — indexed (boosted) for ranking; not stored (derived from the bundle).
    pub title: Field,
    /// Section heading — indexed (boosted) and stored.
    pub heading: Field,
    /// Concept id (stored locator).
    pub concept_id: Field,
    /// Section slug (stored locator).
    pub slug: Field,
    /// 1-based section line (stored locator).
    pub line: Field,
    /// Heading depth (stored locator).
    pub level: Field,
}

/// A built/opened index together with its field handles.
pub struct SearchIndex {
    /// The Tantivy index.
    pub index: Index,
    /// Field handles.
    pub fields: Fields,
}

fn build_schema() -> (Schema, Fields) {
    let mut sb = Schema::builder();
    // Lowercase + English stemming (recall), with positions so phrase queries work.
    let stemmed = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("en_stem")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );
    let body = sb.add_text_field("body", stemmed.clone().set_stored());
    let title = sb.add_text_field("title", stemmed.clone());
    let heading = sb.add_text_field("heading", stemmed.set_stored());
    let concept_id = sb.add_text_field("concept_id", STORED);
    let slug = sb.add_text_field("slug", STORED);
    let line = sb.add_u64_field("line", STORED);
    let level = sb.add_u64_field("level", STORED);
    let schema = sb.build();
    (
        schema,
        Fields {
            body,
            title,
            heading,
            concept_id,
            slug,
            line,
            level,
        },
    )
}

/// Opens a fresh index from the bundle, persisting it to the XDG cache unless
/// `ephemeral` (or the cache is unwritable), in which case it builds in RAM.
pub fn open_or_build(
    corpus: &Corpus,
    reindex: bool,
    ephemeral: bool,
) -> Result<SearchIndex, AppError> {
    let (schema, fields) = build_schema();

    if ephemeral {
        return build_in_ram(schema, fields, corpus);
    }

    let Some(cache) = cache_dir_for(corpus) else {
        eprintln!("okq: warning: no cache directory available; using an in-memory index");
        return build_in_ram(schema, fields, corpus);
    };
    let index_dir = cache.join("index");
    let manifest_path = cache.join("manifest.json");
    let want = current_manifest(corpus);

    // Reuse a fresh, matching index if one exists.
    if !reindex && index_dir.join("meta.json").exists() {
        if let Ok(text) = fs::read_to_string(&manifest_path) {
            if serde_json::from_str::<Manifest>(&text).ok().as_ref() == Some(&want) {
                if let Ok(index) = Index::open_in_dir(&index_dir) {
                    return Ok(SearchIndex { index, fields });
                }
            }
        }
    }

    // Otherwise (re)build. Fall back to RAM if the cache can't be written.
    if fs::create_dir_all(&index_dir).is_err() || clear_dir(&index_dir).is_err() {
        eprintln!("okq: warning: cache directory is not writable; using an in-memory index");
        return build_in_ram(schema, fields, corpus);
    }
    let index =
        Index::create_in_dir(&index_dir, schema).map_err(|e| AppError::Index(e.to_string()))?;
    add_concepts(&index, &fields, corpus)?;
    let _ = fs::write(
        &manifest_path,
        serde_json::to_string(&want).unwrap_or_default(),
    );
    Ok(SearchIndex { index, fields })
}

fn build_in_ram(schema: Schema, fields: Fields, corpus: &Corpus) -> Result<SearchIndex, AppError> {
    let index = Index::create_in_ram(schema);
    add_concepts(&index, &fields, corpus)?;
    Ok(SearchIndex { index, fields })
}

/// Indexes every section of every visible concept, in deterministic order.
/// `.okqignore`-hidden concepts are skipped (`corpus.concepts()` filters them).
fn add_concepts(index: &Index, fields: &Fields, corpus: &Corpus) -> Result<(), AppError> {
    let mut writer = index
        .writer(50_000_000)
        .map_err(|e| AppError::Index(e.to_string()))?;

    for c in corpus.concepts() {
        let raw = fs::read_to_string(&c.path)?;
        let start = sections::body_start_line(&raw);
        let title = crate::model::concept_title(c);
        let id = c.id.to_string();

        for sec in index_sections(&c.document.body, start, &title) {
            let mut doc = TantivyDocument::default();
            doc.add_text(fields.body, &sec.body);
            doc.add_text(fields.title, &title);
            doc.add_text(fields.heading, &sec.heading);
            doc.add_text(fields.concept_id, &id);
            doc.add_text(fields.slug, &sec.slug);
            doc.add_u64(fields.line, sec.line as u64);
            doc.add_u64(fields.level, sec.level as u64);
            writer
                .add_document(doc)
                .map_err(|e| AppError::Index(e.to_string()))?;
        }
    }

    writer
        .commit()
        .map_err(|e| AppError::Index(e.to_string()))?;
    Ok(())
}

struct IndexSection {
    heading: String,
    slug: String,
    level: u8,
    line: usize,
    body: String,
}

/// Sections to index for one body: the heading-delimited sections plus a
/// leading "preamble" covering any prose before the first heading (so nothing
/// is unsearchable). A doc with no headings becomes one whole-body section.
fn index_sections(body: &str, start: usize, title: &str) -> Vec<IndexSection> {
    let secs = sections::parse_sections(body, start);
    let body_lines: Vec<&str> = body.lines().collect();
    let mut out = Vec::new();

    let preamble = match secs.first() {
        Some(first) => {
            let n = first.line.saturating_sub(start).min(body_lines.len());
            body_lines[..n].join("\n")
        }
        None => body.to_string(),
    };
    if !preamble.trim().is_empty() {
        out.push(IndexSection {
            heading: title.to_string(),
            slug: String::new(),
            level: 0,
            line: start,
            body: preamble,
        });
    }

    for s in secs {
        out.push(IndexSection {
            heading: s.heading,
            slug: s.slug,
            level: s.level,
            line: s.line,
            body: s.body,
        });
    }
    out
}

/// The cache directory for this bundle: `<base>/<key>/`, keyed by the bundle's
/// canonical path. The base is `$OKQ_CACHE_DIR` if set (handy for relocating
/// the cache or for tests), else the per-platform XDG cache dir + `okq`.
/// `None` if no base is available.
fn cache_dir_for(corpus: &Corpus) -> Option<PathBuf> {
    let base = match std::env::var_os("OKQ_CACHE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => dirs::cache_dir()?.join("okq"),
    };
    let root = corpus.bundle().root();
    let canon = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canon.to_string_lossy().hash(&mut hasher);
    // `--no-ignore` sees a different concept set than the default, so it must
    // not share a cache with it — give the two modes separate directories.
    let suffix = if corpus.ignore().is_enabled() {
        ""
    } else {
        "-all"
    };
    Some(base.join(format!("{:016x}{suffix}", hasher.finish())))
}

#[derive(Serialize, Deserialize, PartialEq)]
struct Manifest {
    schema_version: u32,
    files: Vec<FileStamp>,
}

#[derive(Serialize, Deserialize, PartialEq)]
struct FileStamp {
    id: String,
    mtime_ns: u128,
    size: u64,
}

fn current_manifest(corpus: &Corpus) -> Manifest {
    // Stamp every visible concept...
    let mut files: Vec<FileStamp> = corpus
        .concepts()
        .filter_map(|c| stamp(c.id.to_string(), &c.path))
        .collect();
    // ...plus every `.okqignore` file, so editing the rules (which can change
    // which concepts are visible) invalidates the index and forces a rebuild.
    for path in corpus.ignore().files() {
        if let Some(s) = stamp(format!(".okqignore:{}", path.display()), path) {
            files.push(s);
        }
    }
    files.sort_by(|a, b| a.id.cmp(&b.id));
    Manifest {
        schema_version: INDEX_SCHEMA_VERSION,
        files,
    }
}

/// A `(mtime, size)` stamp for one file, or `None` if it can't be stat'd.
fn stamp(id: String, path: &Path) -> Option<FileStamp> {
    let meta = fs::metadata(path).ok()?;
    let mtime_ns = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(FileStamp {
        id,
        mtime_ns,
        size: meta.len(),
    })
}

/// Removes the contents of a directory (so `create_in_dir` sees it empty).
fn clear_dir(dir: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}
