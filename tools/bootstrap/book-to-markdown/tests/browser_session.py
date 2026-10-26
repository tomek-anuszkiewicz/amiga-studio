"""Share one browser process while isolating every UI test in a fresh context."""

from contextlib import contextmanager

from playwright.sync_api import sync_playwright


_playwright = None
_browser = None
_suite_active = False


def close_browser():
    global _playwright, _browser
    try:
        if _browser is not None:
            _browser.close()
    finally:
        _browser = None
        if _playwright is not None:
            _playwright.stop()
            _playwright = None


@contextmanager
def browser_suite():
    global _suite_active
    _suite_active = True
    try:
        yield
    finally:
        _suite_active = False
        close_browser()


@contextmanager
def browser_page(*, viewport):
    global _playwright, _browser
    try:
        if _browser is None:
            _playwright = sync_playwright().start()
            _browser = _playwright.chromium.launch(channel="msedge")
        context = _browser.new_context(viewport=viewport)
        try:
            yield context.new_page()
        finally:
            context.close()
    finally:
        if not _suite_active:
            close_browser()
