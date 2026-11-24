"""Serve a built GUI distribution for browser tests on loopback."""

from contextlib import contextmanager
from functools import partial
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread
from urllib.parse import urlsplit


class IsolatedFiles(SimpleHTTPRequestHandler):
    def send_head(self):
        # Chromium requests a default favicon for pages without a declared icon.
        path = urlsplit(self.path).path
        if path == "/favicon.ico" and not Path(self.translate_path(path)).is_file():
            self.send_response(HTTPStatus.NO_CONTENT)
            self.end_headers()
            return None
        return super().send_head()

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def log_message(self, *_args):
        pass


@contextmanager
def serve_distribution(directory):
    handler = partial(IsolatedFiles, directory=str(directory.resolve()))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
