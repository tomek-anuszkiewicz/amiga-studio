"""One local server with request-scoped books and independent render queues."""

from collections import Counter
from http.server import ThreadingHTTPServer
import signal
import threading
from urllib.parse import parse_qs, unquote, urlparse

from .model import confined, json_bytes
from .ownership import BookInUseError, BookWriter
from .render import HERE
from .service import Handler, ReviewState


BOOKS = HERE.parents[1] / "books"


class BookLibrary:
    def __init__(self, root=BOOKS, width=1200, channel="msedge", allowed=None):
        self.root = root.resolve()
        self.allowed = None if allowed is None else frozenset(allowed)
        self.stopping = False
        self.width, self.channel = width, channel
        self.lock = threading.RLock()
        self.sessions = {}

    def catalog(self):
        variants = sorted(path.parent.parent for path in self.root.glob("*/*/working/book.json"))
        variants = [path for path in variants if path.resolve().is_relative_to(self.root)
                    and (self.allowed is None or path.relative_to(self.root).as_posix() in self.allowed)]
        counts = Counter(path.parent.name for path in variants)
        return [{"id": path.relative_to(self.root).as_posix(),
                 "name": path.parent.name + (f" ({path.name})" if counts[path.parent.name] > 1 else "")}
                for path in variants]

    def state(self, book_id):
        with self.lock:
            if self.stopping:
                raise ValueError("Book Review is shutting down")
            if book_id not in {book["id"] for book in self.catalog()}:
                raise FileNotFoundError("Unknown or unimported book")
            if book_id not in self.sessions:
                root = confined(self.root, book_id)
                writer = BookWriter(root)
                writer.__enter__()
                try:
                    state = ReviewState(root, self.width, self.channel)
                    worker = threading.Thread(target=state.render_loop, name=f"book-renderer-{book_id}", daemon=True)
                    worker.start()
                except BaseException:
                    writer.__exit__()
                    raise
                self.sessions[book_id] = state, worker, writer
            return self.sessions[book_id][0]

    def status(self):
        with self.lock:
            return {"books": self.catalog(), "reviews": [
                {"id": book_id, "pending": state.pending, "error": state.error}
                for book_id, (state, _, _) in self.sessions.items()]}

    def stop(self):
        with self.lock:
            self.stopping = True
            for state, _, _ in self.sessions.values():
                state.stopping.set()
                with state.condition:
                    if state.codex_stop:
                        state.codex_stop.set()
                    state.condition.notify_all()

    def close(self):
        self.stop()
        with self.lock:
            sessions = list(self.sessions.values())
            self.sessions.clear()
        for state, worker, writer in sessions:
            worker.join()
            if state.codex_worker is not None:
                state.codex_worker.join()
            writer.__exit__()


class LibraryHandler(Handler):
    timeout = 5

    @property
    def state(self):
        return self.review_state

    def select_book(self):
        # Bind every request, including old tabs and in-flight commands, to its book.
        path = unquote(urlparse(self.path).path)
        parts = path.split("/", 4)
        if (len(parts) != 5 or parts[1] != "books"
                or not parts[4].startswith(("api/", "files/"))):
            raise FileNotFoundError("Select a book")
        self.review_state = self.server.library.state("/".join(parts[2:4]))
        self.path = "/" + parts[4]

    def do_GET(self):
        parsed = urlparse(self.path)
        try:
            if parsed.path == "/api/status":
                self.send_bytes(json_bytes(self.server.library.status()), "application/json")
            elif parsed.path == "/":
                catalog = self.server.library.catalog()
                selected = parse_qs(parsed.query).get("book", [""])[0]
                if selected and selected not in {book["id"] for book in catalog}:
                    raise FileNotFoundError("Unknown or unimported book")
                data = json_bytes({"books": catalog, "selected": selected}).decode("utf-8").replace("<", "\\u003c")
                markup = (HERE / "viewer.html").read_text(encoding="utf-8")
                markup = markup.replace('<script src="/viewer.js">',
                                        f'<script id="book-catalog" type="application/json">{data}</script><script src="/viewer.js">')
                self.send_bytes(markup.encode("utf-8"), "text/html; charset=utf-8")
            elif parsed.path in {"/viewer.js", "/viewer.css"} or parsed.path.startswith("/katex/"):
                super().do_GET()
            else:
                self.select_book()
                super().do_GET()
        except BookInUseError as error:
            self.send_bytes(str(error).encode("utf-8"), "text/plain; charset=utf-8", 409)
        except (ValueError, OSError) as error:
            self.send_bytes(str(error).encode("utf-8"), "text/plain; charset=utf-8", 404)

    def do_POST(self):
        try:
            self.select_book()
        except (ValueError, OSError) as error:
            self.close_connection = True
            status = 409 if isinstance(error, BookInUseError) else 404
            self.send_bytes(json_bytes({"error": str(error)}), "application/json", status)
            return
        super().do_POST()


def serve(port, width, channel, *, host="127.0.0.1", root=BOOKS, allowed=None):
    library = BookLibrary(root=root, width=width, channel=channel, allowed=allowed)
    server = ThreadingHTTPServer((host, port), LibraryHandler)
    # Join in-flight HTTP writers before relinquishing the shared book claim.
    server.daemon_threads = False
    server.library = library

    def terminate(*_):
        raise KeyboardInterrupt

    previous = signal.signal(signal.SIGTERM, terminate)
    print(f"Book review: http://{host}:{server.server_port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        signal.signal(signal.SIGTERM, previous)
        library.stop()
        server.server_close()
        library.close()
