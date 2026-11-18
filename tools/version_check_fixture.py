"""Finite owned HTTP fixture; no public listener or external redirect"""
import http.server
import json
import os
from pathlib import Path
import signal
import socket
import sys
import threading
import time

assert len(sys.argv) == 2
RUN = Path(sys.argv[1])
assert RUN.is_absolute() and RUN.resolve() == RUN and RUN.is_dir()
READY = RUN / "server-ready.json"
LOG = RUN / "requests.jsonl"
assert not READY.exists() and not LOG.exists()
LOG.touch(exist_ok=False)
stopping = threading.Event()
lock = threading.Lock()
ordinal = 0


def record(event, **fields):
    with lock, LOG.open("a", encoding="utf-8") as stream:
        stream.write(json.dumps({"event": event, **fields}, sort_keys=True) + "\n")
        stream.flush()


def stop(_number, _frame):
    stopping.set()


class Fixture(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"

    def log_message(self, _format, *_args):
        pass

    def do_GET(self):
        global ordinal
        assert self.client_address[0] == "127.0.0.1"
        with lock:
            ordinal += 1
            number = ordinal
        record("request", ordinal=number, path=self.path)
        if number > 64:
            record("limit", limit="requests")
            stopping.set()
            return
        bodies = {
            "/line-crlf": b"10001\r\nignored",
            "/line-lf": b"10000\nignored",
            "/line-cr": b"900\rignored",
            "/empty-line": b"\n10000",
            "/bom-multiline": b"\xef\xbb\xbf900\r\n10000",
            "/eof": b"",
            "/http-error": b"900",
            "/oversize": b"9" * 4097,
            "/invalid-utf8": b"\xff\n",
            "/nul": b"900\x00\n",
        }
        status = 503 if self.path == "/http-error" else 200
        body = bodies.get(self.path)
        location = None
        if self.path == "/redirect":
            status, body = 302, b""
            location = "http://127.0.0.1:%d/redirect" % self.server.server_port
        elif self.path == "/timeout":
            stopping.wait(12.0)
            body = b"10000\n"
        elif self.path in ["/cancel", "/duplicate", "/hold-locale", "/backup", "/shutdown"]:
            flag = RUN / ("release-" + self.path[1:])
            deadline = time.monotonic() + 12.0
            while not stopping.is_set() and not flag.exists() and time.monotonic() < deadline:
                stopping.wait(0.025)
            body = b"10000\n"
        elif body is None:
            status, body = 404, b"unknown owned fixture"
        if stopping.is_set():
            return
        try:
            self.send_response(status)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Connection", "close")
            if location is not None:
                self.send_header("Location", location)
            self.end_headers()
            self.wfile.write(body)
            self.wfile.flush()
            record("response", ordinal=number, path=self.path, status=status, bytes=len(body))
        except (BrokenPipeError, ConnectionResetError):
            record("peer-closed", ordinal=number, path=self.path)


class Server(http.server.ThreadingHTTPServer):
    daemon_threads = True
    block_on_close = False


previous = {number: signal.signal(number, stop) for number in [signal.SIGINT, signal.SIGTERM]}
unavailable = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server = None
try:
    unavailable.bind(("127.0.0.1", 0))
    server = Server(("127.0.0.1", 0), Fixture)
    server.timeout = 0.25
    ready = {"pid": os.getpid(), "pgid": os.getpgrp(), "host": "127.0.0.1", "port": server.server_port, "unavailable_port": unavailable.getsockname()[1], "request_log": str(LOG), "deadline_seconds": 180, "request_limit": 64}
    temporary_ready = RUN / "server-ready.json.tmp"
    with temporary_ready.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(ready, indent=2) + "\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary_ready, READY)
    print(json.dumps({"ready": str(READY), "host": ready["host"], "port": ready["port"]}), flush=True)
    deadline = time.monotonic() + 180.0
    while not stopping.is_set() and time.monotonic() < deadline:
        server.handle_request()
    record("stopped", requests=ordinal)
finally:
    stopping.set()
    if server is not None:
        server.server_close()
    unavailable.close()
    for number, handler in previous.items():
        signal.signal(number, handler)
