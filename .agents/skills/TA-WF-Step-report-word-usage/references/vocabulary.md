# Word filters and their sources

The report uses two local lists to exclude words from its ranking. Lines
beginning with `#` are comments. Each other line contains one lowercase word
form. The repository's spelling checker and verification gates use their own
rules.

## Common English

`assets/common-words.txt` draws on the New General Service List 1.2 by Charles
Browne, Brent Culligan and Joseph Phillips. Its 2,809 headwords, listed
inflected forms and supplementary words provide a practical filter for general
English. An exhaustive classification of A1–B2 words is outside the filter's
scope.

For the initial list, the project collected the word cells from both official
downloads below. It removed comment lines and duplicate forms, converted the
letters to lowercase and sorted the result.

- [NGSL 1.2 lemmatized for research](https://www.newgeneralservicelist.com/s/NGSL_12_lemmatized_for_research.csv).
- [NGSL supplementary words](https://www.newgeneralservicelist.com/s/SUP_lemmatized.csv).

The NGSL project publishes the source material under Creative Commons
Attribution-ShareAlike 4.0. The derived common-word list retains that license
and attribution. The steps above changed its format. Reports use the stored
list and skip downloads. [NGSL project](https://www.newgeneralservicelist.com/new-general-service-list),
[Creative Commons Attribution-ShareAlike 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

## Project vocabulary

`assets/project-words.txt` starts with a fixed list of hardware names, tools,
formats and ordinary project terms, including `amiga`, `git`, `workflow` and
`milestone`. The project used its initial word-frequency analysis to select
terms from the repository's programming, hardware, software and name
vocabularies under `tools/verification/checks/english_words/`. Later spelling-dictionary changes
affect this report's filter only when the report's list is updated.

To decide whether a word belongs in the project vocabulary, read its uses
and consider what it names. Frequent use in instructions alone gives
insufficient evidence. `closure` and `explicit` remain candidates for review
because a reader may need the actual action or condition spelled out.

During a report, suggest additions and leave both lists unchanged. Say
whether each suggestion is ordinary English or an established project term and
explain it using the examples you read. Update the appropriate list when the
user selects that maintenance. Continue to assess each word in context, even
when repetition makes it familiar.
