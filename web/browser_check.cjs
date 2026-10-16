// Browser self-test driver: real remote control of headless Chromium via
// chromedriver (W3C WebDriver, plain HTTP — Firefox's headless dump-dom is
// unusable in this environment). Loads the in-page self-test
// (selftest.html), which drives the real app DOM through every flow that
// regressed: routing, list filters, form search (base/prefixed/
// declined-prefixed/kṛdanta-subanta, Deva + HK), hit-link navigation with
// prefix preselected, family default, and the kṛदांanta click-through.
//
// Run by ./check.sh, or standalone: node web/browser_check.cjs [url]
//
// Requires `chromedriver` on PATH (WebDriver 4 / chromium ń compatible).

const { spawn } = require('child_process');

const URL_UNDER_TEST = process.argv[2] || 'http://127.0.0.1:8124/selftest.html';
const PORT = 9515;
const DEADLINE_MS = 240_000;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const post = async (path, body) => {
  const res = await fetch(`http://127.0.0.1:${PORT}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  const json = await res.json().catch(() => ({}));
  if (!res.ok && json.value && json.value.error) {
    throw new Error(`${json.value.error}: ${json.value.message || ''}`);
  }
  return json;
};

async function main() {
  const driver = spawn('chromedriver', [`--port=${PORT}`, '--silent'], { stdio: 'ignore' });
  let sessionId = null;
  try {
    // Wait for the driver's HTTP endpoint.
    for (let i = 0; i < 40; i += 1) {
      try {
        await fetch(`http://127.0.0.1:${PORT}/status`);
        break;
      } catch {
        await sleep(250);
      }
    }
    const created = await post('/session', {
      capabilities: {
        alwaysMatch: {
          browserName: 'chrome',
          'goog:chromeOptions': {
            binary: '/usr/bin/chromium',
            args: ['--headless=new', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage'],
          },
        },
      },
    });
    sessionId = created.value.sessionId;
    if (!sessionId) throw new Error(`no session: ${JSON.stringify(created)}`);

    await post(`/session/${sessionId}/url`, { url: URL_UNDER_TEST });

    const readVerdict = async () => {
      const res = await post(`/session/${sessionId}/execute/sync`, {
        script:
          'const p = document.getElementById("selftest-out");' +
          'return p ? p.textContent : "NO-OUT-ELEMENT";',
        args: [],
      });
      return res.value;
    };

    const t0 = Date.now();
    let verdict = '';
    let last = '';
    while (Date.now() - t0 < DEADLINE_MS) {
      await sleep(2000);
      try {
        last = (await readVerdict()) || last;
      } catch (e) {
        last = `poll error: ${e.message}`;
      }
      if (last.includes('SELFTEST: PASS') || last.includes('SELFTEST: FAIL')) {
        verdict = last;
        break;
      }
    }
    if (!verdict) {
      console.error(`BROWSER SELF-TEST: no verdict after ${DEADLINE_MS / 1000}s; last state:\n${last}`);
      process.exitCode = 1;
    } else if (verdict.includes('SELFTEST: FAIL')) {
      console.error(verdict.trim());
      process.exitCode = 1;
    } else {
      console.log(verdict.trim().split('\n').slice(0, 2).join('\n'));
    }
  } finally {
    if (sessionId) {
      fetch(`http://127.0.0.1:${PORT}/session/${sessionId}`, { method: 'DELETE' }).catch(() => {});
    }
    driver.kill('SIGTERM');
  }
}

main().catch((e) => {
  console.error(`BROWSER SELF-TEST: driver error: ${e.message}`);
  process.exit(1);
});
