// Static file server for the browser self-test (and quick manual serving):
// threaded + HTTP/1.1 keep-alive + correct MIME types, so the self-test's
// ~100 shard fetches don't queue behind python http.server's single thread.
//
//   node web/serve.cjs [dir] [port]      (defaults: dist, 8124)

const http = require('http');
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const dir = path.resolve(process.argv[2] || 'dist');
const port = Number(process.argv[3] || 8124);

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript',
  '.mjs': 'text/javascript',
  '.json': 'application/json',
  '.wasm': 'application/wasm',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
};

http
  .createServer((req, res) => {
    const url = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
    const file = path.join(dir, url === '/' ? 'index.html' : url);
    if (!file.startsWith(dir)) {
      res.writeHead(403).end('forbidden');
      return;
    }
    fs.readFile(file, (err, data) => {
      if (err) {
        res.writeHead(404, { 'Content-Type': 'text/html' }).end('not found');
        return;
      }
      const headers = {
        'Content-Type': MIME[path.extname(file)] || 'application/octet-stream',
        'Cache-Control': 'no-store',
      };
      // On-the-fly gzip for large compressible payloads (blobs ~1 MB JSON
      // → ~300 KB). Browsers auto-decompress; the SW caches the gzipped
      // response. Below the threshold, CPU isn't worth it.
      const acceptsGzip = (req.headers['accept-encoding'] || '').includes('gzip');
      if (acceptsGzip && data.length > 100000) {
        zlib.gzip(data, (gzErr, compressed) => {
          if (gzErr) {
            res.writeHead(200, headers);
            res.end(data);
            return;
          }
          res.writeHead(200, { ...headers, 'Content-Encoding': 'gzip' });
          res.end(compressed);
        });
        return;
      }
      res.writeHead(200, headers);
      res.end(data);
    });
  })
  .listen(port, '127.0.0.1', () => {
    process.stdout.write(`serving ${dir} on http://127.0.0.1:${port}\n`);
  });
