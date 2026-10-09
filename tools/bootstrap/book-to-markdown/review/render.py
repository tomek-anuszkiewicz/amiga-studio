"""Persistent full-chapter browser measurement and immutable cached PNG strips."""

from copy import deepcopy
import base64
import html
import json
from pathlib import Path
import shutil
import time

from markdown_it import MarkdownIt
from mdit_py_plugins.dollarmath import dollarmath_plugin
from playwright.sync_api import sync_playwright

from .model import asset_names, digest, element_markdown, json_bytes, toc_entries, write_json

HERE = Path(__file__).resolve().parent
KATEX = HERE.parent / "node_modules" / "katex" / "dist"


def math_node(source, options):
    tag = "div" if options["display_mode"] else "span"
    display = "true" if options["display_mode"] else "false"
    return f'<{tag} data-math="{html.escape(source, quote=True)}" data-display="{display}"></{tag}>'


MEASURE = """() => {
  const rectangle = node => {
    const r = node.getBoundingClientRect();
    return [r.x, r.y, r.right, r.bottom];
  };
  return {height: Math.max(document.body.scrollHeight, document.documentElement.scrollHeight),
    elements: [...document.querySelectorAll('[data-element]')].map(node => ({
      id: node.dataset.element, bbox: rectangle(node),
      links: [...node.querySelectorAll('a[data-target]')].flatMap(a =>
        [...a.getClientRects()].map(r => ({target: a.dataset.target,
          bbox: [r.x,r.y,r.right,r.bottom]})))
    }))};
}"""


class Renderer:
    """Own this object on the rendering thread, never on HTTP request threads."""

    def __init__(self, width=1200, channel="msedge"):
        if width < 320:
            raise ValueError("Render width must be at least 320 pixels")
        if not (KATEX / "katex.min.js").exists():
            raise FileNotFoundError("Install local math assets with npm install in book-to-markdown")
        self.width = width
        self.style = (HERE / "document.css").read_text(encoding="utf-8")
        self.settings = {"width": width, "channel": channel, "style": digest(self.style.encode()),
                         "renderer": digest(Path(__file__).read_bytes()), "katex": "0.16.22"}
        self.md = MarkdownIt("commonmark", {"html": True}).enable("table").enable("strikethrough")
        self.md.use(dollarmath_plugin, allow_labels=False, renderer=math_node)
        self.playwright = sync_playwright().start()
        self.browser = self.playwright.chromium.launch(channel=channel or None)
        self.page = self.browser.new_page(viewport={"width": width, "height": 900}, device_scale_factor=1)
        self.capture = self.page.context.new_cdp_session(self.page)
        # The renderer is local and deterministic: never fetch remote resources.
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

    def render_chapter(self, entries, directory, cache, key, asset_hashes):
        bodies = []
        for entry in entries:
            if not entry.get("visible", True):
                continue
            body = entry.get("html") if "html" in entry else self.md.render(element_markdown(entry))
            bodies.append(f'<div data-element="{entry["id"]}" id="element-{entry["id"]}">{body}</div>')
        markup = (f'<!doctype html><html><head><meta charset="utf-8">'
                  f'<base href="{directory.as_uri()}/">'
                  f'<link rel="stylesheet" href="{(KATEX / "katex.min.css").as_uri()}">'
                  f'<style>{self.style}</style></head><body>{"".join(bodies)}'
                  f'<script src="{(KATEX / "katex.min.js").as_uri()}"></script>'
                  '<script>for(const n of document.querySelectorAll("[data-math]")) '
                  'katex.render(n.dataset.math,n,{displayMode:n.dataset.display==="true",throwOnError:true});'
                  '</script></body></html>')
        # Navigate a complete HTML file instead of patching a live document.
        document = directory / "chapter.html"
        document.write_text(markup, encoding="utf-8")
        errors = []
        def record_error(error):
            errors.append(str(error))
        self.page.on("pageerror", record_error)
        try:
            self.page.goto(document.as_uri(), wait_until="load")
            self.page.evaluate("""async () => {
              await Promise.all([...document.images].map(i => i.decode()));
              await document.fonts.ready;
              for (const pre of document.querySelectorAll('pre')) {
                const available = pre.clientWidth - 32;
                if (pre.scrollWidth > pre.clientWidth) {
                  const code = pre.querySelector('code') || pre;
                  code.style.fontSize = `${parseFloat(getComputedStyle(code).fontSize) * available / (pre.scrollWidth - 32)}px`;
                }
              }
              for (const table of document.querySelectorAll('table')) {
                const available = table.parentElement.clientWidth;
                const width = table.getBoundingClientRect().width;
                if (width > available) table.style.zoom = available / width;
              }
              await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
            }""")
            if errors:
                raise ValueError("Math/document rendering failed: " + "; ".join(errors))
            measured = self.page.evaluate(MEASURE)
        finally:
            self.page.remove_listener("pageerror", record_error)
        boxes = measured["elements"]
        boundaries = [0] + [round(box["bbox"][1]) for box in boxes[1:]] + [round(measured["height"])]
        strips, captures = [], 0
        by_id = {e["id"]: e for e in entries}
        for index, box in enumerate(boxes):
            start, end = boundaries[index:index + 2]
            if end <= start:
                # Empty retained objects still need a stable selectable location.
                strips.append({"id": box["id"], "image": None, "width": self.width, "height": 0,
                               "bbox": [0, 0, self.width, 0], "links": [], "cache_key": None})
                continue
            x0, y0, x1, y1 = box["bbox"]
            local = [x0, max(0, y0 - start), x1, min(end - start, y1 - start)]
            entry = by_id[box["id"]]
            relevant = {k: v for k, v in entry.items() if k not in {"pdf_regions", "render_regions"}}
            assets = {name: asset_hashes[name] for name in asset_names(entry)}
            links = [{"target": link["target"], "bbox": [link["bbox"][0], link["bbox"][1] - start,
                       link["bbox"][2], link["bbox"][3] - start]} for link in box["links"]]
            identity = digest(json_bytes({"input": relevant, "assets": assets, "settings": self.settings,
                                          "height": end - start, "bbox": local, "links": links}))
            image = cache / f"{identity}.png"
            if not image.exists():
                captured = self.capture.send("Page.captureScreenshot", {
                    "format": "png", "captureBeyondViewport": True, "fromSurface": True,
                    "optimizeForSpeed": True,
                    "clip": {"x": 0, "y": start, "width": self.width, "height": end - start, "scale": 1}})
                image.write_bytes(base64.b64decode(captured["data"]))
                captures += 1
            target = directory / image.name
            shutil.copy2(image, target)
            strips.append({"id": box["id"], "image": target.name, "width": self.width,
                           "height": end - start, "bbox": local, "links": links, "cache_key": identity})
        document.unlink()
        return {"key": key, "strips": strips}, captures

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
        cache = working / "renders" / ".cache"
        cache.mkdir(parents=True, exist_ok=True)
        snapshot = deepcopy(book)
        snapshot.update({"revision": revision, "input_fingerprint": fingerprint,
                         "asset_hashes": asset_hashes, "settings": self.settings, "render": {"chapters": []}})
        entries = toc_entries(book)
        toc_body = '<h1>Table of Contents</h1>' + "".join(
            f'<p class="toc-level-{e["heading_level"]}"><a href="#element-{e["id"]}" '
            f'data-target="{e["id"]}">{self.md.renderInline(" ".join(element_markdown(e).lstrip("# ").split()))}</a></p>'
            for _, e in entries)
        documents = [("toc", [{"id": "generated-toc", "html": toc_body}])] if book["toc"]["enabled"] else []
        documents += [(c["id"], c["elements"]) for c in book["chapters"]]
        previous_layouts = {c["id"]: c for c in previous.get("render", {}).get("chapters", [])} if previous else {}
        captures = 0
        for identifier, chapter_entries in documents:
            key = self.chapter_key(chapter_entries, asset_hashes)
            old = previous_layouts.get(identifier)
            if old and old["key"] == key:
                layout = deepcopy(old)
                for strip in layout["strips"]:
                    if strip["image"]:
                        source = working / strip["image"]
                        target = directory / Path(strip["image"]).name
                        if source != target:
                            shutil.copy2(source, target)
            else:
                layout, count = self.render_chapter(chapter_entries, directory, cache, key, asset_hashes)
                captures += count
            layout["id"] = identifier
            for strip in layout["strips"]:
                if strip["image"]:
                    strip["image"] = f"renders/{revision}/{Path(strip['image']).name}"
            snapshot["render"]["chapters"].append(layout)
        rendered_elements = {s["id"]: s for c in snapshot["render"]["chapters"] for s in c["strips"]}
        for chapter in snapshot["chapters"]:
            for element in chapter["elements"]:
                strip = rendered_elements.get(element["id"])
                element["render_regions"] = ([{"image": strip["image"], "bbox": strip["bbox"]}]
                                              if strip and strip["image"] else [])
        snapshot["metrics"] = {"captures": captures, "seconds": round(time.perf_counter() - started, 3)}
        # Caller publishes only after this complete immutable manifest is ready.
        write_json(directory / "rendered.json", snapshot)
        return snapshot
