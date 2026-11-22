"""Word rankings and source samples from Markdown with fenced blocks."""

from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rank_words import build_report


class FencedBlockRankingTests(unittest.TestCase):
    def report_for(self, sources):
        root = Path(self.enterContext(tempfile.TemporaryDirectory())).resolve()
        for name, source in sources.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(source.encode("utf-8"))

        report = build_report(root, [(name, "docs") for name in sources],
                              {"common": {"the"}, "project": {"amiga"}}, limit=20)

        self.assertTrue(report["complete"], report["issues"])
        for name, source in sources.items():
            self.assertEqual((root / name).read_bytes(), source.encode("utf-8"))
        return report

    def test_ranks_backtick_bodies_with_filtered_words_and_original_source_lines(self):
        body = "the amiga spectral spectral [opal](vapor) docs/amber.txt packed_word parsedToken"
        report = self.report_for({
            "docs/first.md": (
                "---\n"
                "title: metaphoric\n"
                "---\n"
                "Cadence\n"
                "```synthetic\n"
                f"{body}\n"
                "```\n"
                "    cadence\n"
                "~~~alternate\n"
                "velvety spectral\n"
                "~~~\n"
            ),
            "docs/second.md": (
                "Rhythmic\n"
                "````imaginary\n"
                "```\n"
                "spectral cadence\n"
                "```\n"
                "`````\n"
                "quixotic\n"
            ),
        })

        self.assertEqual(report["document_count"], 2)
        self.assertEqual(report["token_count"], 11)
        self.assertEqual(report["candidate_count"], 9)
        self.assertEqual(report["ranking"], [
            {"word": "cadence", "count": 3, "document_count": 2, "samples": [
                {"path": "docs/first.md", "line": 4, "context": "Cadence"},
                {"path": "docs/second.md", "line": 4, "context": "spectral cadence"},
            ]},
            {"word": "spectral", "count": 3, "document_count": 2, "samples": [
                {"path": "docs/first.md", "line": 6, "context": body},
                {"path": "docs/second.md", "line": 4, "context": "spectral cadence"},
            ]},
            {"word": "opal", "count": 1, "document_count": 1, "samples": [
                {"path": "docs/first.md", "line": 6, "context": body},
            ]},
            {"word": "quixotic", "count": 1, "document_count": 1, "samples": [
                {"path": "docs/second.md", "line": 7, "context": "quixotic"},
            ]},
            {"word": "rhythmic", "count": 1, "document_count": 1, "samples": [
                {"path": "docs/second.md", "line": 1, "context": "Rhythmic"},
            ]},
        ])

    def test_backticks_in_fences_leave_neighboring_prose_and_fences_visible(self):
        report = self.report_for({
            "docs/boundaries.md": (
                "`ornate` timbre\n"
                "```\n"
                "`cadence\n"
                "```\n"
                "rhythmic `chimerical\n"
                "loquacious` quixotic\n"
                "```\n"
                "spectral`\n"
                "```\n"
                "`opal` amber\n"
            ),
        })

        self.assertEqual({row["word"]: row["samples"][0]["line"] for row in report["ranking"]}, {
            "ornate": 1, "timbre": 1, "cadence": 3, "rhythmic": 5,
            "quixotic": 6, "spectral": 8, "opal": 10, "amber": 10,
        })

    def test_multiline_links_rank_labels_and_prose_with_original_source_lines(self):
        report = self.report_for({
            "docs/first.md": (
                "Cadence [Visible label](\n"
                "quartzdestination(nested)\n"
                '"hidden )\n'
                'secrettitle"\n'
                ") cadence\n"
                "[Opal][referencekey]\n"
                '[referencekey]: singleline "referencetitle"\n'
            ),
            "docs/second.md": (
                "[Cadence](<hiddenpath>\n"
                "  (parenthetical\n"
                "  hiddentitle)) spectral\n"
                "[Opal](singleline 'hidden title')\n"
            ),
        })

        self.assertEqual(report["document_count"], 2)
        self.assertEqual(report["token_count"], 8)
        self.assertEqual(report["candidate_count"], 8)
        self.assertEqual(report["ranking"], [
            {"word": "cadence", "count": 3, "document_count": 2, "samples": [
                {"path": "docs/first.md", "line": 1, "context": "Cadence [Visible label]("},
                {"path": "docs/second.md", "line": 1, "context": "[Cadence](<hiddenpath>"},
            ]},
            {"word": "opal", "count": 2, "document_count": 2, "samples": [
                {"path": "docs/first.md", "line": 6, "context": "[Opal][referencekey]"},
                {"path": "docs/second.md", "line": 4, "context": "[Opal](singleline 'hidden title')"},
            ]},
            {"word": "label", "count": 1, "document_count": 1, "samples": [
                {"path": "docs/first.md", "line": 1, "context": "Cadence [Visible label]("},
            ]},
            {"word": "spectral", "count": 1, "document_count": 1, "samples": [
                {"path": "docs/second.md", "line": 3, "context": "hiddentitle)) spectral"},
            ]},
            {"word": "visible", "count": 1, "document_count": 1, "samples": [
                {"path": "docs/first.md", "line": 1, "context": "Cadence [Visible label]("},
            ]},
        ])

    def test_multiline_link_in_backtick_body_ranks_label_and_neighboring_words(self):
        report = self.report_for({
            "docs/example.md": (
                "Before\n"
                "```markdown\n"
                "[Opal](\n"
                "quartz\n"
                "'suppress )\n"
                "private'\n"
                ") spectral\n"
                "```\n"
                "Cadence\n"
            ),
        })

        self.assertEqual({row["word"]: row["samples"][0]["line"] for row in report["ranking"]}, {
            "before": 1, "opal": 3, "spectral": 7, "cadence": 9,
        })

    def test_unfinished_link_keeps_prose_in_the_same_and_following_paragraphs(self):
        report = self.report_for({
            "docs/unfinished.md": (
                "[Opal](\n"
                "\n"
                "Cadence)\n"
                "[Visible](\n"
                "ordinary prose)\n"
                "Quixotic\n"
            ),
        })

        self.assertEqual({row["word"]: row["samples"][0]["line"] for row in report["ranking"]}, {
            "opal": 1, "cadence": 3, "visible": 4, "ordinary": 5, "prose": 5, "quixotic": 6,
        })

    def test_quote_markers_and_four_spaces_inside_a_fence_stay_literal(self):
        for marker in ("> ```", "    ```"):
            with self.subTest(marker=marker):
                body = "`brontolith cadence`"
                report = self.report_for({
                    "docs/example.md": f"```text\n{marker}\n{body}\n```\nSpectral\n",
                })

                self.assertEqual(report["token_count"], 3)
                self.assertEqual(report["ranking"], [
                    {"word": word, "count": 1, "document_count": 1, "samples": [
                        {"path": "docs/example.md", "line": line, "context": context},
                    ]}
                    for word, line, context in (
                        ("brontolith", 3, body), ("cadence", 3, body), ("spectral", 5, "Spectral")
                    )
                ])

    def test_fences_inside_quote_and_list_use_their_container_boundaries(self):
        report = self.report_for({
            "docs/nested.md": (
                "> - ```remarkable\n"
                ">   `brontolith opal`\n"
                ">   ```\n"
                ">   Cadence\n"
                ">\n"
                "> - ~~~\n"
                ">   secrettitle\n"
                ">   ~~~\n"
                "Spectral\n"
                "\n"
                "- ```remarkable\n"
                "  loquacious\n"
                "- `hidden phrase` quixotic\n"
            ),
        })

        self.assertEqual(report["token_count"], 6)
        self.assertEqual({row["word"]: row["samples"][0]["line"] for row in report["ranking"]}, {
            "brontolith": 2, "opal": 2, "cadence": 4, "spectral": 9,
            "loquacious": 12, "quixotic": 13,
        })

    def test_wrapped_visible_label_keeps_both_source_lines_and_excludes_destination(self):
        report = self.report_for({
            "docs/wrapped.md": "[Opal\ncadence](\nquartzdestination\n) spectral\n",
        })

        self.assertEqual(report["token_count"], 3)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/wrapped.md", "line": line, "context": context},
            ]}
            for word, line, context in (
                ("cadence", 2, "cadence]("), ("opal", 1, "[Opal"), ("spectral", 4, ") spectral")
            )
        ])

    def test_multiline_reference_title_is_excluded_and_following_prose_keeps_its_line(self):
        report = self.report_for({
            "docs/reference.md": (
                "[Opal][ref]\n"
                "\n"
                "[ref]: quartzdestination\n"
                '  "secrettitle\n'
                '  hiddentitle"\n'
                "\n"
                "spectral\n"
            ),
        })

        self.assertEqual(report["token_count"], 2)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/reference.md", "line": line, "context": context},
            ]}
            for word, line, context in (("opal", 1, "[Opal][ref]"), ("spectral", 7, "spectral"))
        ])

    def test_tab_in_list_fence_preserves_the_original_sample_column(self):
        line = "\tfolder/" + "x" * 150 + ' [Spectral](spectral "secrettitle")'
        report = self.report_for({
            "docs/tabs.md": f"- ```markdown\n{line}\n  ```",
        })

        self.assertEqual(report["token_count"], 1)
        self.assertEqual(report["ranking"], [{
            "word": "spectral", "count": 1, "document_count": 1, "samples": [{
                "path": "docs/tabs.md", "line": 2,
                "context": "\u2026" + "x" * 68 + ' [Spectral](spectral "secrettitle")',
            }],
        }])

    def test_obsidian_aliases_and_comments_keep_only_visible_words(self):
        report = self.report_for({
            "docs/obsidian.md": (
                "---\n"
                "title: hiddentitle\n"
                "...\n"
                "[[Amber note|Opal]] ![[private/image.png|Cadence]] [[Timbre]]\n"
                "<!-- concealed\n"
                "\n"
                "```\n"
                "indistinct\n"
                "```\n"
                "--> Spectral\n"
                "Rhythmic <!-- shadowed\n"
                "obscured --> Quixotic\n"
            ),
        })

        self.assertEqual(report["token_count"], 6)
        self.assertEqual({row["word"]: row["samples"][0]["line"] for row in report["ranking"]}, {
            "opal": 4, "cadence": 4, "timbre": 4, "spectral": 10,
            "rhythmic": 11, "quixotic": 12,
        })

    def test_inline_comment_can_cross_a_blank_line_and_contain_a_fence_marker(self):
        report = self.report_for({
            "docs/comment.md": (
                "Rhythmic <!-- concealed\n"
                "\n"
                "```\n"
                "indistinct\n"
                "--> Quixotic\n"
                "`obscured phrase` Spectral\n"
            ),
        })

        self.assertEqual(report["token_count"], 3)
        self.assertEqual({row["word"]: row["samples"][0]["line"] for row in report["ranking"]}, {
            "rhythmic": 1, "quixotic": 5, "spectral": 6,
        })
