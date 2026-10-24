#!/usr/bin/env node
/**
 * scripts/upload-r2.cjs
 *
 * Uploads dist/data/ (~3.2 GB) to Cloudflare R2 bucket.
 * 
 * Usage:
 *   node scripts/upload-r2.cjs [--dry-run] [--bucket dhaturupa-data] [--concurrency 10]
 *
 * Authentication:
 *   Method 1 (Fastest - Direct S3 API with parallel streams, zero dependencies):
 *     Set environment variables or create a .env file:
 *       R2_ACCOUNT_ID=...
 *       R2_ACCESS_KEY_ID=...
 *       R2_SECRET_ACCESS_KEY=...
 *       R2_BUCKET=dhaturupa-data  (default)
 *
 *   Method 2 (Wrangler CLI):
 *     If no S3 credentials are provided, it automatically uses `npx wrangler r2 object put`
 *     with concurrent worker processes.
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const crypto = require('crypto');
const { spawn } = require('child_process');

// Load .env if present
const envPath = path.resolve(__dirname, '..', '.env');
if (fs.existsSync(envPath)) {
  const lines = fs.readFileSync(envPath, 'utf8').split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx > 0) {
      const k = trimmed.slice(0, eqIdx).trim();
      const v = trimmed.slice(eqIdx + 1).trim().replace(/^["']|["']$/g, '');
      if (!process.env[k]) process.env[k] = v;
    }
  }
}

// Parse CLI flags
const args = process.argv.slice(2);
const isDryRun = args.includes('--dry-run');
let bucket = process.env.R2_BUCKET || 'dhaturupa-data';
let concurrency = 8;

for (let i = 0; i < args.length; i++) {
  if (args[i] === '--bucket' && args[i + 1]) {
    bucket = args[i + 1];
    i++;
  } else if (args[i] === '--concurrency' && args[i + 1]) {
    concurrency = parseInt(args[i + 1], 10) || 8;
    i++;
  }
}

const rootDir = path.resolve(__dirname, '..');
const dataDir = path.join(rootDir, 'dist', 'data');

if (!fs.existsSync(dataDir)) {
  console.error('Error: dist/data directory not found. Please build artifacts first.');
  process.exit(1);
}

// Collect all files to upload
function collectFiles(dir, base = '') {
  let results = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    const relKey = base ? `${base}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      results = results.concat(collectFiles(fullPath, relKey));
    } else {
      const stat = fs.statSync(fullPath);
      results.push({ fullPath, key: relKey, size: stat.size });
    }
  }
  return results;
}

const allFiles = collectFiles(dataDir);
const totalBytes = allFiles.reduce((acc, f) => acc + f.size, 0);
const totalMb = (totalBytes / (1024 * 1024)).toFixed(2);

console.log('='.repeat(60));
console.log('  Cloudflare R2 Data Sync Tool');
console.log('='.repeat(60));
console.log(`Target Bucket : ${bucket}`);
console.log(`Source Folder : dist/data/`);
console.log(`Total Files   : ${allFiles.length}`);
console.log(`Total Size    : ${totalMb} MB (~${(totalBytes / (1024 * 1024 * 1024)).toFixed(2)} GB)`);
console.log(`Concurrency   : ${concurrency}`);

const accountId = process.env.R2_ACCOUNT_ID;
const accessKeyId = process.env.R2_ACCESS_KEY_ID;
const secretAccessKey = process.env.R2_SECRET_ACCESS_KEY;
const hasS3Creds = Boolean(accountId && accessKeyId && secretAccessKey);

console.log(`Transfer Mode : ${hasS3Creds ? 'Direct S3 API (SigV4 high-speed)' : 'Wrangler CLI pool'}`);
console.log('='.repeat(60));

if (isDryRun) {
  console.log('\n[DRY RUN MODE] No files were uploaded.');
  console.log('\nTop 10 largest files to transfer:');
  const sorted = [...allFiles].sort((a, b) => b.size - a.size).slice(0, 10);
  for (const f of sorted) {
    console.log(`  ${(f.size / (1024 * 1024)).toFixed(2).padStart(6)} MB : ${f.key}`);
  }
  console.log('\nTo execute live upload:');
  console.log(`  node scripts/upload-r2.cjs --bucket ${bucket}\n`);
  process.exit(0);
}

// Progress tracking cache file to allow resume
const cacheFile = path.join(rootDir, '.r2-upload-cache.json');
let uploadedCache = {};
if (fs.existsSync(cacheFile)) {
  try {
    uploadedCache = JSON.parse(fs.readFileSync(cacheFile, 'utf8'));
  } catch {}
}

function saveCache() {
  fs.writeFileSync(cacheFile, JSON.stringify(uploadedCache, null, 2), 'utf8');
}

// AWS SigV4 Signing Implementation (Zero external dependencies)
function hmac(key, str) {
  return crypto.createHmac('sha256', key).update(str, 'utf8').digest();
}

function hmacHex(key, str) {
  return crypto.createHmac('sha256', key).update(str, 'utf8').digest('hex');
}

function hashHex(bufferOrStr) {
  return crypto.createHash('sha256').update(bufferOrStr).digest('hex');
}

function uploadViaS3(file) {
  return new Promise((resolve, reject) => {
    const fileBuffer = fs.readFileSync(file.fullPath);
    const contentSha256 = hashHex(fileBuffer);
    const host = `${accountId}.r2.cloudflarestorage.com`;
    const canonicalUri = `/${bucket}/${encodeURI(file.key).replace(/\+/g, '%20')}`;

    const now = new Date();
    const dateStamp = now.toISOString().replace(/[:-]|\.\d{3}/g, '').slice(0, 8);
    const amzDate = now.toISOString().replace(/[:-]|\.\d{3}/g, '');

    const region = 'auto';
    const service = 's3';
    const credentialScope = `${dateStamp}/${region}/${service}/aws4_request`;

    const canonicalHeaders = `host:${host}\nx-amz-content-sha256:${contentSha256}\nx-amz-date:${amzDate}\n`;
    const signedHeaders = 'host;x-amz-content-sha256;x-amz-date';

    const canonicalRequest = [
      'PUT',
      canonicalUri,
      '',
      canonicalHeaders,
      signedHeaders,
      contentSha256,
    ].join('\n');

    const stringToSign = [
      'AWS4-HMAC-SHA256',
      amzDate,
      credentialScope,
      hashHex(canonicalRequest),
    ].join('\n');

    const kDate = hmac('AWS4' + secretAccessKey, dateStamp);
    const kRegion = hmac(kDate, region);
    const kService = hmac(kRegion, service);
    const kSigning = hmac(kService, 'aws4_request');
    const signature = hmacHex(kSigning, stringToSign);

    const authorization = `AWS4-HMAC-SHA256 Credential=${accessKeyId}/${credentialScope}, SignedHeaders=${signedHeaders}, Signature=${signature}`;

    const req = https.request(
      {
        hostname: host,
        port: 443,
        path: canonicalUri,
        method: 'PUT',
        headers: {
          'Host': host,
          'Content-Length': fileBuffer.length,
          'Content-Type': 'application/json',
          'x-amz-date': amzDate,
          'x-amz-content-sha256': contentSha256,
          'Authorization': authorization,
          'Cache-Control': file.key.endsWith('manifest.json')
            ? 'public, max-age=300'
            : 'public, max-age=31536000, immutable',
        },
      },
      (res) => {
        let body = '';
        res.on('data', (d) => (body += d));
        res.on('end', () => {
          if (res.statusCode >= 200 && res.statusCode < 300) {
            resolve();
          } else {
            reject(new Error(`S3 PUT ${file.key} failed with HTTP ${res.statusCode}: ${body}`));
          }
        });
      }
    );

    req.on('error', reject);
    req.write(fileBuffer);
    req.end();
  });
}

function uploadViaWrangler(file) {
  return new Promise((resolve, reject) => {
    const dest = `${bucket}/${file.key}`;
    const child = spawn(
      'npx',
      [
        'wrangler',
        'r2',
        'object',
        'put',
        dest,
        '--file',
        file.fullPath,
        '--content-type',
        'application/json',
        '--cache-control',
        file.key.endsWith('manifest.json') ? 'public, max-age=300' : 'public, max-age=31536000, immutable',
        '-y',
      ],
      { stdio: ['ignore', 'pipe', 'pipe'] }
    );

    let errOut = '';
    child.stderr.on('data', (d) => (errOut += d));
    child.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`wrangler failed on ${file.key} (exit ${code}): ${errOut.trim()}`));
    });
  });
}

// Queue execution with concurrency
async function run() {
  const queue = allFiles.filter((f) => {
    const cached = uploadedCache[f.key];
    return !(cached && cached.size === f.size);
  });

  const skipped = allFiles.length - queue.length;
  if (skipped > 0) {
    console.log(`Skipping ${skipped} files already synced (cache match).`);
  }
  console.log(`Uploading ${queue.length} files...\n`);

  let completed = 0;
  let failed = 0;
  let uploadedBytes = 0;
  const startTime = Date.now();

  async function worker() {
    while (queue.length > 0) {
      const file = queue.shift();
      try {
        if (hasS3Creds) {
          await uploadViaS3(file);
        } else {
          await uploadViaWrangler(file);
        }
        completed++;
        uploadedBytes += file.size;
        uploadedCache[file.key] = { size: file.size, time: Date.now() };

        if (completed % 25 === 0 || queue.length === 0) {
          saveCache();
          const elapsedSec = ((Date.now() - startTime) / 1000) || 1;
          const mbPerSec = ((uploadedBytes / (1024 * 1024)) / elapsedSec).toFixed(1);
          const percent = (((completed + skipped) / allFiles.length) * 100).toFixed(1);
          process.stdout.write(
            `\rProgress: [${completed + skipped}/${allFiles.length}] (${percent}%) | Speed: ${mbPerSec} MB/s | Current: ${file.key.slice(0, 30).padEnd(30)}`
          );
        }
      } catch (err) {
        failed++;
        console.error(`\nFailed: ${file.key} — ${err.message}`);
      }
    }
  }

  const workers = Array.from({ length: concurrency }, () => worker());
  await Promise.all(workers);
  saveCache();

  console.log('\n\n' + '='.repeat(60));
  console.log(`Upload Complete!`);
  console.log(`Successful: ${completed + skipped} / ${allFiles.length}`);
  if (failed > 0) {
    console.log(`Failed: ${failed} (re-run command to retry failed files)`);
  }
  console.log(`Time Elapsed: ${((Date.now() - startTime) / 1000).toFixed(1)}s`);
  console.log('='.repeat(60));
}

run().catch((err) => {
  console.error('Fatal error during upload:', err);
  process.exit(1);
});
