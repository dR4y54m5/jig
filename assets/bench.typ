// Page and typography rules for documents rendered by `jig doc pdf` and `jig doc pack`.
//
// A single document uses `bench-doc`. A gate package uses `bench-pack` and wraps
// each included document in `doc-section`. Every document places a metadata
// marker, which the header, footer and draft watermark read to know which
// document a page belongs to.

#let ink = rgb("#1f2933")
#let muted = rgb("#616e7c")
#let rule = rgb("#cbd2d9")
#let tint = rgb("#eef2f6")
#let accent = rgb("#1f4e79")
#let alert-colors = (
  note: rgb("#1f6feb"),
  tip: rgb("#1a7f37"),
  important: rgb("#8250df"),
  warning: rgb("#9a6700"),
  caution: rgb("#cf222e"),
)

#let sans = ("Helvetica Neue", "Helvetica", "Arial", "Apple Symbols", "DejaVu Sans Mono")
#let mono = ("Menlo", "DejaVu Sans Mono")

#let status-names = (
  draft: "Draft",
  in-review: "In review",
  released: "Released",
  superseded: "Superseded",
  generated: "Generated",
)
#let watermarked = ("draft", "in-review", "superseded")

#let status-name(status) = status-names.at(status, default: status)

#let revision-label(doc) = {
  if doc.revision == "" { return "–" }
  let rev = "Rev " + doc.revision
  if doc.status in watermarked { rev + " (" + lower(status-name(doc.status)) + ")" } else { rev }
}

// --- Document markers -------------------------------------------------------

#let doc-marker(doc) = [#metadata(doc) <bench-doc>]

// The document a page belongs to: the first one starting on the page, else the
// last one started on an earlier page.
#let page-doc() = {
  let page = here().page()
  let markers = query(<bench-doc>)
  let starting = markers.filter(m => m.location().page() == page)
  if starting.len() > 0 { return starting.first().value }
  let earlier = markers.filter(m => m.location().page() < page)
  if earlier.len() > 0 { earlier.last().value } else { none }
}

// A link to another document in the same PDF, or plain text when it is absent.
#let doc-ref(id, body) = context {
  let hits = query(<bench-doc>).filter(m => m.value.id == id)
  if hits.len() > 0 { link(hits.first().location(), body) } else { body }
}

// --- Page furniture ---------------------------------------------------------

#let page-header() = context {
  let doc = page-doc()
  if doc == none { return }
  set text(size: 7.5pt, fill: muted)
  grid(
    columns: (auto, 1fr),
    align: (left, right),
    [*#doc.id* #h(0.4em) #revision-label(doc)], doc.title,
  )
  v(-6pt)
  line(length: 100%, stroke: 0.4pt + rule)
}

#let page-footer(project) = context {
  set text(size: 7.5pt, fill: muted)
  line(length: 100%, stroke: 0.4pt + rule)
  v(-6pt)
  grid(
    columns: (1fr, auto),
    project, [Page #counter(page).display("1 of 1", both: true)],
  )
}

#let page-background() = context {
  let doc = page-doc()
  if doc != none and doc.status in watermarked {
    place(center + horizon, rotate(-40deg, text(
      size: 90pt,
      weight: "bold",
      fill: alert-colors.caution.transparentize(90%),
      upper(status-name(doc.status)),
    )))
  }
}

// --- Blocks used by rendered Markdown ----------------------------------------

#let callout(kind, body) = {
  let color = alert-colors.at(kind, default: muted)
  block(
    width: 100%,
    inset: (left: 10pt, right: 8pt, y: 7pt),
    stroke: (left: 2.5pt + color),
    fill: color.transparentize(93%),
    {
      text(size: 7.5pt, weight: "bold", fill: color, upper(kind))
      v(-4pt)
      body
    },
  )
}

// A rendered diagram at its natural size, shrunk to the text width if wider.
#let diagram(path, alt: none) = layout(size => {
  let natural = measure(image(path)).width
  let width = if natural > size.width { size.width } else { natural }
  align(center, block(above: 10pt, below: 10pt, image(path, width: width, alt: alt)))
})

#let body-style(body) = {
  set text(font: sans, size: 9.5pt, fill: ink, lang: "en")
  set par(leading: 0.6em, spacing: 0.95em, justify: false)
  show heading: set text(fill: ink)
  show heading.where(level: 1): it => block(above: 1.5em, below: 0.75em,
    text(size: 13pt, weight: "bold", fill: accent, it.body))
  show heading.where(level: 2): it => block(above: 1.25em, below: 0.6em,
    text(size: 10.5pt, weight: "bold", it.body))
  show heading.where(level: 3): it => block(above: 1.1em, below: 0.5em,
    text(size: 9.5pt, weight: "bold", style: "italic", it.body))
  show link: set text(fill: accent)
  show raw: set text(font: mono, size: 8.5pt)
  show raw.where(block: true): it => block(width: 100%, fill: luma(246), inset: 8pt, radius: 2pt, it)
  show quote.where(block: true): it => block(
    width: 100%, inset: (left: 10pt, y: 4pt), stroke: (left: 2pt + rule), text(fill: muted, it.body))
  set table(
    inset: (x: 5pt, y: 4pt),
    stroke: (_, y) => (bottom: 0.4pt + rule),
    fill: (_, y) => if y == 0 { tint },
  )
  show table.cell.where(y: 0): set text(weight: "bold")
  // Pandoc wraps every table in a centered, unbreakable figure. Unwrap it so
  // tables sit flush left, align their cells left and can break across pages.
  show figure.where(kind: table): set block(breakable: true)
  show figure.where(kind: table): it => {
    show align: a => a.body
    set align(left)
    it.body
  }
  set list(indent: 0.4em)
  set enum(indent: 0.4em)
  body
}

// --- Title block -------------------------------------------------------------

#let title-block(doc) = block(width: 100%, below: 16pt, {
  text(size: 8.5pt, fill: muted, weight: "medium", doc.project-title)
  v(-2pt)
  text(size: 19pt, weight: "bold", doc.title)
  v(2pt)
  let label(body) = text(fill: muted, body)
  table(
    columns: (auto, 1fr, auto, 1fr),
    inset: (x: 6pt, y: 4pt),
    stroke: 0.4pt + rule,
    fill: (x, _) => if calc.even(x) { tint },
    label[Document], doc.id, label[Revision], revision-label(doc),
    label[Type], doc.kind-title, label[Status], status-name(doc.status),
    label[Project], doc.project, label[Date], doc.date,
    label[Author], doc.author, label[Gate], doc.gate,
  )
})

// --- Entry points ------------------------------------------------------------

#let page-setup(project-title, body) = {
  set page(
    paper: "a4",
    margin: (x: 2cm, top: 2.4cm, bottom: 2.2cm),
    header: page-header(),
    footer: page-footer(project-title),
    background: page-background(),
  )
  show: body-style
  body
}

#let bench-doc(doc, body) = {
  set document(title: doc.id + " " + doc.title, author: doc.author)
  show: page-setup.with(doc.project-title)
  doc-marker(doc)
  title-block(doc)
  body
}

#let doc-section(doc, body) = {
  pagebreak(weak: true)
  doc-marker(doc)
  title-block(doc)
  body
}

#let pack-cover(pack) = {
  v(2.5cm)
  text(size: 9pt, fill: muted, weight: "medium", pack.project-title)
  v(-2pt)
  text(size: 24pt, weight: "bold", pack.title)
  v(-6pt)
  text(size: 12pt, fill: muted)[Gate review package]
  v(0.8cm)
  table(
    columns: (auto, 1fr),
    inset: (x: 6pt, y: 5pt),
    stroke: 0.4pt + rule,
    fill: (x, _) => if x == 0 { tint },
    text(fill: muted)[Project], pack.project,
    text(fill: muted)[Gate], pack.gate-title,
    text(fill: muted)[Question], pack.question,
    text(fill: muted)[Phase], pack.phase,
    text(fill: muted)[Prepared], pack.date,
    text(fill: muted)[Prepared by], pack.author,
  )
  v(0.6cm)
  text(size: 11pt, weight: "bold", fill: accent)[Package contents]
  v(-2pt)
  table(
    columns: (auto, 1fr, auto, auto),
    ..([Document], [Title], [Revision], [Status]),
    ..pack.docs.map(d => (doc-ref(d.id, d.id), d.title, revision-label(d), status-name(d.status))).flatten(),
  )
}

#let bench-pack(pack, body) = {
  set document(title: pack.project + " " + pack.gate + " gate review package", author: pack.author)
  show: page-setup.with(pack.project-title)
  pack-cover(pack)
  body
}
