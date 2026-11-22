# Word filters and their sources

The report uses two local word lists. Lines beginning with `#` are comments;
other lines contain individual lowercase word forms. The lists affect this report
only. They do not change the repository's spelling checker or verification gates.

## Common English

`assets/common-words.txt` is derived from the New General Service List 1.2 by
Charles Browne, Brent Culligan and Joseph Phillips. The 2,809 headwords, their
listed inflected forms and the supplementary words provide a practical filter
for general English. They are not an exhaustive classification of A1–B2 words.

The initial list combines the word cells from these official downloads, removes
comment lines and duplicate forms, converts letters to lowercase, and sorts the
result:

- [NGSL 1.2 lemmatized for research](https://www.newgeneralservicelist.com/s/NGSL_12_lemmatized_for_research.csv).
- [NGSL supplementary words](https://www.newgeneralservicelist.com/s/SUP_lemmatized.csv).

The [NGSL project](https://www.newgeneralservicelist.com/new-general-service-list)
publishes the source material under
[Creative Commons Attribution-ShareAlike 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
That license and attribution apply to the derived common-word list. The changes
described above concern its format; the report does not download updates.

## Project vocabulary

`assets/project-words.txt` is a static starting list of hardware names, tools,
formats and ordinary project terms, including `amiga`, `git`, `workflow` and
`milestone`. It was assembled from the repository's existing programming,
hardware, software and name vocabularies under
`tools/verification/checks/english_words/`, with the terms selected during the
initial word-frequency analysis. Later spelling-dictionary changes do not alter
this report's filter automatically.

A word's repeated appearance in instructions does not by itself make it project
vocabulary. Consider what it names and inspect its uses. For example, `closure`
and `explicit` remain candidates: their presence may need an explanation of the
actual action or condition.

During a report, suggest additions without changing either list. Identify whether
each suggestion is ordinary English or an established project term, and give a
reason based on the inspected examples. Update the appropriate list when the user
selects that maintenance. Do not replace contextual assessment with a growing
list of words that have become familiar through repetition.
