#!/usr/bin/env python3

import http.server
import os
import socketserver
import sys

port = int(sys.argv[1])
directory = sys.argv[2]
os.chdir(directory)
Handler = http.server.SimpleHTTPRequestHandler
Handler.extensions_map.update({
    '.wasm': 'application/wasm',
})


socketserver.TCPServer.allow_reuse_address = True
# with socketserver.TCPServer(('127.0.0.1', port), Handler) as httpd:
with socketserver.TCPServer(('0.0.0.0', port), Handler) as httpd:
    httpd.allow_reuse_address = True
    print('serving at port', port)
    httpd.serve_forever()
