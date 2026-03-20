use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const LETTER_WIDTH: f32 = 612.0;
const LETTER_HEIGHT: f32 = 792.0;
const MARGIN_X: f32 = 54.0;
const MARGIN_TOP: f32 = 60.0;
const MARGIN_BOTTOM: f32 = 54.0;
const BODY_FONT_SIZE: f32 = 11.0;
const BODY_LEADING: f32 = 15.0;
const CODE_FONT_SIZE: f32 = 8.8;
const CODE_LEADING: f32 = 11.2;
const TITLE_SIZE: f32 = 28.0;
const CHAPTER_SIZE: f32 = 20.0;
const SECTION_SIZE: f32 = 15.0;
const SUBSECTION_SIZE: f32 = 12.5;
const SMALL_SIZE: f32 = 10.0;

fn main() -> io::Result<()> {
    let output = output_path(std::env::args().skip(1));
    let output = PathBuf::from(output);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let files = ordered_source_files(Path::new("nostarch"))?;
    let mut blocks = Vec::new();
    for path in files {
        blocks.extend(parse_file(&path)?);
    }

    let toc_entries = collect_toc_entries(&blocks);
    let toc_pages = toc_page_count(&toc_entries);
    let layout = layout_document(&blocks, 1 + toc_pages);
    let pdf = render_pdf(&blocks, &toc_entries, toc_pages, &layout);
    fs::write(&output, pdf)?;
    eprintln!("Wrote {}", output.display());
    Ok(())
}

fn ordered_source_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = vec![root.join("frontmatter.md")];

    let mut chapter_files = fs::read_dir(root)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| {
                    name.starts_with("chapter") && name.ends_with(".md")
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    chapter_files.sort_by(|left, right| {
        filename_number(left)
            .cmp(&filename_number(right))
            .then_with(|| left.cmp(right))
    });

    files.extend(chapter_files);
    files.push(root.join("appendix.md"));
    Ok(files)
}

fn filename_number(path: &Path) -> u32 {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            stem.chars()
                .filter(|ch| ch.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(u32::MAX)
        })
        .unwrap_or(u32::MAX)
}

fn output_path(args: impl IntoIterator<Item = String>) -> String {
    args.into_iter()
        .find(|arg| !arg.trim().is_empty())
        .unwrap_or_else(|| "dist/the-rust-programming-language.pdf".into())
}

#[derive(Debug, Clone)]
enum Block {
    Heading { level: u8, text: String },
    Paragraph(String),
    CodeBlock { title: Option<String>, code: String },
    Quote(String),
    List { ordered: bool, items: Vec<String> },
    Table(Vec<String>),
    Figure(String),
    Spacer(f32),
}

#[derive(Debug, Clone)]
struct TocEntry {
    title: String,
    level: u8,
    page: usize,
}

fn collect_toc_entries(blocks: &[Block]) -> Vec<TocEntry> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Heading { level, text } if *level <= 2 => Some(TocEntry {
                title: text.clone(),
                level: *level,
                page: 0,
            }),
            _ => None,
        })
        .collect()
}

fn toc_page_count(entries: &[TocEntry]) -> usize {
    let first_page_lines = 28;
    let later_page_lines = 38;
    if entries.len() + 3 <= first_page_lines {
        1
    } else {
        1 + (entries.len() + 3 - first_page_lines).div_ceil(later_page_lines)
    }
}

fn parse_file(path: &Path) -> io::Result<Vec<Block>> {
    let input = fs::read_to_string(path)?;
    let mut blocks = Vec::new();
    let mut lines = input.lines().peekable();
    let mut in_comment = false;
    let mut pending_code_title: Option<String> = None;

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if in_comment {
            if trimmed.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("<!--") {
            if !trimmed.contains("-->") {
                in_comment = true;
            }
            continue;
        }
        if trimmed.is_empty() {
            pending_code_title = None;
            continue;
        }
        if trimmed == "[TOC]" || trimmed.starts_with("<a id=") {
            continue;
        }
        if let Some(alt) = extract_img_alt(trimmed) {
            blocks.push(Block::Figure(alt));
            continue;
        }
        if trimmed.starts_with("```") {
            let mut code_lines = Vec::new();
            for code_line in lines.by_ref() {
                if code_line.trim_start().starts_with("```") {
                    break;
                }
                code_lines.push(code_line.to_string());
            }
            blocks.push(Block::CodeBlock {
                title: pending_code_title.take(),
                code: code_lines.join("\n"),
            });
            blocks.push(Block::Spacer(4.0));
            continue;
        }
        if is_standalone_code_title(trimmed, lines.peek().copied()) {
            pending_code_title = Some(trimmed.to_string());
            continue;
        }
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|ch| *ch == '#').count() as u8;
            let text = trimmed[level as usize..].trim().to_string();
            blocks.push(Block::Heading { level, text });
            continue;
        }
        if trimmed.starts_with('>') {
            let mut quote_lines = vec![trimmed.trim_start_matches('>').trim().to_string()];
            while let Some(next) = lines.peek() {
                if next.trim_start().starts_with('>') {
                    quote_lines.push(
                        next.trim_start()
                            .trim_start_matches('>')
                            .trim()
                            .to_string(),
                    );
                    lines.next();
                } else if next.trim().is_empty() {
                    lines.next();
                    break;
                } else {
                    break;
                }
            }
            blocks.push(Block::Quote(join_wrapped_lines(&quote_lines)));
            continue;
        }
        if is_list_item(trimmed) {
            let ordered = is_ordered_item(trimmed);
            let mut items = vec![strip_list_marker(trimmed)];
            while let Some(next) = lines.peek() {
                let next_trimmed = next.trim();
                if next_trimmed.is_empty() {
                    lines.next();
                    break;
                }
                if is_list_item(next_trimmed) && is_ordered_item(next_trimmed) == ordered {
                    items.push(strip_list_marker(next_trimmed));
                    lines.next();
                } else {
                    break;
                }
            }
            blocks.push(Block::List { ordered, items });
            continue;
        }
        if looks_like_table_row(trimmed) {
            let mut table = vec![trimmed.to_string()];
            while let Some(next) = lines.peek() {
                let next_trimmed = next.trim();
                if looks_like_table_row(next_trimmed) {
                    table.push(next_trimmed.to_string());
                    lines.next();
                } else {
                    break;
                }
            }
            blocks.push(Block::Table(table));
            continue;
        }

        let mut para_lines = vec![sanitize_inline_html(trimmed)];
        while let Some(next) = lines.peek() {
            let next_trimmed = next.trim();
            if next_trimmed.is_empty()
                || next_trimmed.starts_with('#')
                || next_trimmed.starts_with('>')
                || next_trimmed.starts_with("```")
                || looks_like_table_row(next_trimmed)
                || is_list_item(next_trimmed)
                || next_trimmed.starts_with("<!--")
                || next_trimmed.starts_with("<a id=")
            {
                break;
            }
            if let Some(alt) = extract_img_alt(next_trimmed) {
                if !para_lines.is_empty() {
                    blocks.push(Block::Paragraph(join_wrapped_lines(&para_lines)));
                    para_lines.clear();
                }
                blocks.push(Block::Figure(alt));
                lines.next();
                break;
            }
            para_lines.push(sanitize_inline_html(next_trimmed));
            lines.next();
        }
        if !para_lines.is_empty() {
            blocks.push(Block::Paragraph(join_wrapped_lines(&para_lines)));
        }
    }

    Ok(blocks)
}

fn sanitize_inline_html(input: &str) -> String {
    input
        .replace("<br>", " ")
        .replace("<br />", " ")
        .replace("<br/>", " ")
}

fn extract_img_alt(line: &str) -> Option<String> {
    let img_start = line.find("<img ")?;
    let alt_start = line[img_start..].find("alt=")? + img_start + 4;
    let quote = line[alt_start..].chars().next()?;
    let rest = &line[alt_start + quote.len_utf8()..];
    let alt_end = rest.find(quote)?;
    Some(format!("Figure: {}", &rest[..alt_end]))
}

fn is_standalone_code_title(line: &str, next: Option<&str>) -> bool {
    next.map(|next| next.trim_start().starts_with("```"))
        .unwrap_or(false)
        && !line.contains(' ')
        && (line.contains('/') || line.contains('.') || line.contains('-'))
}

fn is_list_item(line: &str) -> bool {
    line.starts_with("- ") || line.starts_with("* ") || is_ordered_item(line)
}

fn is_ordered_item(line: &str) -> bool {
    let mut chars = line.chars().peekable();
    let mut saw_digit = false;
    while let Some(ch) = chars.peek() {
        match ch {
            '0'..='9' => {
                saw_digit = true;
                chars.next();
            }
            '.' if saw_digit => {
                chars.next();
                return chars.next() == Some(' ');
            }
            _ => return false,
        }
    }
    false
}

fn strip_list_marker(line: &str) -> String {
    if let Some(stripped) = line.strip_prefix("- ") {
        stripped.to_string()
    } else if let Some(stripped) = line.strip_prefix("* ") {
        stripped.to_string()
    } else {
        let marker_end = line.find('.').unwrap_or(0);
        line[marker_end + 2..].to_string()
    }
}

fn looks_like_table_row(line: &str) -> bool {
    let pipes = line.matches('|').count();
    pipes >= 2 && !line.starts_with("[")
}

fn join_wrapped_lines(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Default)]
struct LayoutResult {
    heading_pages: Vec<usize>,
    body_page_count: usize,
}

fn layout_document(blocks: &[Block], start_page: usize) -> LayoutResult {
    let mut layout = LayoutResult::default();
    let mut page = start_page;
    let mut y = LETTER_HEIGHT - MARGIN_TOP;
    let body_width = LETTER_WIDTH - MARGIN_X * 2.0;

    for block in blocks {
        let height = block_height(block, body_width);
        if matches!(block, Block::Heading { .. }) {
            if y - height < MARGIN_BOTTOM {
                page += 1;
                y = LETTER_HEIGHT - MARGIN_TOP;
            }
            layout.heading_pages.push(page);
        } else if y - height < MARGIN_BOTTOM {
            page += 1;
            y = LETTER_HEIGHT - MARGIN_TOP;
        }
        y -= height;
    }

    layout.body_page_count = page - start_page + 1;
    layout
}

fn block_height(block: &Block, width: f32) -> f32 {
    match block {
        Block::Heading { level, text } => {
            let (size, spacing_before, spacing_after) = match level {
                1 => (CHAPTER_SIZE, 26.0, 12.0),
                2 => (SECTION_SIZE, 18.0, 8.0),
                _ => (SUBSECTION_SIZE, 14.0, 6.0),
            };
            let lines = wrap_text(text, width, size, Font::SansBold).len().max(1) as f32;
            spacing_before + lines * (size * 1.2) + spacing_after
        }
        Block::Paragraph(text) => wrap_text(text, width, BODY_FONT_SIZE, Font::Serif).len() as f32 * BODY_LEADING + 8.0,
        Block::Quote(text) => wrap_text(text, width - 24.0, BODY_FONT_SIZE, Font::SerifItalic).len() as f32 * BODY_LEADING + 18.0,
        Block::Figure(text) => wrap_text(text, width - 24.0, SMALL_SIZE, Font::Sans).len() as f32 * 13.0 + 18.0,
        Block::List { items, .. } => {
            items.iter().map(|item| wrap_text(item, width - 24.0, BODY_FONT_SIZE, Font::Serif).len() as f32 * BODY_LEADING + 4.0).sum::<f32>() + 6.0
        }
        Block::Table(lines) => lines.iter().map(|line| wrap_code_line(line, width - 20.0, CODE_FONT_SIZE).len() as f32 * CODE_LEADING).sum::<f32>() + 18.0,
        Block::CodeBlock { title, code } => {
            let title_height = title.as_ref().map(|_| 16.0).unwrap_or(0.0);
            let code_height = code
                .lines()
                .map(|line| wrap_code_line(line, width - 20.0, CODE_FONT_SIZE).len().max(1) as f32 * CODE_LEADING)
                .sum::<f32>();
            title_height + code_height + 20.0
        }
        Block::Spacer(space) => *space,
    }
}

#[derive(Clone, Copy)]
enum Font {
    Sans,
    SansBold,
    Serif,
    SerifBold,
    SerifItalic,
    Mono,
}

impl Font {
    fn pdf_name(self) -> &'static str {
        match self {
            Font::Sans => "F1",
            Font::SansBold => "F2",
            Font::Serif => "F3",
            Font::SerifBold => "F4",
            Font::SerifItalic => "F5",
            Font::Mono => "F6",
        }
    }

    fn average_width_factor(self) -> f32 {
        match self {
            Font::Sans | Font::SansBold => 0.53,
            Font::Serif | Font::SerifBold | Font::SerifItalic => 0.51,
            Font::Mono => 0.60,
        }
    }
}

fn wrap_text(text: &str, width: f32, font_size: f32, font: Font) -> Vec<String> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if estimated_width(&candidate, font_size, font) <= width {
            current = candidate;
        } else {
            if !current.is_empty() {
                lines.push(current);
            }
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn wrap_code_line(line: &str, width: f32, font_size: f32) -> Vec<String> {
    let max_chars = ((width / (font_size * Font::Mono.average_width_factor())).floor() as usize).max(24);
    if line.chars().count() <= max_chars {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in line.chars() {
        current.push(ch);
        if current.chars().count() >= max_chars {
            lines.push(current);
            current = String::from("  ");
        }
    }
    if !current.trim().is_empty() {
        lines.push(current);
    }
    lines
}

fn estimated_width(text: &str, font_size: f32, font: Font) -> f32 {
    text.chars().count() as f32 * font_size * font.average_width_factor()
}

fn render_pdf(
    blocks: &[Block],
    toc_entries: &[TocEntry],
    toc_pages: usize,
    layout: &LayoutResult,
) -> Vec<u8> {
    let mut doc = PdfBuilder::new();
    let mut page_number = 1usize;

    let mut title_page = PageCanvas::new(page_number);
    render_title_page(&mut title_page);
    doc.push_page(title_page.finish());
    page_number += 1;

    let mut resolved_toc = Vec::new();
    let mut heading_index = 0usize;
    for entry in toc_entries {
        let mut resolved = entry.clone();
        resolved.page = layout.heading_pages[heading_index];
        heading_index += 1;
        resolved_toc.push(resolved);
    }

    let toc_chunks = chunk_toc_entries(&resolved_toc, toc_pages);
    for (i, chunk) in toc_chunks.iter().enumerate() {
        let mut page = PageCanvas::new(page_number);
        render_toc_page(&mut page, chunk, i == 0);
        doc.push_page(page.finish());
        page_number += 1;
    }

    let mut body = BodyRenderer::new(page_number);
    for block in blocks {
        body.render_block(block);
    }
    for page in body.finish() {
        doc.push_page(page);
    }

    doc.finish()
}

fn chunk_toc_entries(entries: &[TocEntry], page_count: usize) -> Vec<Vec<TocEntry>> {
    let mut chunks = Vec::new();
    let mut remaining = entries;
    for index in 0..page_count {
        let cap = if index == 0 { 25 } else { 36 };
        let take = remaining.len().min(cap);
        chunks.push(remaining[..take].to_vec());
        remaining = &remaining[take..];
    }
    while chunks.len() < page_count {
        chunks.push(Vec::new());
    }
    chunks
}

fn render_title_page(page: &mut PageCanvas) {
    page.text_centered(LETTER_HEIGHT - 170.0, TITLE_SIZE, Font::SansBold, "The Rust Programming Language");
    page.text_centered(LETTER_HEIGHT - 205.0, 14.0, Font::SerifItalic, "A PDF edition generated from the No Starch print-ready sources");
    page.draw_rule(120.0, LETTER_HEIGHT - 225.0, LETTER_WIDTH - 120.0, LETTER_HEIGHT - 225.0, 1.0, (0.75, 0.22, 0.17));
    page.text_centered(LETTER_HEIGHT - 280.0, 13.0, Font::Serif, "Steve Klabnik · Carol Nichols · Chris Krycho");
    page.text_centered(LETTER_HEIGHT - 312.0, 11.0, Font::Serif, "with contributions from the Rust community");
    page.text_centered(120.0, 10.0, Font::Sans, "Built locally from the book repository's `nostarch/` sources.");
}

fn render_toc_page(page: &mut PageCanvas, entries: &[TocEntry], first_page: bool) {
    let mut y = LETTER_HEIGHT - MARGIN_TOP;
    if first_page {
        page.text(MARGIN_X, y, 22.0, Font::SansBold, "Contents");
        y -= 32.0;
        page.text(MARGIN_X, y, 10.5, Font::Serif, "Chapter openings and major sections.");
        y -= 28.0;
    }

    for entry in entries {
        let indent = if entry.level == 1 { 0.0 } else { 18.0 };
        let font = if entry.level == 1 { Font::SerifBold } else { Font::Serif };
        let size = if entry.level == 1 { 11.0 } else { 10.5 };
        let title_width = LETTER_WIDTH - MARGIN_X * 2.0 - indent - 36.0;
        let lines = wrap_text(&entry.title, title_width, size, font);
        for (index, line) in lines.iter().enumerate() {
            let x = MARGIN_X + indent;
            page.text(x, y, size, font, line);
            if index == 0 {
                let page_label = entry.page.to_string();
                let number_width = estimated_width(&page_label, size, Font::Sans);
                page.text(LETTER_WIDTH - MARGIN_X - number_width, y, size, Font::Sans, &page_label);
                let line_end = LETTER_WIDTH - MARGIN_X - number_width - 6.0;
                let line_start = x + estimated_width(line, size, font) + 8.0;
                if line_end > line_start {
                    page.draw_dots(line_start, y - 2.0, line_end, 1.8);
                }
            }
            y -= 15.0;
        }
        y -= if entry.level == 1 { 5.0 } else { 2.0 };
    }
}

struct BodyRenderer {
    current: PageCanvas,
    pages: Vec<PageContent>,
    y: f32,
}

impl BodyRenderer {
    fn new(start_page: usize) -> Self {
        Self {
            current: PageCanvas::new(start_page),
            pages: Vec::new(),
            y: LETTER_HEIGHT - MARGIN_TOP,
        }
    }

    fn ensure_space(&mut self, height: f32) {
        if self.y - height < MARGIN_BOTTOM {
            let next_number = self.current.page_number + 1;
            let old = std::mem::replace(&mut self.current, PageCanvas::new(next_number));
            self.pages.push(old.finish());
            self.y = LETTER_HEIGHT - MARGIN_TOP;
        }
    }

    fn render_block(&mut self, block: &Block) {
        let width = LETTER_WIDTH - MARGIN_X * 2.0;
        let height = block_height(block, width);
        if matches!(block, Block::Heading { .. }) {
            self.ensure_space(height);
        } else {
            self.ensure_space(height);
        }

        match block {
            Block::Heading { level, text } => self.render_heading(*level, text),
            Block::Paragraph(text) => self.render_paragraph(text),
            Block::Quote(text) => self.render_quote(text),
            Block::Figure(text) => self.render_figure(text),
            Block::List { ordered, items } => self.render_list(*ordered, items),
            Block::Table(lines) => self.render_table(lines),
            Block::CodeBlock { title, code } => self.render_code_block(title.as_deref(), code),
            Block::Spacer(space) => self.y -= *space,
        }
    }

    fn render_heading(&mut self, level: u8, text: &str) {
        let (size, font, before, after, color) = match level {
            1 => (CHAPTER_SIZE, Font::SansBold, 26.0, 12.0, (0.62, 0.14, 0.11)),
            2 => (SECTION_SIZE, Font::SansBold, 18.0, 8.0, (0.18, 0.22, 0.32)),
            _ => (SUBSECTION_SIZE, Font::SansBold, 14.0, 6.0, (0.18, 0.22, 0.32)),
        };
        self.y -= before;
        let lines = wrap_text(text, LETTER_WIDTH - MARGIN_X * 2.0, size, font);
        for line in lines {
            self.current.text_colored(MARGIN_X, self.y, size, font, color, &line);
            self.y -= size * 1.2;
        }
        if level == 1 {
            self.current.draw_rule(MARGIN_X, self.y + 4.0, LETTER_WIDTH - MARGIN_X, self.y + 4.0, 1.2, color);
        }
        self.y -= after;
    }

    fn render_paragraph(&mut self, text: &str) {
        let lines = wrap_text(text, LETTER_WIDTH - MARGIN_X * 2.0, BODY_FONT_SIZE, Font::Serif);
        for line in lines {
            self.current.text(MARGIN_X, self.y, BODY_FONT_SIZE, Font::Serif, &line);
            self.y -= BODY_LEADING;
        }
        self.y -= 8.0;
    }

    fn render_quote(&mut self, text: &str) {
        let box_x = MARGIN_X;
        let box_width = LETTER_WIDTH - MARGIN_X * 2.0;
        let inner_x = box_x + 18.0;
        let lines = wrap_text(text, box_width - 32.0, BODY_FONT_SIZE, Font::SerifItalic);
        let box_height = lines.len() as f32 * BODY_LEADING + 12.0;
        self.current.fill_rect(box_x, self.y - box_height + 5.0, box_width, box_height, (0.97, 0.96, 0.93));
        self.current.fill_rect(box_x, self.y - box_height + 5.0, 5.0, box_height, (0.75, 0.22, 0.17));
        for line in lines {
            self.current.text(inner_x, self.y, BODY_FONT_SIZE, Font::SerifItalic, &line);
            self.y -= BODY_LEADING;
        }
        self.y -= 8.0;
    }

    fn render_figure(&mut self, text: &str) {
        let lines = wrap_text(text, LETTER_WIDTH - MARGIN_X * 2.0 - 24.0, SMALL_SIZE, Font::Sans);
        let box_height = lines.len() as f32 * 13.0 + 8.0;
        self.current.fill_rect(MARGIN_X, self.y - box_height + 4.0, LETTER_WIDTH - MARGIN_X * 2.0, box_height, (0.95, 0.97, 0.99));
        for line in lines {
            self.current.text(MARGIN_X + 12.0, self.y, SMALL_SIZE, Font::Sans, &line);
            self.y -= 13.0;
        }
        self.y -= 8.0;
    }

    fn render_list(&mut self, ordered: bool, items: &[String]) {
        for (index, item) in items.iter().enumerate() {
            let marker = if ordered {
                format!("{}.", index + 1)
            } else {
                "•".into()
            };
            let marker_width = estimated_width(&marker, BODY_FONT_SIZE, Font::SansBold);
            let lines = wrap_text(item, LETTER_WIDTH - MARGIN_X * 2.0 - 24.0, BODY_FONT_SIZE, Font::Serif);
            if let Some(first) = lines.first() {
                self.current.text(MARGIN_X, self.y, BODY_FONT_SIZE, Font::SansBold, &marker);
                self.current.text(MARGIN_X + marker_width + 8.0, self.y, BODY_FONT_SIZE, Font::Serif, first);
                self.y -= BODY_LEADING;
            }
            for line in lines.iter().skip(1) {
                self.current.text(MARGIN_X + marker_width + 8.0, self.y, BODY_FONT_SIZE, Font::Serif, line);
                self.y -= BODY_LEADING;
            }
            self.y -= 4.0;
        }
        self.y -= 2.0;
    }

    fn render_table(&mut self, lines: &[String]) {
        let total_height = lines
            .iter()
            .map(|line| wrap_code_line(line, LETTER_WIDTH - MARGIN_X * 2.0 - 20.0, CODE_FONT_SIZE).len().max(1) as f32 * CODE_LEADING)
            .sum::<f32>() + 10.0;
        self.current.fill_rect(MARGIN_X, self.y - total_height + 4.0, LETTER_WIDTH - MARGIN_X * 2.0, total_height, (0.96, 0.96, 0.96));
        for line in lines {
            for wrapped in wrap_code_line(line, LETTER_WIDTH - MARGIN_X * 2.0 - 20.0, CODE_FONT_SIZE) {
                self.current.text(MARGIN_X + 10.0, self.y, CODE_FONT_SIZE, Font::Mono, &wrapped);
                self.y -= CODE_LEADING;
            }
        }
        self.y -= 8.0;
    }

    fn render_code_block(&mut self, title: Option<&str>, code: &str) {
        let width = LETTER_WIDTH - MARGIN_X * 2.0;
        let title_height = title.map(|_| 16.0).unwrap_or(0.0);
        let code_height = code
            .lines()
            .map(|line| wrap_code_line(line, width - 20.0, CODE_FONT_SIZE).len().max(1) as f32 * CODE_LEADING)
            .sum::<f32>();
        let total_height = title_height + code_height + 12.0;
        self.current.fill_rect(MARGIN_X, self.y - total_height + 4.0, width, total_height, (0.965, 0.967, 0.972));
        if let Some(title) = title {
            self.current.text(MARGIN_X + 10.0, self.y, 9.6, Font::SansBold, title);
            self.y -= 16.0;
        }
        for line in code.lines() {
            for wrapped in wrap_code_line(line, width - 20.0, CODE_FONT_SIZE) {
                self.current.text(MARGIN_X + 10.0, self.y, CODE_FONT_SIZE, Font::Mono, &wrapped);
                self.y -= CODE_LEADING;
            }
        }
        self.y -= 8.0;
    }

    fn finish(mut self) -> Vec<PageContent> {
        self.pages.push(self.current.finish());
        self.pages
    }
}

#[derive(Debug, Clone)]
struct PageContent {
    stream: Vec<u8>,
}

struct PageCanvas {
    page_number: usize,
    content: String,
}

impl PageCanvas {
    fn new(page_number: usize) -> Self {
        Self { page_number, content: String::new() }
    }

    fn text(&mut self, x: f32, y: f32, size: f32, font: Font, text: &str) {
        self.text_colored(x, y, size, font, (0.12, 0.12, 0.12), text);
    }

    fn text_centered(&mut self, y: f32, size: f32, font: Font, text: &str) {
        let width = estimated_width(text, size, font);
        let x = (LETTER_WIDTH - width) / 2.0;
        self.text(x, y, size, font, text);
    }

    fn text_colored(&mut self, x: f32, y: f32, size: f32, font: Font, color: (f32, f32, f32), text: &str) {
        let escaped = escape_pdf_text(text);
        self.content.push_str(&format!("BT /{} {} Tf {} {} {} rg 1 0 0 1 {:.2} {:.2} Tm ({}) Tj ET\n", font.pdf_name(), size, color.0, color.1, color.2, x, y, escaped));
    }

    fn draw_rule(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: (f32, f32, f32)) {
        self.content.push_str(&format!("q {} {} {} RG {} w {:.2} {:.2} m {:.2} {:.2} l S Q\n", color.0, color.1, color.2, width, x1, y1, x2, y2));
    }

    fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: (f32, f32, f32)) {
        self.content.push_str(&format!("q {} {} {} rg {:.2} {:.2} {:.2} {:.2} re f Q\n", color.0, color.1, color.2, x, y, width, height));
    }

    fn draw_dots(&mut self, x1: f32, y: f32, x2: f32, step: f32) {
        let mut x = x1;
        while x < x2 {
            self.fill_rect(x, y, 1.0, 1.0, (0.55, 0.55, 0.55));
            x += step;
        }
    }

    fn finish(mut self) -> PageContent {
        let label = self.page_number.to_string();
        let footer_width = estimated_width(&label, 10.0, Font::Sans);
        self.text((LETTER_WIDTH - footer_width) / 2.0, 28.0, 10.0, Font::Sans, &label);
        PageContent { stream: self.content.into_bytes() }
    }
}

struct PdfBuilder {
    pages: Vec<PageContent>,
}

impl PdfBuilder {
    fn new() -> Self {
        Self { pages: Vec::new() }
    }

    fn push_page(&mut self, page: PageContent) {
        self.pages.push(page);
    }

    fn finish(self) -> Vec<u8> {
        let page_count = self.pages.len();
        let mut objects: Vec<Vec<u8>> = Vec::new();

        objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
        objects.push(Vec::new());
        objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
        objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_vec());
        objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman >>".to_vec());
        objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Bold >>".to_vec());
        objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Italic >>".to_vec());
        objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>".to_vec());

        let first_page_object = objects.len() + 1;
        let first_stream_object = first_page_object + page_count;

        for (index, _page) in self.pages.iter().enumerate() {
            let stream_id = first_stream_object + index;
            let page_obj = format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R /F5 7 0 R /F6 8 0 R >> >> /Contents {} 0 R >>",
                LETTER_WIDTH, LETTER_HEIGHT, stream_id
            );
            objects.push(page_obj.into_bytes());
        }

        for page in &self.pages {
            let mut stream = format!("<< /Length {} >>\nstream\n", page.stream.len()).into_bytes();
            stream.extend_from_slice(&page.stream);
            stream.extend_from_slice(b"endstream");
            objects.push(stream);
        }

        let kids = (0..page_count)
            .map(|index| format!("{} 0 R", first_page_object + index))
            .collect::<Vec<_>>()
            .join(" ");
        objects[1] = format!("<< /Type /Pages /Count {} /Kids [{}] >>", page_count, kids).into_bytes();

        let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = vec![0usize];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            pdf.extend_from_slice(object);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets.iter().skip(1) {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                objects.len() + 1,
                xref_offset
            )
            .as_bytes(),
        );
        pdf
    }
}

fn escape_pdf_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_image_alt_text() {
        let alt = extract_img_alt(r#"<img alt="Example diagram" src="img/example.svg" />"#);
        assert_eq!(alt.as_deref(), Some("Figure: Example diagram"));
    }

    #[test]
    fn wraps_text_without_losing_words() {
        let lines = wrap_text(
            "Rust makes systems programming more approachable.",
            120.0,
            BODY_FONT_SIZE,
            Font::Serif,
        );
        assert!(lines.len() >= 2);
        assert_eq!(lines.join(" "), "Rust makes systems programming more approachable.");
    }

    #[test]
    fn strips_list_markers() {
        assert_eq!(strip_list_marker("- hello"), "hello");
        assert_eq!(strip_list_marker("* hello"), "hello");
        assert_eq!(strip_list_marker("12. hello"), "hello");
    }

    #[test]
    fn defaults_output_path_when_no_argument_is_passed() {
        assert_eq!(
            output_path(Vec::<String>::new()),
            "dist/the-rust-programming-language.pdf"
        );
    }

    #[test]
    fn defaults_output_path_when_argument_is_empty() {
        assert_eq!(
            output_path(vec![String::new()]),
            "dist/the-rust-programming-language.pdf"
        );
    }
}
