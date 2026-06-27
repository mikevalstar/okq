//! `.okqignore` support — excluding files from a bundle (ADR-0006).
//!
//! okf treats every non-reserved `.md` file under the bundle root as a concept.
//! A bundle's tree, though, often holds files that aren't really concepts:
//! deliberately malformed test fixtures, drafts, vendored copies. An
//! [`IgnoreSet`] loads the `.okqignore` files in a tree and answers one
//! question — *is this path excluded?* — using full `.gitignore` semantics via
//! the `ignore` crate.
//!
//! Filtering itself lives in the query layer (see `view.rs`): okf stays unaware
//! of exclusion, and okq applies the [`IgnoreSet`] to the concept list okf
//! returns. `--no-ignore` loads a disabled set that excludes nothing.

use std::path::{Path, PathBuf};

use ::ignore::Match;
use ::ignore::gitignore::{Gitignore, GitignoreBuilder};

/// The per-directory ignore filename, mirroring `.gitignore`.
pub const IGNORE_FILENAME: &str = ".okqignore";

/// The `.okqignore` rules discovered in a bundle tree, ready to test paths.
///
/// Nested files are supported: a `.okqignore` in any directory governs that
/// directory and below. Matchers are applied **deepest-directory-first**, and
/// the first decisive match (ignore or whitelist) wins — so a nested file
/// overrides a shallower one, and within a single file the last matching
/// pattern wins (the `ignore` crate's own rule).
pub struct IgnoreSet {
    /// `(directory, matcher)` pairs, sorted deepest-directory-first.
    matchers: Vec<(PathBuf, Gitignore)>,
    /// Absolute paths of the `.okqignore` files found, sorted — fed into the
    /// search index's staleness manifest so editing rules rebuilds the index.
    files: Vec<PathBuf>,
    /// `false` when `--no-ignore` disabled the feature for this invocation.
    enabled: bool,
}

impl IgnoreSet {
    /// Loads every `.okqignore` under `root`. When `disabled` (the `--no-ignore`
    /// path), returns a no-op set that excludes nothing and lists no files.
    ///
    /// Loading is permissive: an unreadable or invalid `.okqignore` is reported
    /// on stderr and skipped, never fatal (mirrors okf's permissive load).
    pub fn load(root: &Path, disabled: bool) -> IgnoreSet {
        if disabled {
            return IgnoreSet {
                matchers: Vec::new(),
                files: Vec::new(),
                enabled: false,
            };
        }

        let mut files = Vec::new();
        collect_ignore_files(root, &mut files);
        files.sort();

        let mut matchers = Vec::new();
        for file in &files {
            let dir = file.parent().unwrap_or(root).to_path_buf();
            let mut builder = GitignoreBuilder::new(&dir);
            if let Some(err) = builder.add(file) {
                eprintln!("okq: warning: could not read {}: {err}", file.display());
                continue;
            }
            match builder.build() {
                Ok(gi) => matchers.push((dir, gi)),
                Err(err) => eprintln!("okq: warning: invalid {}: {err}", file.display()),
            }
        }
        // Deepest directory first, so a nested `.okqignore` takes precedence.
        matchers.sort_by_key(|(dir, _)| std::cmp::Reverse(dir.components().count()));

        IgnoreSet {
            matchers,
            files,
            enabled: true,
        }
    }

    /// Whether `path` (an absolute file path, as okf produces) is excluded.
    ///
    /// Checks matchers deepest-first and returns on the first decisive verdict,
    /// so nested rules override shallower ones. Directory patterns (`tests/`)
    /// match files beneath them because the `ignore` crate also tests each
    /// parent directory of the path.
    pub fn is_ignored(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }
        for (dir, gi) in &self.matchers {
            if !path.starts_with(dir) {
                continue;
            }
            match gi.matched_path_or_any_parents(path, false) {
                Match::Ignore(_) => return true,
                Match::Whitelist(_) => return false,
                Match::None => {}
            }
        }
        false
    }

    /// The `.okqignore` files found, in path order (for cache staleness).
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// `true` if ignore processing is on and at least one rule file was loaded.
    pub fn is_active(&self) -> bool {
        self.enabled && !self.matchers.is_empty()
    }

    /// `true` unless `--no-ignore` disabled processing. Distinguishes the two
    /// effective modes so the search index can cache them separately.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Recursively collects `.okqignore` files under `dir`, skipping `.git`.
fn collect_ignore_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            collect_ignore_files(&path, out);
        } else if path.file_name().is_some_and(|n| n == IGNORE_FILENAME) {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn disabled_excludes_nothing() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join(".okqignore"), "tests/\n");
        let set = IgnoreSet::load(dir.path(), true);
        assert!(!set.is_ignored(&dir.path().join("tests/a.md")));
        assert!(!set.is_active());
        assert!(set.files().is_empty());
    }

    #[test]
    fn directory_pattern_excludes_descendants() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join(".okqignore"), "tests/\n");
        let set = IgnoreSet::load(dir.path(), false);
        assert!(set.is_ignored(&dir.path().join("tests/a.md")));
        assert!(set.is_ignored(&dir.path().join("tests/deep/b.md")));
        assert!(!set.is_ignored(&dir.path().join("features/a.md")));
    }

    #[test]
    fn negation_reincludes() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join(".okqignore"), "drafts/\n!drafts/keep.md\n");
        let set = IgnoreSet::load(dir.path(), false);
        assert!(set.is_ignored(&dir.path().join("drafts/scratch.md")));
        assert!(!set.is_ignored(&dir.path().join("drafts/keep.md")));
    }

    #[test]
    fn nested_file_overrides_shallower() {
        let dir = TempDir::new().unwrap();
        // Root ignores everything under notes/; a nested file re-includes keep.md.
        write(&dir.path().join(".okqignore"), "notes/\n");
        write(&dir.path().join("notes/.okqignore"), "!keep.md\n");
        let set = IgnoreSet::load(dir.path(), false);
        assert!(set.is_ignored(&dir.path().join("notes/scratch.md")));
        assert!(!set.is_ignored(&dir.path().join("notes/keep.md")));
        assert_eq!(set.files().len(), 2);
    }

    #[test]
    fn no_ignore_files_means_inactive() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join("a.md"), "x");
        let set = IgnoreSet::load(dir.path(), false);
        assert!(!set.is_active());
        assert!(!set.is_ignored(&dir.path().join("a.md")));
    }
}
