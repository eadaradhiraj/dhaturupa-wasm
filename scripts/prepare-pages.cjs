#!/usr/bin/env node
/**
 * Assembles the lightweight Cloudflare Pages deployment shell into dist-pages/
 * Size: ~1.5 MB (static HTML/JS/WASM).
 * Heavy data (~3.2 GB) is served via the Cloudflare Pages Function binding to R2.
 */
const fs = require('fs');
const path = require('path');

const rootDir = path.resolve(__dirname, '..');
const distDir = path.join(rootDir, 'dist');
const outDir = path.join(rootDir, 'dist-pages');

if (!fs.existsSync(distDir)) {
  console.error('Error: dist/ does not exist. Run ./check.sh first.');
  process.exit(1);
}

// Clean and recreate dist-pages
fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

// Files to copy
const files = [
  ['index.html', 'index.html'],
  ['app.js', 'app.js'],
  ['render.js', 'render.js'],
  ['sw.js', 'sw.js'],
  ['selftest.html', 'selftest.html'],
  ['selftest.js', 'selftest.js'],
];

for (const [src, dest] of files) {
  const srcPath = path.join(distDir, src);
  if (fs.existsSync(srcPath)) {
    fs.copyFileSync(srcPath, path.join(outDir, dest));
  } else {
    console.warn(`Warning: missing ${src}`);
  }
}

// Copy pkg/ directory (WASM binaries)
const pkgSrc = path.join(distDir, 'pkg');
const pkgDest = path.join(outDir, 'pkg');
if (fs.existsSync(pkgSrc)) {
  fs.cpSync(pkgSrc, pkgDest, { recursive: true });
}

// Create _headers for Cloudflare Pages
const headersContent = `/*
  X-Content-Type-Options: nosniff
  Access-Control-Allow-Origin: *

/pkg/*.wasm
  Content-Type: application/wasm
  Cache-Control: public, max-age=31536000, immutable

/sw.js
  Cache-Control: no-cache
`;
fs.writeFileSync(path.join(outDir, '_headers'), headersContent, 'utf8');

// Copy functions directory into dist-pages as well for complete self-containment
const functionsSrc = path.join(rootDir, 'functions');
const functionsDest = path.join(outDir, 'functions');
if (fs.existsSync(functionsSrc)) {
  fs.cpSync(functionsSrc, functionsDest, { recursive: true });
}

// Calculate total size
function getDirSize(dir) {
  let total = 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      total += getDirSize(p);
    } else {
      total += fs.statSync(p).size;
    }
  }
  return total;
}

const totalSize = getDirSize(outDir);
const totalMb = (totalSize / (1024 * 1024)).toFixed(2);

console.log(`Cloudflare Pages bundle prepared at: dist-pages/`);
console.log(`Total bundle size: ${totalMb} MB (${totalSize} bytes)`);
console.log(`Ready for deployment: npx wrangler pages deploy dist-pages --project-name dhaturupa`);
