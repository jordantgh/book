use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lazy_static::lazy_static;
use pathdiff::diff_paths;
use regex::{Captures, Regex};

const DEFAULT_OUTPUT_DIR: &str = "dist/typst";
const TITLE_PAGE_PATH: &str = "src/title-page.md";
const SUMMARY_PATH: &str = "src/SUMMARY.md";
const FRONTMATTER_MD: &str = "frontmatter.md";
const FRONTMATTER_TYP: &str = "frontmatter.typ";
const BODY_MD: &str = "book.md";
const BODY_TYP: &str = "the-rust-programming-language.body.typ";
const BOOK_TYP: &str = "the-rust-programming-language.typ";
const PDF_PATH: &str = "dist/the-rust-programming-language-typst.pdf";
const PANDOC_FORMAT: &str = "markdown-tex_math_dollars+fenced_divs+bracketed_spans+raw_html+pipe_tables+table_captions+smart";

type DynError = Box<dyn std::error::Error>;

lazy_static! {
    static ref SUMMARY_LINK: Regex =
        Regex::new(r"\(([^)]+\.md)\)").expect("valid SUMMARY regex");
    static ref DIRECTIVE: Regex = Regex::new(
        r"\{\{#(?P<kind>include|rustdoc_include)\s+(?P<spec>[^}]+)\}\}"
    )
    .expect("valid directive regex");
    static ref INCLUDE_RANGE: Regex =
        Regex::new(r"^(?P<path>.*?)(?::(?P<start>\d*))?(?::(?P<end>\d*))?$")
            .expect("valid include range regex");
    static ref ATTR: Regex = Regex::new(r#"([A-Za-z0-9_-]+)="([^"]*)""#)
        .expect("valid attribute regex");
    static ref ANCHOR_START: Regex =
        Regex::new(r#"^\s*//\s*ANCHOR:\s*([A-Za-z0-9_-]+)\s*$"#)
            .expect("valid anchor start regex");
    static ref ANCHOR_END: Regex =
        Regex::new(r#"^\s*//\s*ANCHOR_END:\s*([A-Za-z0-9_-]+)\s*$"#)
            .expect("valid anchor end regex");
    static ref INLINE_COMMENT: Regex =
        Regex::new(r"<!--.*?-->").expect("valid inline comment regex");
    static ref INLINE_KBD: Regex =
        Regex::new(r"<kbd>(.*?)</kbd>").expect("valid kbd regex");
    static ref INLINE_IMG: Regex =
        Regex::new(r#"<img\s+[^>]*?src="([^"]+)"[^>]*?alt="([^"]*)"[^>]*/?>"#)
            .expect("valid inline image regex");
    static ref INLINE_IMG_ALT_FIRST: Regex =
        Regex::new(r#"<img\s+[^>]*?alt="([^"]*)"[^>]*?src="([^"]+)"[^>]*/?>"#)
            .expect("valid inline image alt-first regex");
}

#[derive(Debug, Clone)]
struct ListingMeta {
    number: Option<String>,
    file_name: Option<String>,
    caption: Option<String>,
}

#[derive(Debug, Clone)]
struct ImageMeta {
    src: String,
    alt: String,
}

fn main() -> Result<(), DynError> {
    let repo_root = env::current_dir()?;
    let output_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                repo_root.join(path)
            }
        })
        .unwrap_or_else(|| repo_root.join(DEFAULT_OUTPUT_DIR));
    fs::create_dir_all(&output_dir)?;

    let summary = read_text(Path::new(SUMMARY_PATH))?;
    let source_paths = summary_paths(&summary);
    let title_page_path = repo_root.join(TITLE_PAGE_PATH);
    let title_page = read_text(&title_page_path)?;
    let frontmatter_source = strip_title_page_header(&title_page);
    let frontmatter_md =
        normalize_markdown(&frontmatter_source, &title_page_path, &output_dir)?;

    let body_paths = source_paths
        .into_iter()
        .filter(|path| path != Path::new("title-page.md"))
        .collect::<Vec<_>>();
    let mut body_md = String::new();
    for relative in body_paths {
        let source_path = repo_root.join("src").join(relative);
        if !body_md.is_empty() {
            body_md.push_str("\n\n");
        }
        body_md.push_str(&normalize_source_file(&source_path, &output_dir)?);
    }

    let frontmatter_md_path = output_dir.join(FRONTMATTER_MD);
    let body_md_path = output_dir.join(BODY_MD);
    let frontmatter_typ_path = output_dir.join(FRONTMATTER_TYP);
    let body_typ_path = output_dir.join(BODY_TYP);
    let book_typ_path = output_dir.join(BOOK_TYP);

    fs::write(&frontmatter_md_path, &frontmatter_md)?;
    fs::write(&body_md_path, &body_md)?;

    run_pandoc(&frontmatter_md_path, &frontmatter_typ_path)?;
    run_pandoc(&body_md_path, &body_typ_path)?;
    prepend_body_import(&repo_root, &body_typ_path)?;

    fs::write(
        &book_typ_path,
        build_wrapper(
            &repo_root,
            &output_dir,
            &frontmatter_typ_path,
            &body_typ_path,
        ),
    )?;

    run_typst(&book_typ_path)?;

    println!("Wrote {}", book_typ_path.display());
    println!("Wrote {}", Path::new(PDF_PATH).display());
    Ok(())
}

fn summary_paths(summary: &str) -> Vec<PathBuf> {
    SUMMARY_LINK
        .captures_iter(summary)
        .filter_map(|caps| caps.get(1))
        .map(|m| PathBuf::from(m.as_str()))
        .collect()
}

fn strip_title_page_header(source: &str) -> String {
    let mut paragraphs = source.split("\n\n");
    let _ = paragraphs.next();
    let _ = paragraphs.next();
    paragraphs.collect::<Vec<_>>().join("\n\n")
}

fn normalize_source_file(
    source_path: &Path,
    output_dir: &Path,
) -> Result<String, DynError> {
    let source = read_text(source_path)?;
    normalize_markdown(&source, source_path, output_dir)
}

fn normalize_markdown(
    source: &str,
    source_path: &Path,
    output_dir: &Path,
) -> Result<String, DynError> {
    let expanded = expand_directives(source, source_path)?;
    let rewritten = rewrite_markdown(&expanded, source_path, output_dir)?;
    let wrapped = wrap_filename_listings(&rewritten);
    let without_hidden = remove_hidden_lines(&wrapped);
    Ok(without_hidden.trim().to_string())
}

fn expand_directives(
    source: &str,
    source_path: &Path,
) -> Result<String, DynError> {
    let mut output = String::new();
    let mut last = 0;

    for caps in DIRECTIVE.captures_iter(source) {
        let whole = caps.get(0).expect("directive match");
        let kind = caps.name("kind").expect("directive kind").as_str();
        let spec = caps.name("spec").expect("directive spec").as_str().trim();
        output.push_str(&source[last..whole.start()]);
        output.push_str(&expand_directive(kind, spec, source_path)?);
        last = whole.end();
    }

    output.push_str(&source[last..]);
    Ok(output)
}

fn expand_directive(
    kind: &str,
    spec: &str,
    source_path: &Path,
) -> Result<String, DynError> {
    match kind {
        "include" => {
            if let Some((relative, anchor)) = parse_anchor_spec(spec) {
                let path = resolve_relative_path(source_path, &relative);
                let contents = read_text(&path)?;
                extract_rustdoc_anchor(&contents, &anchor)
            } else {
                let (relative, start, end) = parse_include_spec(spec)?;
                let path = resolve_relative_path(source_path, &relative);
                let contents = read_text(&path)?;
                Ok(extract_line_range(&contents, start, end))
            }
        }
        "rustdoc_include" => {
            let (relative, anchor) = parse_rustdoc_spec(spec);
            let path = resolve_relative_path(source_path, &relative);
            let contents = read_text(&path)?;
            match anchor {
                Some(anchor) => extract_rustdoc_anchor(&contents, &anchor),
                None => Ok(contents),
            }
        }
        _ => unreachable!("directive regex only matches supported directives"),
    }
}

fn parse_include_spec(
    spec: &str,
) -> Result<(PathBuf, Option<usize>, Option<usize>), DynError> {
    let caps = INCLUDE_RANGE
        .captures(spec)
        .ok_or_else(|| format!("invalid include directive: {spec}"))?;
    let path = caps
        .name("path")
        .map(|m| PathBuf::from(m.as_str()))
        .ok_or_else(|| format!("missing include path: {spec}"))?;
    let start = parse_optional_usize(caps.name("start").map(|m| m.as_str()))?;
    let end = parse_optional_usize(caps.name("end").map(|m| m.as_str()))?;
    Ok((path, start, end))
}

fn parse_rustdoc_spec(spec: &str) -> (PathBuf, Option<String>) {
    match parse_anchor_spec(spec) {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (PathBuf::from(spec), None),
    }
}

fn parse_anchor_spec(spec: &str) -> Option<(PathBuf, String)> {
    match spec.rsplit_once(':') {
        Some((path, anchor))
            if !anchor.is_empty()
                && !anchor.contains(['/', '\\'])
                && !anchor.chars().all(|ch| ch.is_ascii_digit()) =>
        {
            Some((PathBuf::from(path), anchor.to_string()))
        }
        _ => None,
    }
}

fn parse_optional_usize(
    value: Option<&str>,
) -> Result<Option<usize>, DynError> {
    match value {
        Some("") | None => Ok(None),
        Some(raw) => Ok(Some(raw.parse::<usize>()?)),
    }
}

fn resolve_relative_path(source_path: &Path, relative: &Path) -> PathBuf {
    source_path
        .parent()
        .expect("source file has parent")
        .join(relative)
}

fn extract_line_range(
    contents: &str,
    start: Option<usize>,
    end: Option<usize>,
) -> String {
    let lines = contents.lines().collect::<Vec<_>>();
    let start_index = start.unwrap_or(1).saturating_sub(1);
    let end_index = end.unwrap_or(lines.len()).min(lines.len());

    if start_index >= end_index || start_index >= lines.len() {
        return String::new();
    }

    lines[start_index..end_index].join("\n")
}

fn extract_rustdoc_anchor(
    contents: &str,
    anchor: &str,
) -> Result<String, DynError> {
    let mut capturing = false;
    let mut matched = false;
    let mut output = Vec::new();

    for line in contents.lines() {
        if let Some(caps) = ANCHOR_START.captures(line) {
            if caps.get(1).expect("anchor name").as_str() == anchor {
                matched = true;
                capturing = !capturing;
                continue;
            }
        }

        if let Some(caps) = ANCHOR_END.captures(line) {
            if caps.get(1).expect("anchor name").as_str() == anchor {
                matched = true;
                capturing = false;
                continue;
            }
        }

        if capturing {
            output.push(line);
        }
    }

    if !matched {
        return Err(format!("missing rustdoc anchor `{anchor}`").into());
    }

    Ok(output.join("\n"))
}

fn rewrite_markdown(
    source: &str,
    source_path: &Path,
    output_dir: &Path,
) -> Result<String, DynError> {
    let mut output = Vec::new();
    let lines = source.lines().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut in_code_block = false;
    let mut in_html_comment = false;
    let mut pending_listing: Option<ListingMeta> = None;
    let mut pending_image: Option<ImageMeta> = None;

    while index < lines.len() {
        let raw_line = lines[index];
        let trimmed = raw_line.trim();

        if let Some(image) = pending_image.clone() {
            if trimmed.is_empty() {
                index += 1;
                continue;
            }

            if trimmed.starts_with(r#"<span class="caption">"#) {
                let (caption, consumed) =
                    collect_span(&lines, index, "caption");
                output.push(markdown_image(
                    &image.src,
                    &caption,
                    source_path,
                    output_dir,
                )?);
                output.push(String::new());
                pending_image = None;
                index += consumed;
                continue;
            }

            output.push(markdown_image(
                &image.src,
                &image.alt,
                source_path,
                output_dir,
            )?);
            output.push(String::new());
            pending_image = None;
            continue;
        }

        if in_html_comment {
            if trimmed.contains("-->") {
                in_html_comment = false;
            }
            index += 1;
            continue;
        }

        if is_fence(raw_line) {
            output.push(normalize_fence(raw_line));
            in_code_block = !in_code_block;
            index += 1;
            continue;
        }

        if in_code_block {
            output.push(raw_line.to_string());
            index += 1;
            continue;
        }

        if trimmed.starts_with("<!--") && !trimmed.contains("-->") {
            in_html_comment = true;
            index += 1;
            continue;
        }

        if trimmed.starts_with(r#"<a id=""#) {
            index += 1;
            continue;
        }

        if trimmed.starts_with("<Listing") {
            let (tag, consumed) = collect_tag(&lines, index, ">");
            let listing = parse_listing_tag(&tag);
            output.push(render_listing_open(&listing));
            output.push(String::new());
            pending_listing = Some(listing);
            index += consumed;
            continue;
        }

        if trimmed.starts_with("</Listing>") {
            if let Some(listing) = pending_listing.take() {
                if let Some(caption) = listing.caption {
                    output.push(String::new());
                    output.push(caption);
                }
                output.push(":::".to_string());
            }
            index += 1;
            continue;
        }

        if trimmed.starts_with(r#"<span class="filename">"#) {
            let (contents, consumed) = collect_span(&lines, index, "filename");
            output.push(contents);
            index += consumed;
            continue;
        }

        if trimmed.starts_with(r#"<span class="caption">"#) {
            let (contents, consumed) = collect_span(&lines, index, "caption");
            output.push(contents);
            index += consumed;
            continue;
        }

        if trimmed.starts_with("<img ") {
            let (tag, consumed) = collect_tag(&lines, index, ">");
            pending_image = Some(parse_image_tag(&tag)?);
            index += consumed;
            continue;
        }

        let stripped = INLINE_COMMENT.replace_all(raw_line, "").to_string();
        let stripped = INLINE_KBD
            .replace_all(&stripped, |caps: &Captures<'_>| {
                format!("`{}`", &caps[1])
            })
            .to_string();
        let stripped =
            replace_inline_images(&stripped, source_path, output_dir)?;

        output.push(stripped);
        index += 1;
    }

    if let Some(image) = pending_image {
        output.push(markdown_image(
            &image.src,
            &image.alt,
            source_path,
            output_dir,
        )?);
    }

    Ok(output.join("\n"))
}

fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

fn normalize_fence(line: &str) -> String {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("```") {
        return line.to_string();
    }

    let indent = &line[..line.len() - trimmed.len()];
    let info = trimmed.trim_start_matches("```").trim();
    if info.is_empty() {
        return format!("{indent}```");
    }

    let language = info
        .split([',', ' ', '{'])
        .find(|segment| !segment.is_empty())
        .unwrap_or("");

    if language.is_empty() {
        format!("{indent}```")
    } else {
        format!("{indent}```{language}")
    }
}

fn collect_tag(
    lines: &[&str],
    start: usize,
    terminator: &str,
) -> (String, usize) {
    let mut collected = vec![lines[start].trim().to_string()];
    let mut index = start;

    while !collected.last().expect("has line").contains(terminator)
        && index + 1 < lines.len()
    {
        index += 1;
        collected.push(lines[index].trim().to_string());
    }

    (collected.join(" "), index - start + 1)
}

fn collect_span(
    lines: &[&str],
    start: usize,
    class_name: &str,
) -> (String, usize) {
    let (tag, consumed) = collect_tag(lines, start, "</span>");
    let opening = format!(r#"<span class="{class_name}">"#);
    let stripped = tag
        .replace(&opening, "")
        .replace("</span>", "")
        .trim()
        .to_string();
    (stripped, consumed)
}

fn parse_listing_tag(tag: &str) -> ListingMeta {
    let mut number = None;
    let mut file_name = None;
    let mut caption = None;

    for caps in ATTR.captures_iter(tag) {
        let key = caps.get(1).expect("attr key").as_str();
        let value = caps.get(2).expect("attr value").as_str().to_string();
        match key {
            "number" => number = Some(value),
            "file-name" => file_name = Some(value),
            "caption" => caption = Some(value),
            _ => {}
        }
    }

    ListingMeta {
        number,
        file_name,
        caption,
    }
}

fn render_listing_open(listing: &ListingMeta) -> String {
    let mut attrs = vec![".listing".to_string()];
    if let Some(number) = &listing.number {
        attrs.push(format!("#listing-{number}"));
        attrs.push(format!(r#"number="{number}""#));
    }
    if let Some(file_name) = &listing.file_name {
        attrs.push(format!(r#"file-name="{}""#, escape_attr(file_name)));
    }
    format!("::: {{{}}}", attrs.join(" "))
}

fn parse_image_tag(tag: &str) -> Result<ImageMeta, DynError> {
    let src = parse_attr(tag, "src")
        .ok_or_else(|| format!("image tag missing src: {tag}"))?;
    let alt = parse_attr(tag, "alt").unwrap_or_default();
    Ok(ImageMeta { src, alt })
}

fn parse_attr(tag: &str, name: &str) -> Option<String> {
    ATTR.captures_iter(tag).find_map(|caps| {
        let key = caps.get(1).expect("attr key").as_str();
        if key == name {
            Some(caps.get(2).expect("attr value").as_str().to_string())
        } else {
            None
        }
    })
}

fn replace_inline_images(
    line: &str,
    source_path: &Path,
    output_dir: &Path,
) -> Result<String, DynError> {
    let after_src_first = replace_inline_images_with_regex(
        line,
        &INLINE_IMG,
        false,
        source_path,
        output_dir,
    )?;
    replace_inline_images_with_regex(
        &after_src_first,
        &INLINE_IMG_ALT_FIRST,
        true,
        source_path,
        output_dir,
    )
}

fn replace_inline_images_with_regex(
    line: &str,
    regex: &Regex,
    alt_first: bool,
    source_path: &Path,
    output_dir: &Path,
) -> Result<String, DynError> {
    let mut output = String::new();
    let mut last = 0usize;

    for caps in regex.captures_iter(line) {
        let whole = caps.get(0).expect("image match");
        let (alt, src) = if alt_first {
            (
                caps.get(1).expect("alt").as_str(),
                caps.get(2).expect("src").as_str(),
            )
        } else {
            (
                caps.get(2).expect("alt").as_str(),
                caps.get(1).expect("src").as_str(),
            )
        };

        output.push_str(&line[last..whole.start()]);
        output.push_str(&markdown_image(src, alt, source_path, output_dir)?);
        last = whole.end();
    }

    output.push_str(&line[last..]);
    Ok(output)
}

fn markdown_image(
    source: &str,
    caption: &str,
    source_path: &Path,
    output_dir: &Path,
) -> Result<String, DynError> {
    let image_path = resolve_relative_path(source_path, Path::new(source));
    let relative = diff_paths(&image_path, output_dir).ok_or_else(|| {
        format!("could not compute image path for {}", image_path.display())
    })?;
    let escaped_path = relative.to_string_lossy().replace('\\', "/");
    let escaped_caption = caption
        .replace('[', r"\[")
        .replace(']', r"\]")
        .replace('\n', " ");
    Ok(format!("![{escaped_caption}]({escaped_path})"))
}

fn escape_attr(value: &str) -> String {
    value.replace('"', "&quot;")
}

fn wrap_filename_listings(source: &str) -> String {
    let mut output = Vec::new();
    let lines = source.lines().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut in_code_block = false;

    while index < lines.len() {
        let current = lines[index];

        if !in_code_block && current.trim_start().starts_with("Filename: ") {
            let file_name =
                current.trim_start().trim_start_matches("Filename: ").trim();
            let mut next = index + 1;
            while next < lines.len() && lines[next].trim().is_empty() {
                next += 1;
            }

            if next < lines.len() && is_fence(lines[next]) {
                output.push(format!(
                    r#"::: {{.listing file-name="{}"}}"#,
                    escape_attr(file_name)
                ));
                output.push(String::new());
                index = next;

                while index < lines.len() {
                    let line = lines[index];
                    output.push(line.to_string());
                    if is_fence(line) {
                        in_code_block = !in_code_block;
                        if !in_code_block {
                            index += 1;
                            break;
                        }
                    }
                    index += 1;
                }

                output.push(String::new());
                output.push(":::".to_string());
                continue;
            }
        }

        if is_fence(current) {
            in_code_block = !in_code_block;
        }
        output.push(current.to_string());
        index += 1;
    }

    output.join("\n")
}

fn remove_hidden_lines(source: &str) -> String {
    let mut output = Vec::new();
    let mut in_code_block = false;

    for line in source.lines() {
        if is_fence(line) {
            in_code_block = !in_code_block;
        }

        if !in_code_block || (!line.starts_with("# ") && line != "#") {
            output.push(line);
        }
    }

    output.join("\n")
}

fn build_wrapper(
    repo_root: &Path,
    output_dir: &Path,
    frontmatter_typ_path: &Path,
    body_typ_path: &Path,
) -> String {
    let theme = diff_paths(repo_root.join("tools/typst/theme.typ"), output_dir)
        .expect("theme path can be relativized")
        .to_string_lossy()
        .replace('\\', "/");
    let frontmatter = frontmatter_typ_path
        .file_name()
        .expect("frontmatter file name")
        .to_string_lossy();
    let body = body_typ_path
        .file_name()
        .expect("body file name")
        .to_string_lossy();

    format!(
        r##"#import "{theme}": rust_book_theme

#show: rust_book_theme

#v(18%)
#align(center)[
  #text(size: 28pt, weight: "bold")[The Rust Programming Language]
  #v(1.4em)
  #text(size: 12pt, style: "italic")[
    Steve Klabnik, Carol Nichols, and Chris Krycho
  ]
  #v(0.7em)
  #text(size: 10pt, fill: rgb("#667085"))[
    with contributions from the Rust Community
  ]
]

#pagebreak()
#include "{frontmatter}"

#pagebreak()
#outline(title: [Contents], depth: 2)

#pagebreak()
#include "{body}"
"##
    )
}

fn run_pandoc(input: &Path, output: &Path) -> Result<(), DynError> {
    let status = Command::new(find_program("pandoc"))
        .arg(input)
        .arg("--quiet")
        .arg("-f")
        .arg(PANDOC_FORMAT)
        .arg("-t")
        .arg("typst")
        .arg("--wrap=none")
        .arg("--lua-filter")
        .arg("tools/typst/book.lua")
        .arg("-o")
        .arg(output)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("pandoc failed for {}", input.display()).into())
    }
}

fn run_typst(book_typ_path: &Path) -> Result<(), DynError> {
    let status = Command::new(find_program("typst"))
        .arg("compile")
        .arg("--root")
        .arg(".")
        .arg(book_typ_path)
        .arg(PDF_PATH)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("typst failed for {}", book_typ_path.display()).into())
    }
}

fn read_text(path: &Path) -> Result<String, DynError> {
    fs::read_to_string(path).map_err(|error| {
        format!("failed to read {}: {error}", path.display()).into()
    })
}

fn prepend_body_import(
    repo_root: &Path,
    body_typ_path: &Path,
) -> Result<(), DynError> {
    let body_dir = body_typ_path
        .parent()
        .expect("generated body file has parent");
    let theme = diff_paths(repo_root.join("tools/typst/theme.typ"), body_dir)
        .expect("theme path can be relativized")
        .to_string_lossy()
        .replace('\\', "/");
    let body = read_text(body_typ_path)?;
    fs::write(
        body_typ_path,
        format!("#import \"{theme}\": book_listing\n\n{body}"),
    )?;
    Ok(())
}

fn find_program(name: &str) -> PathBuf {
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
