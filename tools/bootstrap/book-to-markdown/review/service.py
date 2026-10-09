"""Coalescing render queue, local HTTP/SSE and complete-bundle acceptance."""

from copy import deepcopy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import mimetypes
from pathlib import Path
import shutil
import threading
from urllib.parse import unquote, urlparse
import uuid

from PIL import Image

from .model import (HEADING_TYPES, changes, confined, elements, export_markdown, json_bytes,
                    load_input, set_heading_level, write_json)
from .render import HERE, Renderer


def read_snapshot(directory):
    path = directory / "rendered.json"
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else None


def render_accepted(root, renderer):
    """Update presentation settings from Accepted's own authored model."""
    accepted = root / "accepted"
    previous = read_snapshot(accepted)
    if not previous or previous["settings"] == renderer.settings:
        return previous
    shared_cache = root / "working" / "renders" / ".cache"
    if shared_cache.exists():
        shutil.copytree(shared_cache, accepted / "renders" / ".cache", dirs_exist_ok=True)
    book, assets, fingerprint = load_input(accepted)
    snapshot = renderer.render(accepted, book, assets, fingerprint, previous)
    export_markdown(book, accepted)
    write_json(accepted / "rendered.json", snapshot)
    shutil.copytree(accepted / "renders" / ".cache", shared_cache, dirs_exist_ok=True)
    print(f"[accepted presentation] {snapshot['metrics']}", flush=True)
    return snapshot


def replace_accepted(root, snapshot):
    """Prepare before touching Accepted; roll back a failed final rename."""
    prepared = root / f".accepted-prepare-{uuid.uuid4().hex}"
    backup = root / ".accepted-backup"
    accepted = root / "accepted"
    working = root / "working"
    prepared.mkdir()
    try:
        book = {k: deepcopy(snapshot[k]) for k in ("chapters", "toc")}
        for e in elements(book):
            e.pop("render_regions", None)
        write_json(prepared / "book.json", book)
        write_json(prepared / "rendered.json", snapshot)
        export_markdown(book, prepared)
        revision = snapshot["revision"]
        shutil.copytree(working / "renders" / revision, prepared / "renders" / revision)
        source_assets = working / "renders" / revision / "assets"
        if source_assets.exists():
            shutil.copytree(source_assets, prepared / "assets")
        # External edits during preparation must not be implicitly accepted.
        if load_input(working)[2] != snapshot["input_fingerprint"]:
            raise ValueError("Working changed during acceptance; wait for the next render")
        if backup.exists():
            raise RuntimeError("Previous acceptance recovery is required before replacing Accepted")
        if accepted.exists():
            accepted.rename(backup)
        try:
            prepared.rename(accepted)
        except BaseException:
            if backup.exists():
                backup.rename(accepted)
            raise
    finally:
        if prepared.exists():
            shutil.rmtree(prepared)
    if backup.exists():
        try:
            shutil.rmtree(backup)
        except OSError as error:
            # The replacement is already committed; report cleanup separately.
            print(f"[accepted backup cleanup] {error}", flush=True)


class ReviewState:
    def __init__(self, root, width=1200, channel="msedge"):
        self.root = root
        self.width, self.channel = width, channel
        self.lock = threading.RLock()
        self.condition = threading.Condition(self.lock)
        self.stopping = threading.Event()
        self.working = read_snapshot(root / "working")
        backup, accepted = root / ".accepted-backup", root / "accepted"
        if backup.exists():
            if accepted.exists():
                shutil.rmtree(backup)
            else:
                backup.rename(accepted)
        self.accepted = read_snapshot(accepted)
        self.error, self.pending, self.sequence = None, True, 0
        self.failed_fingerprint = None
        self.pdf_pages = []
        for path in sorted((root / "pdf" / "pages").glob("page_*.png")):
            with Image.open(path) as image:
                self.pdf_pages.append({"page": int(path.stem.split("_")[1]), "image": f"pdf/pages/{path.name}",
                                       "width": image.width, "height": image.height})

    def announce(self):
        self.sequence += 1
        self.condition.notify_all()

    def status(self):
        with self.lock:
            try:
                authored = load_input(self.root / "working")[2]
                current = bool(self.working and self.working["input_fingerprint"] == authored)
                input_error = None
            except Exception as error:
                authored, current, input_error = None, False, str(error)
            diff = changes(self.accepted, self.working) if self.accepted and self.working else []
            return {"sequence": self.sequence, "working": self.working, "accepted": self.accepted,
                    "authored_revision": authored, "pending": self.pending or not current,
                    "error": input_error or self.error,
                    "can_accept": current and not self.pending and not self.error,
                    "changes": diff, "pdf_pages": self.pdf_pages, "heading_types": sorted(HEADING_TYPES)}

    def render_loop(self):
        renderer = None
        try:
            renderer = Renderer(self.width, self.channel)
            refreshed_accepted = render_accepted(self.root, renderer)
            while not self.stopping.is_set():
                try:
                    book, assets, fingerprint = load_input(self.root / "working")
                    with self.lock:
                        previous = self.working
                        needs_render = (not previous or previous["input_fingerprint"] != fingerprint
                                        or previous["settings"] != renderer.settings)
                        if self.failed_fingerprint == fingerprint or not needs_render:
                            if not needs_render and (self.pending or self.error):
                                self.pending = False
                                self.error = None
                                self.failed_fingerprint = None
                                if refreshed_accepted:
                                    self.accepted = refreshed_accepted
                                self.announce()
                            continue_render = False
                        else:
                            self.pending, self.error = True, None
                            self.announce()
                            continue_render = True
                    if continue_render:
                        snapshot = renderer.render(self.root / "working", book, assets, fingerprint, previous)
                        with self.lock:
                            write_json(self.root / "working" / "rendered.json", snapshot)
                            self.working = snapshot
                            if refreshed_accepted:
                                self.accepted = refreshed_accepted
                            self.pending = load_input(self.root / "working")[2] != fingerprint
                            if self.accepted is None and not self.pending:
                                replace_accepted(self.root, snapshot)
                                self.accepted = read_snapshot(self.root / "accepted")
                                export_markdown(book, self.root / "working")
                            self.failed_fingerprint, self.error = None, None
                            self.announce()
                            print(f"[render] {snapshot['revision']}: {snapshot['metrics']}", flush=True)
                except Exception as error:
                    with self.lock:
                        message = str(error)
                        if message != self.error:
                            self.error, self.pending = message, False
                            self.announce()
                            print(f"[render failed] {message}", flush=True)
                        self.failed_fingerprint = locals().get("fingerprint")
                self.stopping.wait(0.5)
        except Exception as error:
            with self.lock:
                self.error, self.pending = str(error), False
                self.announce()
        finally:
            if renderer:
                renderer.close()

    def mutate(self, expected_revision, update):
        with self.lock:
            book, _, fingerprint = load_input(self.root / "working")
            if fingerprint != expected_revision:
                raise ValueError("Stale edit command; reload the latest authored revision")
            update(book)
            if load_input(self.root / "working")[2] != fingerprint:
                raise ValueError("Working changed before save; retry the command")
            write_json(self.root / "working" / "book.json", book)
            self.pending, self.error, self.failed_fingerprint = True, None, None
            self.announce()

    def heading(self, command):
        def update(book):
            element = next((e for e in elements(book) if e["id"] == command["id"]), None)
            if element is None:
                raise ValueError("Heading no longer exists")
            set_heading_level(element, command["level"])
        self.mutate(command["expected_revision"], update)

    def toc(self, command):
        if type(command["enabled"]) is not bool or type(command["depth"]) is not int or command["depth"] not in (1, 2, 3):
            raise ValueError("TOC requires a boolean enabled flag and depth 1, 2 or 3")
        self.mutate(command["expected_revision"], lambda book: book.update(
            toc={"enabled": command["enabled"], "depth": command["depth"]}))

    def accept(self, command):
        with self.lock:
            status = self.status()
            if not status["can_accept"] or self.working["revision"] != command["revision"]:
                raise ValueError("Acceptance requires the newest complete Working revision")
            replace_accepted(self.root, self.working)
            self.accepted = read_snapshot(self.root / "accepted")
            self.announce()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    @property
    def state(self):
        return self.server.state

    def send_bytes(self, data, content_type, status=200):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        path = unquote(urlparse(self.path).path)
        if path == "/api/status":
            self.send_bytes(json_bytes(self.state.status()), "application/json")
        elif path == "/api/events":
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Connection", "close")
            self.end_headers()
            sequence = -1
            try:
                while not self.state.stopping.is_set():
                    with self.state.condition:
                        if self.state.sequence == sequence:
                            self.state.condition.wait(15)
                        sequence = self.state.sequence
                    self.wfile.write(f"data: {sequence}\n\n".encode())
                    self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                pass
            self.close_connection = True
        else:
            try:
                with self.state.lock:
                    if path in ("/", "/viewer.js", "/viewer.css"):
                        file = HERE / ("viewer.html" if path == "/" else path[1:])
                    elif path.startswith("/files/"):
                        relative = path.removeprefix("/files/")
                        if relative.split("/", 1)[0] not in {"working", "accepted", "pdf"}:
                            raise FileNotFoundError()
                        file = confined(self.state.root, relative)
                        if file.suffix not in {".png", ".md", ".json"}:
                            raise FileNotFoundError()
                    else:
                        raise FileNotFoundError()
                    data = file.read_bytes()
                self.send_bytes(data, mimetypes.guess_type(file.name)[0] or "application/octet-stream")
            except (ValueError, OSError):
                self.send_bytes(b"Not found", "text/plain", 404)

    def do_POST(self):
        try:
            origin = self.headers.get("Origin")
            if self.headers.get("Sec-Fetch-Site") == "cross-site" or (origin and origin != f"http://{self.headers.get('Host')}"):
                raise ValueError("Cross-origin commands are not allowed")
            if self.headers.get("Content-Type", "").split(";")[0] != "application/json":
                raise ValueError("Expected application/json")
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length < 65536:
                raise ValueError("Invalid command length")
            command = json.loads(self.rfile.read(length))
            if self.path == "/api/heading":
                self.state.heading(command)
            elif self.path == "/api/toc":
                self.state.toc(command)
            elif self.path == "/api/accept":
                self.state.accept(command)
            else:
                self.send_bytes(b"Not found", "text/plain", 404)
                return
            self.send_bytes(json_bytes({"ok": True}), "application/json")
        except Exception as error:
            self.send_bytes(json_bytes({"error": str(error)}), "application/json", 409)


def serve(root, port, width, channel):
    state = ReviewState(root, width, channel)
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    server.daemon_threads = True
    server.state = state
    worker = threading.Thread(target=state.render_loop, name="book-renderer", daemon=True)
    worker.start()
    print(f"Book review: http://127.0.0.1:{server.server_port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        state.stopping.set()
        with state.condition:
            state.condition.notify_all()
        server.server_close()
        worker.join(timeout=10)
