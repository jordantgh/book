use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::typst::DynError;

pub fn run_pandoc(
    input: &Path,
    output: &Path,
    format: &str,
    filter: Option<&Path>,
) -> Result<(), DynError> {
    let mut command = Command::new(find_program("pandoc"));
    command
        .arg(input)
        .arg("--quiet")
        .arg("-f")
        .arg(format)
        .arg("-t")
        .arg("typst")
        .arg("--wrap=none");

    if let Some(filter) = filter {
        command.arg("--lua-filter").arg(filter);
    }

    let status = command.arg("-o").arg(output).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("pandoc failed for {}", input.display()).into())
    }
}

pub fn run_typst(
    root: &Path,
    input: &Path,
    output: &Path,
) -> Result<(), DynError> {
    let status = Command::new(find_program("typst"))
        .arg("compile")
        .arg("--root")
        .arg(root)
        .arg(input)
        .arg(output)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("typst failed for {}", input.display()).into())
    }
}

pub fn find_program(name: &str) -> PathBuf {
    if Path::new(name).components().count() > 1 {
        return PathBuf::from(name);
    }

    let mut candidates = vec![name.to_string()];
    if cfg!(windows) && Path::new(name).extension().is_none() {
        candidates = vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            format!("{name}.bat"),
            name.to_string(),
        ];
    }

    if let Some(path_var) = env::var_os("PATH") {
        for directory in env::split_paths(&path_var) {
            for candidate in &candidates {
                let full = directory.join(candidate);
                if full.is_file() {
                    return full;
                }
            }
        }
    }

    PathBuf::from(candidates[0].clone())
}
