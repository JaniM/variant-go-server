#!/usr/bin/env python

import urllib.parse
import http.server
import socketserver
import re
from pathlib import Path

HOST = ("0.0.0.0", 8080)
pattern = re.compile(".png|.jpg|.jpeg|.js|.css|.ico|.gif|.svg", re.IGNORECASE)


class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        url_parts = urllib.parse.urlparse(self.path)
        request_file_path = Path(url_parts.path.strip("/"))

        ext = request_file_path.suffix
        if not request_file_path.is_file() and not pattern.match(ext):
            self.path = "index.html"

        return http.server.SimpleHTTPRequestHandler.do_GET(self)


httpd = socketserver.TCPServer(HOST, Handler)
httpd.serve_forever()
