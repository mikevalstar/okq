//! `okq skills install` / `okq skills list` — put the okq-* agent skills on disk.
//!
//! See `docs/features/skills-install.md`. Skills install into a canonical
//! `.agents/skills/<name>/` and are symlinked into the agent's own directory
//! (`.claude/skills/`), the same shape skills.sh uses. The default source is the
//! copy **embedded in this binary**; `--from-repo` is the one okq code path that
//! touches the network (ADR-0007).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

use crate::error::AppError;

/// The GitHub repo skills are fetched from with `--from-repo`.
const REPO: &str = "mikevalstar/okq";
/// The branch `--from-repo` tracks.
const BRANCH: &str = "main";

/// The skills baked into this binary, in install order. The `include_str!`
/// paths are resolved at build time relative to this file, so `skills/` must
/// ship in the published crate (ADR-0007).
const EMBEDDED: &[(&str, &str)] = &[
    (
        "okq-explore",
        include_str!("../../skills/okq-explore/SKILL.md"),
    ),
    (
        "okq-write-okf",
        include_str!("../../skills/okq-write-okf/SKILL.md"),
    ),
    (
        "okq-maintain",
        include_str!("../../skills/okq-maintain/SKILL.md"),
    ),
    (
        "okq-reference",
        include_str!("../../skills/okq-reference/SKILL.md"),
    ),
];

/// One skill the binary carries.
pub struct EmbeddedSkill {
    /// Skill (directory) name, e.g. `okq-explore`.
    pub name: String,
    /// One-line `description` from the SKILL.md frontmatter.
    pub description: String,
}

/// The embedded skills, with their descriptions parsed from frontmatter.
pub fn list() -> Vec<EmbeddedSkill> {
    EMBEDDED
        .iter()
        .map(|(name, content)| EmbeddedSkill {
            name: (*name).to_string(),
            description: parse_description(content),
        })
        .collect()
}

/// The outcome of an install, for human/JSON rendering.
pub struct InstallReport {
    /// `"project"` or `"global"`.
    pub scope: &'static str,
    /// `"embedded"` or `"repo"`.
    pub source: &'static str,
    /// Canonical skills directory written to (`.agents/skills`).
    pub base_dir: PathBuf,
    /// Directory the skills are linked into (`.claude/skills`).
    pub link_dir: PathBuf,
    /// Per-skill results.
    pub skills: Vec<InstalledSkill>,
}

/// What happened to one skill during install.
pub struct InstalledSkill {
    /// Skill name.
    pub name: String,
    /// `"created"` or `"updated"` (the `.agents` copy).
    pub verb: &'static str,
    /// `true` if a symlink was made, `false` if copied (no-symlink platform) or
    /// skipped because a real directory already sat at the target.
    pub linked: bool,
    /// A note when linking was skipped, else empty.
    pub note: String,
}

/// Install (or update) the okq-* skills. `global` targets the home directories;
/// `from_repo` fetches the latest from GitHub instead of using the embedded copy.
pub fn install(global: bool, from_repo: bool) -> Result<InstallReport, AppError> {
    let (base_dir, link_dir) = roots(global)?;
    let skills = if from_repo {
        fetch_from_repo()?
    } else {
        EMBEDDED
            .iter()
            .map(|(n, c)| ((*n).to_string(), (*c).to_string()))
            .collect()
    };

    fs::create_dir_all(&base_dir)
        .map_err(|e| AppError::Io(format!("creating {}: {e}", base_dir.display())))?;
    fs::create_dir_all(&link_dir)
        .map_err(|e| AppError::Io(format!("creating {}: {e}", link_dir.display())))?;

    let mut results = Vec::new();
    for (name, content) in &skills {
        let dir = base_dir.join(name);
        let existed = dir.exists();
        fs::create_dir_all(&dir)
            .map_err(|e| AppError::Io(format!("creating {}: {e}", dir.display())))?;
        let file = dir.join("SKILL.md");
        fs::write(&file, content)
            .map_err(|e| AppError::Io(format!("writing {}: {e}", file.display())))?;

        let (linked, note) = link(&link_dir, name)?;
        results.push(InstalledSkill {
            name: name.clone(),
            verb: if existed { "updated" } else { "created" },
            linked,
            note,
        });
    }

    Ok(InstallReport {
        scope: if global { "global" } else { "project" },
        source: if from_repo { "repo" } else { "embedded" },
        base_dir,
        link_dir,
        skills: results,
    })
}

/// Delegate to skills.sh: `npx skills add mikevalstar/okq`.
pub fn run_skills_sh() -> Result<(), AppError> {
    eprintln!("Running: npx skills add {REPO}");
    let status = Command::new("npx")
        .arg("skills")
        .arg("add")
        .arg(REPO)
        .status()
        .map_err(|e| AppError::Io(format!("could not run `npx` (is Node installed?): {e}")))?;
    if !status.success() {
        return Err(AppError::Io(format!(
            "`npx skills add {REPO}` failed{}",
            status
                .code()
                .map(|c| format!(" (exit {c})"))
                .unwrap_or_default()
        )));
    }
    Ok(())
}

/// The `(.agents/skills, .claude/skills)` roots for the chosen scope.
fn roots(global: bool) -> Result<(PathBuf, PathBuf), AppError> {
    if global {
        let home = dirs::home_dir()
            .ok_or_else(|| AppError::Io("could not determine home directory".into()))?;
        Ok((
            home.join(".agents").join("skills"),
            home.join(".claude").join("skills"),
        ))
    } else {
        Ok((
            PathBuf::from(".agents").join("skills"),
            PathBuf::from(".claude").join("skills"),
        ))
    }
}

/// Symlink (or, on no-symlink platforms, copy) `<link_dir>/<name>` to the
/// canonical `.agents/skills/<name>`. Returns `(linked, note)`.
fn link(link_dir: &Path, name: &str) -> Result<(bool, String), AppError> {
    let link_path = link_dir.join(name);

    // Refuse to clobber a real directory we didn't create; replace our own symlink.
    if let Ok(meta) = fs::symlink_metadata(&link_path) {
        if meta.file_type().is_symlink() {
            let _ = fs::remove_file(&link_path);
        } else {
            return Ok((
                false,
                format!(
                    "{} already exists and is not a symlink; left as-is",
                    link_path.display()
                ),
            ));
        }
    }

    // Relative target resolves at both scopes: .claude/skills/<name> -> ../../.agents/skills/<name>.
    let target = Path::new("..")
        .join("..")
        .join(".agents")
        .join("skills")
        .join(name);

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, &link_path)
            .map_err(|e| AppError::Io(format!("linking {}: {e}", link_path.display())))?;
        Ok((true, String::new()))
    }
    #[cfg(not(unix))]
    {
        let src = link_dir
            .join("..")
            .join("..")
            .join(".agents")
            .join("skills")
            .join(name);
        copy_dir(&src, &link_path)?;
        Ok((
            false,
            "copied (symlinks unavailable on this platform)".into(),
        ))
    }
}

/// Recursively copy a directory (fallback when symlinks are unavailable).
#[cfg(not(unix))]
fn copy_dir(src: &Path, dst: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dst)
        .map_err(|e| AppError::Io(format!("creating {}: {e}", dst.display())))?;
    for entry in
        fs::read_dir(src).map_err(|e| AppError::Io(format!("reading {}: {e}", src.display())))?
    {
        let entry = entry.map_err(|e| AppError::Io(e.to_string()))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)
                .map_err(|e| AppError::Io(format!("copying {}: {e}", from.display())))?;
        }
    }
    Ok(())
}

/// Fetch the skill set from the GitHub repo (`--from-repo`). The only code path
/// in okq that makes a network request (ADR-0007).
fn fetch_from_repo() -> Result<Vec<(String, String)>, AppError> {
    let tree_url = format!("https://api.github.com/repos/{REPO}/git/trees/{BRANCH}?recursive=1");
    let body = http_get(&tree_url)?;
    let tree: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::Io(format!("parsing GitHub tree response: {e}")))?;

    let mut names: Vec<String> = tree
        .get("tree")
        .and_then(|t| t.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e.get("path").and_then(|p| p.as_str()))
                .filter_map(skill_name_from_path)
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names.dedup();

    if names.is_empty() {
        return Err(AppError::Io(format!(
            "no skills found under skills/ in {REPO}@{BRANCH}"
        )));
    }

    let mut out = Vec::new();
    for name in names {
        let raw =
            format!("https://raw.githubusercontent.com/{REPO}/{BRANCH}/skills/{name}/SKILL.md");
        let content = http_get(&raw)?;
        out.push((name, content));
    }
    Ok(out)
}

/// Extract `<name>` from a `skills/<name>/SKILL.md` tree path, else `None`.
fn skill_name_from_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("skills/")?;
    let (name, file) = rest.split_once('/')?;
    (file == "SKILL.md" && !name.is_empty()).then(|| name.to_string())
}

/// A blocking HTTP GET returning the body as text, with okq's User-Agent.
fn http_get(url: &str) -> Result<String, AppError> {
    ureq::get(url)
        .set("User-Agent", concat!("okq/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| AppError::Io(format!("fetching {url}: {e}")))?
        .into_string()
        .map_err(|e| AppError::Io(format!("reading {url}: {e}")))
}

/// Pull the one-line `description:` out of SKILL.md frontmatter (best effort).
fn parse_description(content: &str) -> String {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("description:") {
            return rest.trim().to_string();
        }
        if line.trim() == "---" && !content.starts_with("---") {
            break;
        }
    }
    String::new()
}

/// JSON envelope for an install (`okq.skills/v1`).
pub fn to_json(report: &InstallReport) -> String {
    let skills: Vec<_> = report
        .skills
        .iter()
        .map(|s| {
            json!({
                "name": s.name,
                "verb": s.verb,
                "linked": s.linked,
                "note": s.note,
            })
        })
        .collect();
    let value = json!({
        "schema": "okq.skills/v1",
        "scope": report.scope,
        "source": report.source,
        "agents_dir": report.base_dir.display().to_string(),
        "link_dir": report.link_dir.display().to_string(),
        "count": report.skills.len(),
        "skills": skills,
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_skills_present() {
        let names: Vec<_> = list().into_iter().map(|s| s.name).collect();
        assert!(names.contains(&"okq-explore".to_string()));
        assert!(names.contains(&"okq-reference".to_string()));
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn descriptions_parsed() {
        let skills = list();
        assert!(skills.iter().all(|s| !s.description.is_empty()));
    }

    #[test]
    fn skill_name_extraction() {
        assert_eq!(
            skill_name_from_path("skills/okq-explore/SKILL.md").as_deref(),
            Some("okq-explore")
        );
        assert_eq!(skill_name_from_path("skills/okq-explore/other.md"), None);
        assert_eq!(skill_name_from_path("docs/x.md"), None);
    }
}
