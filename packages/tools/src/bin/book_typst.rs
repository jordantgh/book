use std::env;

use rust_book_tools::typst::DynError;
use rust_book_tools::typst::adapters::rust_book::RustBookAdapter;
use rust_book_tools::typst::export::export_to_typst;
use rust_book_tools::typst::paths::{ExportContext, resolve_output_dir};

fn main() -> Result<(), DynError> {
    let repo_root = env::current_dir()?;
    let output_dir =
        resolve_output_dir(&repo_root, env::args().nth(1).map(Into::into))?;
    let ctx = ExportContext::new(repo_root, output_dir);
    let files = export_to_typst(&ctx, &RustBookAdapter)?;

    println!("Wrote {}", files.wrapper_typ.display());
    println!("Wrote {}", files.pdf.display());
    Ok(())
}
