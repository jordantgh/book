#let ink = rgb("#1f2328")
#let rule = rgb("#d0d5dd")
#let code-fill = rgb("#f8f5ef")
#let code-stroke = rgb("#ded6c8")
#let note-fill = rgb("#f4f7fb")
#let note-stroke = rgb("#c7d2e0")

#let rust_book_theme(doc, config) = {
  set page(
    paper: config.paper,
    margin: config.margin,
    numbering: "1",
  )
  set text(
    font: "Libertinus Serif",
    size: config.body_size,
    fill: ink,
    lang: "en",
  )
  set par(justify: true, leading: config.body_leading)

  show heading.where(level: 1): it => [
    #pagebreak(weak: true)
    #it
  ]
  show heading.where(level: 1): set text(size: config.heading_1_size, weight: "bold")
  show heading.where(level: 2): set text(size: config.heading_2_size, weight: "semibold")
  show heading.where(level: 3): set text(size: config.heading_3_size, weight: "semibold")
  show heading.where(level: 4): set text(size: config.heading_4_size, weight: "semibold")
  show heading.where(level: 1): set block(above: 0pt, below: 1.1em)
  show heading.where(level: 2): set block(above: 1.5em, below: 0.7em)
  show heading.where(level: 3): set block(above: 1.15em, below: 0.5em)
  show heading.where(level: 4): set block(above: 0.9em, below: 0.35em)

  show raw.where(block: true): it => block(
    fill: code-fill,
    stroke: (paint: code-stroke, thickness: 0.6pt),
    inset: (x: 11pt, y: 9pt),
    radius: 6pt,
  )[
    #set text(font: "DejaVu Sans Mono", size: config.code_block_size)
    #it
  ]
  show raw.where(block: false): set text(
    font: "DejaVu Sans Mono",
    size: config.inline_code_size,
  )

  show quote.where(block: true): it => block(
    fill: note-fill,
    stroke: (paint: note-stroke, thickness: 0.8pt),
    inset: (x: 11pt, y: 8pt),
    radius: 6pt,
  )[
    #set text(size: config.quote_size)
    #it.body
  ]

  set table(
    inset: 6pt,
    stroke: (paint: rule, thickness: 0.5pt),
  )
  show figure.caption: set text(
    size: config.figure_caption_size,
    fill: config.figure_caption_fill,
  )

  doc
}

#let book_listing_with(
  config,
  number: none,
  file_name: none,
  caption: none,
  lang: "",
  code: "",
) = [
  #if file_name != none [
    #text(
      font: "DejaVu Sans Mono",
      size: config.file_name_size,
      fill: config.file_name_fill,
    )[
      Filename: #file_name
    ]
    #v(0.35em)
  ]

  #let rendered = if lang == "" {
    raw(code, block: true)
  } else {
    raw(code, block: true, lang: lang)
  }

  #rendered

  #if caption != none or number != none [
    #v(0.45em)
    #text(
      size: config.listing_caption_size,
      fill: config.listing_caption_fill,
    )[
      #if number != none [
        #strong[Listing #number]
        #if caption != none [: ]
      ]
      #if caption != none [#caption]
    ]
  ]
]
