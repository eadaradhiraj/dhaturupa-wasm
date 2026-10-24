# 100% Free Cloudflare Pages Deployment for `dhaturupa-wasm`

Your application is now deployed and running live on **Cloudflare Pages** at:
👉 **https://dhaturupa.pages.dev/**
👉 **https://dhaturupa.pages.dev/selftest.html** (All 471 browser verification tests passing)

---

## 1. Zero Credit Card & 100% Free

- **No Billing Info or Credit Card Required**: Cloudflare Pages is completely free forever.
- **Unlimited Global Bandwidth**: Egress and CDN bandwidth are $0.00 without bandwidth metering.
- **SSL / TLS Included**: Automatic free SSL certificates on `*.pages.dev` and custom domains.
- **Asset Hashing**: Every file is content-hashed; future deployments only sync modified files in seconds.

---

## 2. Deployment Architecture

All static assets (HTML, JS, WebAssembly engine, and the 2,752 linguistic JSON shards and blobs) are served directly by Cloudflare Pages' global CDN edge:

- **Frontend App**: `index.html`, `app.js`, `render.js`, `sw.js` (~1.44 MB).
- **WebAssembly**: `pkg/dhaturupa_wasm_bg.wasm` + `pkg/dhaturupa_wasm.js`.
- **Linguistic Shards & Blobs**: `data/blobs/*.json`, `data/shards/*.json`, `data/shabda2.json`.
- **Headers ([`dist/_headers`](file:///home/edhiraj/Documents/projs/dhaturupa-wasm/dist/_headers))**: Sets `Content-Type: application/wasm` and `Cache-Control: public, max-age=31536000, immutable` for all data and wasm files.

---

## 3. How to Deploy Updates in 1 Command

Whenever you update your code or rebuild artifacts:

```bash
npm run deploy
```

This runs:
```bash
npx wrangler pages deploy dist --project-name dhaturupa --branch main --commit-dirty=true
```

Because Cloudflare Pages caches all files by SHA-256 hash, subsequent runs only upload changed files.

---

## 4. Verification

Run the end-to-end browser test against the live Cloudflare production deployment at any time:

```bash
node web/browser_check.cjs "https://dhaturupa.pages.dev/selftest.html"
```
Output:
```text
SELFTEST: PASS
checks passed: 471
```
