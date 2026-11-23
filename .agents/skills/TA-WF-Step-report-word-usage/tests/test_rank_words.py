"""Word rankings, Markdown source samples and Git file selection."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
sys.path.insert(0, str(SCRIPTS))
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

    def test_named_references_in_labels_count_visible_text_with_original_samples(self):
        line = r"[Peregrination &alefsym; \&copy; &amp;reg;](quartzdestination 'secrettitle')"
        report = self.report_for({"docs/entities.md": line})

        self.assertEqual(report["token_count"], 3)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/entities.md", "line": 1, "context": line},
            ]}
            for word in ("copy", "peregrination", "reg")
        ])

    def test_numeric_references_join_visible_words_and_preserve_long_line_columns(self):
        tail = ' &alefsym; &#113;uix&#x6F;tic](hiddenpath "secrettitle") Spectral'
        line = "> [folder/" + "x" * 150 + tail
        report = self.report_for({
            "docs/entities.md": f"{line}\n\nLoqu&#97;cious",
        })

        self.assertEqual(report["token_count"], 3)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/entities.md", "line": number, "context": context},
            ]}
            for word, number, context in (
                ("loquacious", 3, "Loqu&#97;cious"),
                ("quixotic", 1, "\u2026" + "x" * 59 + tail),
                ("spectral", 1, "\u2026" + "x" * 13 + tail),
            )
        ])

    def test_escaped_brackets_keep_visible_label_without_admitting_paths(self):
        line = r'[\[Cadence\]](hiddenpath "secrettitle") Spectral'
        report = self.report_for({
            "docs/labels.md": (
                f"{line}\n"
                r"[folder\amber](hiddenpath) [folder\\amber](hiddenpath)"
                "\n[`hidden phrase`](hiddenpath)"
            ),
        })

        self.assertEqual(report["token_count"], 2)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/labels.md", "line": 1, "context": line},
            ]}
            for word in ("cadence", "spectral")
        ])

    def test_backtick_body_keeps_literal_entities_and_inline_code(self):
        line = '`opal cadence` [Peregrination &alefsym; qui&#120;otic](hiddenpath "secrettitle")'
        report = self.report_for({"docs/example.md": f"```markdown\n{line}\n```"})

        self.assertEqual(report["token_count"], 6)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/example.md", "line": 2, "context": line},
            ]}
            for word in ("alefsym", "cadence", "opal", "otic", "peregrination", "qui")
        ])

    def test_image_label_keeps_visible_text_at_its_source_offset(self):
        tail = r' ![\[Cad&#101;nce\]](docs/image.png "secrettitle")'
        line = "folder/" + "x" * 150 + tail
        report = self.report_for({"docs/image.md": line})

        self.assertEqual(report["token_count"], 1)
        self.assertEqual(report["ranking"], [{
            "word": "cadence", "count": 1, "document_count": 1, "samples": [{
                "path": "docs/image.md", "line": 1, "context": "\u2026" + "x" * 65 + tail,
            }],
        }])

    def test_formatted_words_rank_whole_in_prose_and_link_and_image_labels(self):
        lines = [
            'Pere**grin**ation [Qui*xo*tic](hiddenpath "secrettitle")',
            "Lo***qua***cious",
            "Spec**tra**_l_",
            "Spec*tro*sco**py** [__Chim__](hiddenpath)*eri*cal",
            '![Calli*gra*phy](hiddenimage "secrettitle")',
            "![Rhyth**mic**][nameref]",
            "",
            '[nameref]: hiddenimage "secrettitle"',
        ]
        report = self.report_for({"docs/formatting.md": "\n".join(lines)})

        self.assertEqual(report["token_count"], 8)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/formatting.md", "line": number, "context": lines[number - 1]},
            ]}
            for word, number in (
                ("calligraphy", 5), ("chimerical", 4), ("loquacious", 2),
                ("peregrination", 1), ("quixotic", 1), ("rhythmic", 6),
                ("spectral", 3), ("spectroscopy", 4),
            )
        ])

    def test_hidden_link_newlines_join_letters_but_visible_breaks_keep_words_apart(self):
        lines = [
            "> folder/" + "x" * 150 + " [Qui](",
            '> hiddenpath "secrettitle"',
            "> )xo*tic*",
            "> [Cadence",
            "> Timbre](hiddenpath)",
            "> Spectral  ",
            "> Opal",
            ">",
            "> Amber",
        ]
        report = self.report_for({"docs/multiline.md": "\n".join(lines)})

        self.assertEqual(report["token_count"], 6)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/multiline.md", "line": number, "context": context},
            ]}
            for word, number, context in (
                ("amber", 9, lines[8]), ("cadence", 4, lines[3]), ("opal", 7, lines[6]),
                ("quixotic", 1, "\u2026" + "x" * 68 + " [Qui]("),
                ("spectral", 6, "> Spectral"), ("timbre", 5, lines[4]),
            )
        ])

    def test_html_formatting_and_comments_join_letters_while_layout_preserves_boundaries(self):
        lines = [
            'Per<em class="opaque">e<strong>gri</strong>n</em>ation',
            "Qui<!--hiddencomment",
            "secrettitle-->xo*tic* Rhyth<!--opaque-->mic",
            'Cadence<br>Spectral Opal<img src="hiddenimage">Amber',
            "Timbre![Cal*li*graphy](hiddenimage)Loquacious",
            "",
            '<p>Chi<span title=">">mer</span>ical</p>',
        ]
        report = self.report_for({"docs/html.md": "\n".join(lines)})

        self.assertEqual(report["token_count"], 11)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/html.md", "line": number, "context": lines[number - 1]},
            ]}
            for word, number in (
                ("amber", 4), ("cadence", 4), ("calligraphy", 5), ("chimerical", 7),
                ("loquacious", 5), ("opal", 4), ("peregrination", 1), ("quixotic", 2),
                ("rhythmic", 3), ("spectral", 4), ("timbre", 5),
            )
        ])

    def test_preserves_literal_markers_code_and_path_filtering(self):
        lines = [
            "Pere**gri&#110;ation**",
            r"\*Chi*mer*ical\* &#42;Rhyth*mic*&#42; Qui\*xo\*tic",
            "qui_xo_tic Cadence** **folder**/hiddenpath",
            "Spec` tra `l `hidden phrase` `Opal`",
        ]
        report = self.report_for({"docs/literals.md": "\n".join(lines)})

        self.assertEqual(report["token_count"], 8)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/literals.md", "line": number, "context": lines[number - 1]},
            ]}
            for word, number in (
                ("cadence", 3), ("chimerical", 2), ("opal", 4), ("peregrination", 1),
                ("qui", 2), ("rhythmic", 2), ("spectral", 4), ("tic", 2),
            )
        ])

    def test_strikethrough_ranks_whole_words_while_fenced_examples_stay_literal(self):
        lines = [
            "Cal~~li~~graphy Semi~opa~que",
            "```markdown",
            "Cad**en**ce qui_xo_tic Per~~egrin~~ation",
            "```",
            "~~~",
            "secrettitle",
            "~~~",
        ]
        report = self.report_for({"docs/strikethrough.md": "\n".join(lines)})

        self.assertEqual(report["token_count"], 8)
        self.assertEqual(report["ranking"], [
            {"word": word, "count": 1, "document_count": 1, "samples": [
                {"path": "docs/strikethrough.md", "line": number, "context": lines[number - 1]},
            ]}
            for word, number in (
                ("ation", 3), ("cad", 3), ("calligraphy", 1), ("egrin", 3),
                ("opa", 1), ("per", 3), ("que", 1), ("semi", 1),
            )
        ])


class WordUsageCommandTests(unittest.TestCase):
    def test_cli_ranks_current_git_files_and_lists_unclassified_names(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory())).resolve()
        environment = {key: value for key, value in os.environ.items() if not key.upper().startswith("GIT_")}
        environment.update(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1")
        sources = {
            ".gitignore": b"/docs/ignored.md\n",
            "docs/tracked.md": b"Loquacious\n",
            "docs/new.md": b"Quixotic\n",
            "docs/ignored.md": b"Peregrination\n",
            "docs/ref_src/imported.md": b"Rhythmic\n",
            "unclassified/topics.md": b"Unclassified \xff\n",
        }
        for name, content in sources.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)

        for arguments in (("init", "--quiet"), ("add", "--", ".gitignore", "docs/tracked.md")):
            subprocess.run(["git", *arguments], cwd=root, env=environment, check=True, capture_output=True)
        (root / "docs/tracked.md").write_bytes(b"Cadence Cadence\n")

        output = "report.json"
        result = subprocess.run(
            [sys.executable, str(SCRIPTS / "rank_words.py"), "--root", str(root), "--output", output],
            cwd=root, env=environment, capture_output=True, text=True, encoding="utf-8", check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Unclassified Markdown names in unclassified: 1", result.stdout)
        report = json.loads((root / output).read_text(encoding="utf-8"))
        self.assertTrue(report["complete"], report["issues"])
        self.assertEqual(report["document_count"], 2)
        self.assertEqual(report["scanned_paths"], ["docs/new.md", "docs/tracked.md"])
        self.assertEqual(report["token_count"], 3)
        self.assertEqual(report["candidate_count"], 3)
        self.assertEqual(report["ranking"], [
            {"word": "cadence", "count": 2, "document_count": 1, "samples": [
                {"path": "docs/tracked.md", "line": 1, "context": "Cadence Cadence"},
            ]},
            {"word": "quixotic", "count": 1, "document_count": 1, "samples": [
                {"path": "docs/new.md", "line": 1, "context": "Quixotic"},
            ]},
        ])
        self.assertEqual(report["unclassified_directories"], [
            {"directory": "unclassified", "markdown_count": 1, "examples": ["unclassified/topics.md"]},
        ])
        self.assertEqual(report["discovery_issues"], [])
