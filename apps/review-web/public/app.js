const healthCards = document.querySelector('#health-cards');
const seedForm = document.querySelector('#seed-form');
const seedButton = document.querySelector('#seed-button');
const seedResult = document.querySelector('#seed-result');
const crawlForm = document.querySelector('#crawl-form');
const crawlButton = document.querySelector('#crawl-button');
const crawlResult = document.querySelector('#crawl-result');

document.querySelector('#refresh-health').addEventListener('click', loadHealth);
seedForm.addEventListener('submit', testSeed);
crawlForm.addEventListener('submit', runCrawl);

loadHealth();

async function loadHealth() {
  healthCards.innerHTML = '<p class="muted">Loading health...</p>';
  try {
    const data = await fetchJson('/health');
    healthCards.innerHTML = [
      card('App env', data.env),
      card('Token configured', data.tokenConfigured ? 'yes' : 'no'),
      card('User ID configured', data.userIdConfigured ? 'yes' : 'no'),
    ].join('');
  } catch (error) {
    healthCards.innerHTML = errorBox(error.message);
  }
}

async function testSeed(event) {
  event.preventDefault();
  const keyword = document.querySelector('#seed-keyword').value.trim();
  if (!keyword) return;

  setLoading(seedButton, true, 'Testing...');
  seedResult.innerHTML = '<p class="muted">Testing seed...</p>';
  try {
    const data = await fetchJson(`/api/test-seed?q=${encodeURIComponent(keyword)}`);
    seedResult.innerHTML = renderSeedResult(data);
  } catch (error) {
    seedResult.innerHTML = errorBox(error.message);
  } finally {
    setLoading(seedButton, false, 'Test Seed');
  }
}

async function runCrawl(event) {
  event.preventDefault();
  const seeds = document
    .querySelector('#seed-list')
    .value.split('\n')
    .map((seed) => seed.trim())
    .filter(Boolean);
  const maxPerSeed = Number(document.querySelector('#max-per-seed').value || 3);

  setLoading(crawlButton, true, 'Running...');
  crawlResult.innerHTML = '<p class="muted">Running discovery crawl...</p>';
  try {
    const data = await fetchJson('/api/discovery-crawl', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ seeds, maxPerSeed }),
    });
    crawlResult.innerHTML = renderCrawlResult(data);
  } catch (error) {
    crawlResult.innerHTML = errorBox(error.message);
  } finally {
    setLoading(crawlButton, false, 'Run Discovery Crawl');
  }
}

async function fetchJson(url, options) {
  const response = await fetch(url, options);
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.errorSummary || `Request failed with HTTP ${response.status}`);
  }
  return data;
}

function renderSeedResult(data) {
  return `
    <div class="summary-grid">
      ${metric('Status', data.status)}
      ${metric('Fetched', data.fetchedCount)}
      ${metric('Detail fetched', data.detailFetchedCount)}
      ${metric('Text available', data.textAvailableCount)}
    </div>
    ${data.errorSummary ? `<p class="hint">${escapeHtml(data.errorSummary)}</p>` : ''}
    ${renderEntities(data.detectedEntities)}
    ${renderPosts(data.samplePosts)}
  `;
}

function renderCrawlResult(data) {
  return `
    <div class="summary-grid">
      ${metric('Seeds processed', data.seedsProcessed)}
      ${metric('Fetched total', data.fetchedTotal)}
      ${metric('Detail fetched', data.detailFetchedTotal)}
      ${metric('Text available', data.textAvailableTotal)}
      ${metric('Saved unique', data.savedUniqueTotal)}
      ${metric('Duplicates skipped', data.duplicatesSkipped)}
      ${metric('Zero-result seeds', data.zeroResultSeeds)}
      ${metric('Failed seeds', data.failedSeeds)}
    </div>
    ${
      data.permissionLimitedHint
        ? '<p class="hint">Crawler may be limited to authenticated user posts until threads_keyword_search is approved for public search.</p>'
        : ''
    }
    ${data.errorSummary ? `<p class="hint">${escapeHtml(data.errorSummary)}</p>` : ''}
    ${renderEntities(data.detectedEntities)}
    ${renderSeedRows(data.seedResults)}
    ${renderPosts(data.samplePosts)}
  `;
}

function renderEntities(entities) {
  if (!entities || entities.length === 0) return '<p class="muted">No entities detected yet.</p>';
  return `<div class="pill-row">${entities
    .map((entity) => `<span class="pill">${escapeHtml(entity.name)} (${entity.count})</span>`)
    .join('')}</div>`;
}

function renderSeedRows(rows) {
  if (!rows || rows.length === 0) return '';
  return `
    <table>
      <thead>
        <tr><th>Seed</th><th>Status</th><th>Fetched</th><th>Saved</th><th>Summary</th></tr>
      </thead>
      <tbody>
        ${rows
          .map(
            (row) => `
              <tr>
                <td>${escapeHtml(row.seed)}</td>
                <td>${escapeHtml(row.status)}</td>
                <td>${row.fetchedCount}</td>
                <td>${row.savedCount}</td>
                <td>${escapeHtml(row.errorSummary || '-')}</td>
              </tr>
            `,
          )
          .join('')}
      </tbody>
    </table>
  `;
}

function renderPosts(posts) {
  if (!posts || posts.length === 0) return '<p class="muted">No sample posts returned.</p>';
  return `<div class="post-list">${posts
    .map(
      (post) => `
        <article class="post">
          <strong>${escapeHtml(post.username || post.id)}</strong>
          <p>${escapeHtml(post.textSnippet || 'Text unavailable')}</p>
          <span>${escapeHtml(post.mediaType || 'TEXT')} · ${escapeHtml(post.timestamp || 'no timestamp')}</span>
          ${post.permalink ? `<a href="${escapeHtml(post.permalink)}" target="_blank" rel="noreferrer">Open permalink</a>` : ''}
        </article>
      `,
    )
    .join('')}</div>`;
}

function card(label, value) {
  return `<article class="card"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></article>`;
}

function metric(label, value) {
  return `<article class="metric"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></article>`;
}

function errorBox(message) {
  return `<p class="error">${escapeHtml(message)}</p>`;
}

function setLoading(button, loading, label) {
  button.disabled = loading;
  button.innerHTML = loading ? `<span class="spinner"></span>${label}` : label;
}

function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}
