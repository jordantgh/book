#let rust_book_letter_layout = (
  paper: "us-letter",
  margin: (
    top: 0.9in,
    bottom: 0.95in,
    inside: 0.95in,
    outside: 0.8in,
  ),
  body_size: 10.5pt,
  body_leading: 0.68em,
  heading_1_size: 20pt,
  heading_2_size: 15pt,
  heading_3_size: 12pt,
  heading_4_size: 10.8pt,
  code_block_size: 8.6pt,
  inline_code_size: 0.92em,
  quote_size: 9.7pt,
  figure_caption_size: 8.6pt,
  frontmatter_top_spacing: 18%,
  title_size: 28pt,
  byline_size: 12pt,
  credit_size: 10pt,
)

#let rust_book_tablet_layout = (
  paper: "a5",
  margin: (
    top: 0.72in,
    bottom: 0.82in,
    inside: 0.82in,
    outside: 0.82in,
  ),
  body_size: 10.7pt,
  body_leading: 0.72em,
  heading_1_size: 18pt,
  heading_2_size: 14pt,
  heading_3_size: 11.4pt,
  heading_4_size: 10.4pt,
  code_block_size: 8.2pt,
  inline_code_size: 0.9em,
  quote_size: 9.8pt,
  figure_caption_size: 8.4pt,
  frontmatter_top_spacing: 12%,
  title_size: 24pt,
  byline_size: 11pt,
  credit_size: 9.5pt,
)

// Change this single binding to pick a different reading profile.
#let rust_book_layout = rust_book_tablet_layout
