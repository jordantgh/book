#let ink = rgb("#1f2328")
#let quiet = rgb("#667085")
#let rule = rgb("#d0d5dd")
#let code-fill = rgb("#f8f5ef")
#let code-stroke = rgb("#ded6c8")
#let note-fill = rgb("#f4f7fb")
#let note-stroke = rgb("#c7d2e0")

#let rust_book_theme(doc) = {
  set page(
    paper: "us-letter",
    margin: (
      top: 0.9in,
      bottom: 0.95in,
      inside: 0.95in,
      outside: 0.8in,
    ),
    numbering: "1",
  )
  set text(
    font: "Libertinus Serif",
    size: 10.5pt,
    fill: ink,
    lang: "en",
  )
  set par(justify: true, leading: 0.68em)

  show heading.where(level: 1): it => [
    #pagebreak(weak: true)
    #it
  ]
  show heading.where(level: 1): set text(size: 20pt, weight: "bold")
  show heading.where(level: 2): set text(size: 15pt, weight: "semibold")
  show heading.where(level: 3): set text(size: 12pt, weight: "semibold")
  show heading.where(level: 4): set text(size: 10.8pt, weight: "semibold")
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
    #set text(font: "DejaVu Sans Mono", size: 8.6pt)
    #it
  ]
  show raw.where(block: false): set text(
    font: "DejaVu Sans Mono",
    size: 0.92em,
  )

  show quote.where(block: true): it => block(
    fill: note-fill,
    stroke: (paint: note-stroke, thickness: 0.8pt),
    inset: (x: 11pt, y: 8pt),
    radius: 6pt,
  )[
    #set text(size: 9.7pt)
    #it.body
  ]

  set table(
    inset: 6pt,
    stroke: (paint: rule, thickness: 0.5pt),
  )
  show figure.caption: set text(size: 8.6pt, fill: quiet)

  doc
}

#let book_listing(
  number: none,
  file_name: none,
  caption: none,
  lang: "",
  code: "",
) = [
  #if file_name != none [
    #text(font: "DejaVu Sans Mono", size: 8pt, fill: quiet)[
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
    #text(size: 8.4pt, fill: quiet)[
      #if number != none [
        #strong[Listing #number]
        #if caption != none [: ]
      ]
      #if caption != none [#caption]
    ]
  ]
]
