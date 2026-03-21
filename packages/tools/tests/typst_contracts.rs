use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use rust_book_tools::typst::adapters::rust_book::RustBookAdapter;
use rust_book_tools::typst::export::TypstAdapter;
use rust_book_tools::typst::paths::{
    ExportContext, GeneratedFiles, resolve_output_dir,
};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct TestRepo {
    root: PathBuf,
}

struct ScopedDir {
    root: PathBuf,
}

impl TestRepo {
    fn new(name: &str) -> Self {
        Self {
            root: new_scoped_path(&std::env::temp_dir(), name),
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn touch(&self, relative: &str) {
        self.write(relative, "");
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl ScopedDir {
    fn new_in(base: &Path, name: &str) -> Self {
        Self {
            root: new_scoped_path(base, name),
        }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for ScopedDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn resolve_output_dir_requires_output_to_stay_under_repo_root() {
    let temp = TestRepo::new("resolve-output");
    let repo_root = temp.path("repo");
    fs::create_dir_all(&repo_root).unwrap();

    let relative =
        resolve_output_dir(&repo_root, Some(PathBuf::from("dist/typst")))
            .unwrap();
    assert_eq!(relative, repo_root.join("dist/typst"));

    let absolute_inside = repo_root.join("custom/output");
    let resolved_inside =
        resolve_output_dir(&repo_root, Some(absolute_inside.clone())).unwrap();
    assert_eq!(resolved_inside, absolute_inside);

    let absolute_outside = temp.path("outside");
    let err =
        resolve_output_dir(&repo_root, Some(absolute_outside)).unwrap_err();
    assert!(err.to_string().contains("must be inside repo root"));
}

#[test]
fn rust_book_adapter_prepares_markdown_from_supported_source_features() {
    let (repo, ctx) = rust_book_fixture();
    let adapter = RustBookAdapter;

    let prepared = adapter.prepare_markdown(&ctx).unwrap();

    assert_eq!(prepared.frontmatter, "Front matter paragraph.");
    assert!(prepared.body.contains("line two\nline three"));
    assert!(prepared.body.contains("fn anchored() {}"));
    assert!(prepared.body.contains("`Ctrl`"));
    assert!(prepared.body.contains("number=\"1-1\""));
    assert!(prepared.body.contains("file-name=\"src/main.rs\""));
    assert!(prepared.body.contains("file-name=\"src/lib.rs\""));
    assert!(prepared.body.contains("Listing caption"));
    assert!(prepared.body.contains("![Ferris image]("));
    assert!(prepared.body.contains("visible_line();"));

    for forbidden in [
        "{{#",
        "<Listing",
        "</Listing>",
        "<span class=\"filename\">",
        "<span class=\"caption\">",
        "<kbd>",
        "<a id=",
        "<!--",
        "# hidden line",
    ] {
        assert!(
            !prepared.body.contains(forbidden),
            "normalized markdown unexpectedly still contains `{forbidden}`:\n{}",
            prepared.body
        );
    }

    drop(repo);
}

#[test]
fn rust_book_adapter_uses_relative_theme_paths_in_generated_typst() {
    let (_repo, ctx) = rust_book_fixture();
    let adapter = RustBookAdapter;
    let files = GeneratedFiles::from_layout(&ctx, &adapter.output_layout());

    fs::create_dir_all(files.body_typ.parent().unwrap()).unwrap();
    fs::write(&files.body_typ, "= body").unwrap();

    adapter.postprocess_body_typ(&ctx, &files.body_typ).unwrap();
    let body_typ = fs::read_to_string(&files.body_typ).unwrap();
    assert!(body_typ.starts_with(
        "#import \"../tools/typst/theme.typ\": book_listing\n\n= body"
    ));

    let wrapper = adapter.build_wrapper(&ctx, &files).unwrap();
    assert!(
        wrapper
            .contains("#import \"../tools/typst/theme.typ\": rust_book_theme")
    );
    assert!(
        wrapper.contains("#import \"../tools/typst/layout.typ\": rust_book_layout")
    );
    assert!(wrapper.contains("#show: doc => rust_book_theme(doc, rust_book_layout)"));
    assert!(wrapper.contains("#include \"frontmatter.typ\""));
    assert!(
        wrapper.contains("#include \"the-rust-programming-language.body.typ\"")
    );
}

#[test]
fn rust_book_adapter_matches_current_repo_conventions() {
    let repo_root = current_repo_root();
    let source_corpus = read_markdown_tree(&repo_root.join("src"));
    let directive_kinds = collect_mdbook_directive_kinds(&source_corpus);
    let adapter = RustBookAdapter;
    let output_dir = ScopedDir::new_in(&repo_root.join("target"), "current-repo");
    let ctx = ExportContext::new(repo_root.clone(), output_dir.path().into());

    for kind in directive_kinds {
        assert!(
            matches!(kind.as_str(), "include" | "rustdoc_include"),
            "current book sources use unsupported mdBook directive `{kind}`"
        );
    }

    let prepared = adapter.prepare_markdown(&ctx).unwrap();
    assert!(!prepared.frontmatter.trim().is_empty());
    assert!(!prepared.body.trim().is_empty());
    assert!(
        !prepared.body.contains("{{#"),
        "prepared markdown unexpectedly still contains unresolved mdBook directives"
    );

    if source_corpus.contains("<Listing") {
        assert!(
            prepared.body.contains("::: {.listing"),
            "current book sources contain listing tags but the prepared markdown contains no listing divs"
        );
    }
}

fn rust_book_fixture() -> (TestRepo, ExportContext) {
    let repo = TestRepo::new("rust-book-fixture");
    repo.write(
        "src/SUMMARY.md",
        "# Summary\n\n- [Title Page](title-page.md)\n- [Chapter](chapter.md)\n",
    );
    repo.write(
        "src/title-page.md",
        "# Mini Book\n\nExample Author\n\nFront matter paragraph.\n",
    );
    repo.write(
        "src/chapter.md",
        r#"# Chapter

Intro <!-- inline comment --> with <kbd>Ctrl</kbd>.

<a id="jump-here"></a>

{{#include includes/snippet.rs:2:3}}

{{#rustdoc_include includes/anchored.rs:example}}

<Listing number="1-1" file-name="src/main.rs" caption="Listing caption">
```rust
fn main() {
    println!("hello");
}
```
</Listing>

<img src="images/ferris.png" alt="Ferris">
<span class="caption">Ferris image</span>

Filename: src/lib.rs

```rust
# hidden line
visible_line();
```
"#,
    );
    repo.write(
        "src/includes/snippet.rs",
        "line one\nline two\nline three\nline four\n",
    );
    repo.write(
        "src/includes/anchored.rs",
        "// ANCHOR: example\nfn anchored() {}\n// ANCHOR_END: example\n",
    );
    repo.touch("src/images/ferris.png");
    repo.touch("tools/typst/book.lua");
    repo.touch("tools/typst/layout.typ");
    repo.touch("tools/typst/theme.typ");

    let output_dir = repo.path("generated");
    let ctx = ExportContext::new(repo.root.clone(), output_dir);
    (repo, ctx)
}

fn new_scoped_path(base: &Path, name: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root =
        base.join(format!("rust-book-tools-{name}-{}-{id}", std::process::id()));

    if root.exists() {
        let _ = fs::remove_dir_all(&root);
    }
    fs::create_dir_all(&root).unwrap();
    root
}

fn current_repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn read_markdown_tree(root: &Path) -> String {
    let mut markdown = String::new();
    read_markdown_tree_into(root, &mut markdown);
    markdown
}

fn read_markdown_tree_into(root: &Path, markdown: &mut String) {
    let mut entries = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            read_markdown_tree_into(&path, markdown);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            markdown.push_str(&fs::read_to_string(path).unwrap());
            markdown.push('\n');
        }
    }
}

fn collect_mdbook_directive_kinds(source: &str) -> Vec<String> {
    let mut kinds = Vec::new();
    let mut remainder = source;

    while let Some(start) = remainder.find("{{#") {
        let after_start = &remainder[start + 3..];
        let end = after_start
            .find(|ch: char| ch == ' ' || ch == '\n' || ch == '\r' || ch == '}')
            .unwrap_or(after_start.len());
        let kind = &after_start[..end];
        if !kind.is_empty() && !kinds.iter().any(|existing| existing == kind) {
            kinds.push(kind.to_string());
        }
        remainder = after_start;
    }

    kinds
}
