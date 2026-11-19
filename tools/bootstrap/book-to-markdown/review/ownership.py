"""Cross-process ownership of one host-backed book variant."""

class BookInUseError(ValueError):
    """A different writer already owns the book."""


class BookWriter:
    def __init__(self, variant):
        self.path = variant / ".book-review-writer"

    def __enter__(self):
        # Both the Windows CLI and the Linux service create the same directory
        # on the shared host filesystem. Never remove another owner's claim.
        try:
            self.path.mkdir()
        except FileExistsError as error:
            raise BookInUseError(f"Book already has a writer: {self.path.parent}") from error
        return self

    def __exit__(self, *_):
        self.path.rmdir()
