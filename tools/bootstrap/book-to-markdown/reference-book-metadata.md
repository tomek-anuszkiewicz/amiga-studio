# Reference Book Metadata

Every Markdown file in a book directory under `Obsidian/Amiga/Reference/` has
line-one frontmatter containing `title`, `book`, `chapter` and `tags`. `book` names
the source title or composite publication identifier; `chapter` names its section
or unit, including contents/front matter. Replace `manual` with `book` without
changing the value. Tags include `reference`, `book`, `chapter`, retained relevant
tags and content-supported topics; reuse lowercase hyphenated tags and include
`amiga` for Amiga content. Do not infer topics from the book title alone.

The canonical gate checks every local vault Markdown file, including ignored books.
If metadata is missing, report affected paths and proposed frontmatter for each and
obtain approval before editing those source notes. The verifier does not repair them.
Structural acceptance does not replace the topic-selection review.
