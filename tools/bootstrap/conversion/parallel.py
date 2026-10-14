"""Bounded conversion workers with isolated clients and coordinator-owned output."""

from concurrent.futures import ThreadPoolExecutor
from queue import Queue
from threading import Lock

from .client import CodexClient


def convert_parallel(config, *, stage, operation, jobs, convert, accept,
                     client_factory=CodexClient):
    concurrency = config["llm"]["concurrency"]
    if type(concurrency) is not int or concurrency < 1:
        raise ValueError("llm.concurrency must be a positive integer")
    pending = iter(jobs)
    lock = Lock()
    results = Queue(maxsize=concurrency)
    errors = []
    stopped = False

    def fail(error):
        nonlocal stopped
        with lock:
            stopped = True
            if not errors:
                errors.append(error)

    def take():
        with lock:
            if stopped:
                return None
            return next(pending, None)

    def worker():
        try:
            job = take()
            if job is None:
                return
            with client_factory(config, stage=stage, operation=operation) as client:
                while job is not None:
                    with lock:
                        if stopped:
                            break
                    results.put((job, convert(client, job)))
                    job = take()
        except BaseException as error:
            fail(error)
        finally:
            results.put(None)

    sink_failed = False
    with ThreadPoolExecutor(max_workers=concurrency) as executor:
        for _ in range(concurrency):
            executor.submit(worker)
        remaining = concurrency
        while remaining:
            try:
                result = results.get()
            except BaseException as error:
                # Drain workers before propagating interruption, avoiding blocked puts.
                sink_failed = True
                fail(error)
                continue
            if result is None:
                remaining -= 1
            elif not sink_failed:
                try:
                    accept(*result)
                except BaseException as error:
                    sink_failed = True
                    fail(error)
    if errors:
        raise errors[0]
