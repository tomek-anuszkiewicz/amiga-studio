"""Full-chapter Markdown/KaTeX preparation and immutable HTML publication."""

from copy import deepcopy
import html
from pathlib import Path
import shutil
import time

from markdown_it import MarkdownIt
from mdit_py_plugins.dollarmath import dollarmath_plugin
from playwright.sync_api import sync_playwright

from .model import asset_names, digest, element_markdown, elements, heading_text, json_bytes, toc_entries, write_json
from .markdown import callout_plugin

HERE = Path(__file__).resolve().parent
KATEX = HERE.parent / "node_modules" / "katex" / "dist"


def math_node(source, options):
    tag = "div" if options["display_mode"] else "span"
    display = "true" if options["display_mode"] else "false"
    return f'<{tag} data-math="{html.escape(source, quote=True)}" data-display="{display}"></{tag}>'


class Renderer:
    """Prepare complete HTML on the rendering thread; geometry belongs to the viewer."""

    def __init__(self, width=1200, channel="msedge"):
        if width < 320:
            raise ValueError("Preparation viewport width must be at least 320 pixels")
        if not (KATEX / "katex.min.js").exists():
            raise FileNotFoundError("Install local math assets with npm install in book-to-markdown")
        self.style = (HERE / "document.css").read_text(encoding="utf-8")
        self.settings = {"width": width, "channel": channel, "style": digest(self.style.encode()),
                         "renderer": digest(Path(__file__).read_bytes()), "katex": "0.16.22",
                         "line_breaks": digest((HERE / "markdown.py").read_bytes()),
                         "element_markup": digest((HERE / "model.py").read_bytes()),
                         "presentation": "html"}
        self.md = MarkdownIt("commonmark", {"html": True}).enable("table").enable("strikethrough")
        self.md.use(dollarmath_plugin, allow_labels=False, renderer=math_node)
        self.md.use(callout_plugin)
        self.playwright = sync_playwright().start()
        self.browser = self.playwright.chromium.launch(channel=channel or None)
        self.page = self.browser.new_page(viewport={"width": width, "height": 900})
        # Preparation uses only the snapshotted local resources.
        self.page.route("http://**/*", lambda route: route.abort())
        self.page.route("https://**/*", lambda route: route.abort())

    def close(self):
        self.browser.close()
        self.playwright.stop()

    def chapter_key(self, entries, asset_hashes):
        rendered = [{k: v for k, v in e.items() if k not in {"pdf_regions", "render_regions"}}
                    for e in entries]
        return digest(json_bytes({"elements": rendered, "settings": self.settings,
                                  "assets": {name: asset_hashes[name]
                                             for e in entries for name in asset_names(e)}}))

    def render_chapter(self, entries, directory, key):
        bodies = []
        for entry in entries:
            # Keep hidden content available for review without including it in exports.
            body = entry.get("html") if "html" in entry else self.md.render(
                element_markdown({**entry, "visible": True}))
            bodies.append(f'<div data-element="{entry["id"]}" id="element-{entry["id"]}">{body}</div>')
        # Navigate a trusted shell, then insert document content as inert DOM.
        shell = directory / "preparation.html"
        shell.write_text(
            '<!doctype html><html><head><meta charset="utf-8">'
            f'<link rel="stylesheet" href="{(KATEX / "katex.min.css").as_uri()}">'
            f'<style>{self.style}</style></head><body><div class="document"></div>'
            f'<script src="{(KATEX / "katex.min.js").as_uri()}"></script></body></html>',
            encoding="utf-8")
        try:
            self.page.goto(shell.as_uri(), wait_until="load")
            prepared = self.page.evaluate("""async markup => {
              const template = document.createElement('template');
              template.innerHTML = markup;
              // Raw HTML tables/details remain; executable content is not book content.
              template.content.querySelectorAll('script,style,link,iframe,object,embed,base,meta').forEach(n => n.remove());
              for (const n of template.content.querySelectorAll('*')) {
                for (const a of [...n.attributes]) {
                  if (/^on/i.test(a.name) || ['srcdoc','srcset'].includes(a.name.toLowerCase())) n.removeAttribute(a.name);
                  if (['src','href','xlink:href','action','formaction'].includes(a.name.toLowerCase()) &&
                      /^\\s*(javascript|vbscript|data):/i.test(a.value)) n.removeAttribute(a.name);
                }
              }
              const root = document.querySelector('.document');
              root.append(template.content);
              for (const n of root.querySelectorAll('[data-math]')) {
                katex.render(n.dataset.math,n,{displayMode:n.dataset.display==='true',throwOnError:true});
                n.removeAttribute('data-math'); n.removeAttribute('data-display');
              }
              await Promise.all([...root.querySelectorAll('img')].map(i => i.decode()));
              await document.fonts.ready;
              return root.innerHTML;
            }""", "".join(bodies))
        finally:
            shell.unlink(missing_ok=True)
        filename = f"{key}.html"
        (directory / filename).write_text(prepared, encoding="utf-8")
        return {"key": key, "document": filename}

    def render(self, working, book, assets, fingerprint, previous=None):
        started = time.perf_counter()
        asset_hashes = {k: digest(v) for k, v in assets.items()}
        revision = digest(json_bytes({"input": fingerprint, "settings": self.settings}))[:24]
        directory = working / "renders" / revision
        directory.mkdir(parents=True, exist_ok=True)
        for relative, data in assets.items():
            target = directory / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        (directory / "document.css").write_text(self.style, encoding="utf-8")
        snapshot = deepcopy(book)
        for element in elements(snapshot):
            element.pop("render_regions", None)
        snapshot.update({"revision": revision, "input_fingerprint": fingerprint,
                         "asset_hashes": asset_hashes, "settings": self.settings, "render": {"chapters": []}})
        entries = toc_entries(book)
        toc_body = '<h1>Table of Contents</h1>' + "".join(
            f'<p class="toc-level-{e["heading_level"]}"><a href="#element-{e["id"]}" '
            f'data-target="{e["id"]}">{self.md.renderInline(" ".join(heading_text(e).split()))}</a></p>'
            for _, e in entries)
        documents = [("toc", [{"id": "generated-toc", "html": toc_body}])]
        documents += [(c["id"], c["elements"]) for c in book["chapters"]]
        previous_layouts = {c["id"]: c for c in previous.get("render", {}).get("chapters", [])} if previous else {}
        prepared = 0
        for identifier, chapter_entries in documents:
            key = self.chapter_key(chapter_entries, asset_hashes)
            old = previous_layouts.get(identifier)
            source = working / old["document"] if old and "document" in old else None
            if old and old["key"] == key and source and source.exists():
                layout = deepcopy(old)
                target = directory / source.name
                if source != target:
                    shutil.copy2(source, target)
                layout["document"] = target.name
            else:
                layout = self.render_chapter(chapter_entries, directory, key)
                prepared += 1
            layout.update({"id": identifier, "document": f"renders/{revision}/{layout['document']}"})
            snapshot["render"]["chapters"].append(layout)
        snapshot["metrics"] = {"prepared_chapters": prepared, "seconds": round(time.perf_counter() - started, 3)}
        write_json(directory / "rendered.json", snapshot)
        return snapshot
