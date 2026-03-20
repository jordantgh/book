use std::fs;
use std::path::{Path, PathBuf};

use crate::typst::DynError;
use crate::typst::paths::{ExportContext, GeneratedFiles, OutputLayout};
use crate::typst::process::{run_pandoc, run_typst};

pub struct PreparedMarkdown {
    pub frontmatter: String,
    pub body: String,
}

pub trait TypstAdapter {
    fn output_layout(&self) -> OutputLayout;
    fn pandoc_format(&self) -> &str;
    fn prepare_markdown(
        &self,
        ctx: &ExportContext,
    ) -> Result<PreparedMarkdown, DynError>;
    fn pandoc_filter(&self, ctx: &ExportContext) -> Option<PathBuf>;
    fn postprocess_body_typ(
        &self,
        ctx: &ExportContext,
        body_typ_path: &Path,
    ) -> Result<(), DynError>;
    fn build_wrapper(
        &self,
        ctx: &ExportContext,
        files: &GeneratedFiles,
    ) -> Result<String, DynError>;
}

pub fn export_to_typst<A: TypstAdapter>(
    ctx: &ExportContext,
    adapter: &A,
) -> Result<GeneratedFiles, DynError> {
    fs::create_dir_all(&ctx.output_dir)?;

    let files = GeneratedFiles::from_layout(ctx, &adapter.output_layout());
    if let Some(parent) = files.pdf.parent() {
        fs::create_dir_all(parent)?;
    }

    let markdown = adapter.prepare_markdown(ctx)?;
    fs::write(&files.frontmatter_md, markdown.frontmatter)?;
    fs::write(&files.body_md, markdown.body)?;

    let pandoc_filter = adapter.pandoc_filter(ctx);
    run_pandoc(
        &files.frontmatter_md,
        &files.frontmatter_typ,
        adapter.pandoc_format(),
        pandoc_filter.as_deref(),
    )?;
    run_pandoc(
        &files.body_md,
        &files.body_typ,
        adapter.pandoc_format(),
        pandoc_filter.as_deref(),
    )?;

    adapter.postprocess_body_typ(ctx, &files.body_typ)?;

    let wrapper = adapter.build_wrapper(ctx, &files)?;
    fs::write(&files.wrapper_typ, wrapper)?;

    run_typst(&ctx.repo_root, &files.wrapper_typ, &files.pdf)?;
    Ok(files)
}
