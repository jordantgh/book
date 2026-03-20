use std::fs;
use std::path::{Path, PathBuf};

use pathdiff::diff_paths;

use crate::typst::{DEFAULT_OUTPUT_DIR, DynError};

#[derive(Debug, Clone)]
pub struct ExportContext {
    pub repo_root: PathBuf,
    pub output_dir: PathBuf,
}

impl ExportContext {
    pub fn new(repo_root: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            repo_root,
            output_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputLayout {
    pub frontmatter_md: PathBuf,
    pub body_md: PathBuf,
    pub frontmatter_typ: PathBuf,
    pub body_typ: PathBuf,
    pub wrapper_typ: PathBuf,
    pub pdf: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GeneratedFiles {
    pub frontmatter_md: PathBuf,
    pub body_md: PathBuf,
    pub frontmatter_typ: PathBuf,
    pub body_typ: PathBuf,
    pub wrapper_typ: PathBuf,
    pub pdf: PathBuf,
}

impl GeneratedFiles {
    pub fn from_layout(ctx: &ExportContext, layout: &OutputLayout) -> Self {
        Self {
            frontmatter_md: resolve_path(
                &ctx.output_dir,
                &layout.frontmatter_md,
            ),
            body_md: resolve_path(&ctx.output_dir, &layout.body_md),
            frontmatter_typ: resolve_path(
                &ctx.output_dir,
                &layout.frontmatter_typ,
            ),
            body_typ: resolve_path(&ctx.output_dir, &layout.body_typ),
            wrapper_typ: resolve_path(&ctx.output_dir, &layout.wrapper_typ),
            pdf: resolve_path(&ctx.repo_root, &layout.pdf),
        }
    }
}

pub fn resolve_output_dir(
    repo_root: &Path,
    output_arg: Option<PathBuf>,
) -> PathBuf {
    match output_arg {
        Some(path) if path.is_absolute() => path,
        Some(path) => repo_root.join(path),
        None => repo_root.join(DEFAULT_OUTPUT_DIR),
    }
}

pub fn read_text(path: &Path) -> Result<String, DynError> {
    fs::read_to_string(path).map_err(|error| {
        format!("failed to read {}: {error}", path.display()).into()
    })
}

pub fn relative_path(
    from_dir: &Path,
    target: &Path,
) -> Result<String, DynError> {
    let relative = diff_paths(target, from_dir).ok_or_else(|| {
        format!(
            "could not compute relative path from {} to {}",
            from_dir.display(),
            target.display()
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn resolve_path(base: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base.join(candidate)
    }
}
