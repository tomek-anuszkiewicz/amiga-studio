"""Read one bounded-size HTTP response in a process the caller can stop."""

import http.client
import json
import sys
from urllib.error import HTTPError
from urllib.request import Request, urlopen


RESPONSE_LIMIT = 24 * 1024 * 1024


def main():
    url, accept = json.load(sys.stdin)
    headers = {"Accept": accept, "User-Agent": "Amiga-development-dependency-check"}
    data = b""
    try:
        with urlopen(Request(url, headers=headers)) as response:
            data = response.read(RESPONSE_LIMIT + 1)
    except HTTPError as error:
        status = f"HTTP {error.code}".encode("ascii")
    except (http.client.HTTPException, OSError, ValueError):
        status = b"unreadable"
    else:
        if len(data) > RESPONSE_LIMIT:
            status, data = b"oversize", b""
        else:
            status = b"ok"
    sys.stdout.buffer.write(status + b"\n")
    sys.stdout.buffer.write(data)


if __name__ == "__main__":
    main()
