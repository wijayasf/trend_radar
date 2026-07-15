import express from 'express';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

loadLocalEnv();

const app = express();
const PORT = Number(process.env.PORT || 3000);
const APP_NAME = 'AI Agent Trend Radar Review Web Demo';
const THREADS_KEYWORD_SEARCH_ENDPOINT = 'https://graph.threads.net/v1.0/keyword_search';
const THREADS_DETAIL_ENDPOINT_BASE = 'https://graph.threads.net/v1.0';
const THREADS_DETAIL_FIELDS = 'id,text,media_type,permalink,timestamp,username,owner';
const DEFAULT_SEEDS = ['AI Agent', 'Ponytail', 'Cavemen', 'Astryx', 'Claude Code'];
const MAX_SEEDS = 12;
const MAX_PER_SEED_LIMIT = 10;

const ENTITY_ALIASES = [
  { canonical: 'Ponytail', aliases: ['ponytail'] },
  { canonical: 'Caveman', aliases: ['caveman', 'cavemen'] },
  { canonical: 'Astryx', aliases: ['astryx'] },
  { canonical: 'Claude Code', aliases: ['claude code'] },
  { canonical: 'Cline', aliases: ['cline'] },
  { canonical: 'Cursor', aliases: ['cursor'] },
  { canonical: 'MCP', aliases: ['mcp', 'model context protocol'] },
  { canonical: 'Codex CLI', aliases: ['codex cli'] },
];

app.disable('x-powered-by');
app.use(express.json({ limit: '16kb' }));
app.use(express.static(path.join(__dirname, 'public')));

app.get('/health', (_req, res) => {
  res.json({
    ok: true,
    app: APP_NAME,
    env: process.env.APP_ENV || 'review',
    tokenConfigured: isConfigured(process.env.THREADS_ACCESS_TOKEN),
    userIdConfigured: isConfigured(process.env.THREADS_USER_ID),
  });
});

app.get('/api/test-seed', async (req, res) => {
  const keyword = String(req.query.q || '').trim();
  if (!keyword) {
    return res.status(400).json({
      keyword,
      status: 'error',
      errorSummary: 'Keyword is required.',
    });
  }

  try {
    const result = await runSeed(keyword, 5);
    res.json({
      keyword,
      status: result.status,
      fetchedCount: result.fetchedCount,
      detailFetchedCount: result.detailFetchedCount,
      textAvailableCount: result.textAvailableCount,
      samplePosts: result.samplePosts,
      detectedEntities: detectEntities(result.samplePosts),
      errorSummary: result.errorSummary,
    });
  } catch (error) {
    const safe = safeError(error);
    res.status(safe.httpStatus).json({
      keyword,
      status: safe.status,
      fetchedCount: 0,
      detailFetchedCount: 0,
      textAvailableCount: 0,
      samplePosts: [],
      detectedEntities: [],
      errorSummary: safe.message,
    });
  }
});

app.post('/api/discovery-crawl', async (req, res) => {
  const inputSeeds = Array.isArray(req.body?.seeds) ? req.body.seeds : DEFAULT_SEEDS;
  const seeds = normalizeSeeds(inputSeeds);
  const maxPerSeed = clampNumber(req.body?.maxPerSeed, 1, MAX_PER_SEED_LIMIT, 3);

  if (seeds.length === 0) {
    return res.status(400).json({
      status: 'error',
      errorSummary: 'At least one seed keyword is required.',
    });
  }

  const seenPostIds = new Set();
  const uniquePosts = [];
  const seedResults = [];
  let fetchedTotal = 0;
  let detailFetchedTotal = 0;
  let textAvailableTotal = 0;
  let duplicatesSkipped = 0;
  let zeroResultSeeds = 0;
  let failedSeeds = 0;
  let permissionLimitedHint = false;

  for (const seed of seeds) {
    try {
      const result = await runSeed(seed, maxPerSeed);
      fetchedTotal += result.fetchedCount;
      detailFetchedTotal += result.detailFetchedCount;
      textAvailableTotal += result.textAvailableCount;
      if (result.fetchedCount === 0) zeroResultSeeds += 1;

      let savedCount = 0;
      for (const post of result.samplePosts) {
        if (!post.id || seenPostIds.has(post.id)) {
          duplicatesSkipped += 1;
          continue;
        }
        seenPostIds.add(post.id);
        uniquePosts.push(post);
        savedCount += 1;
      }

      seedResults.push({
        seed,
        status: result.status,
        fetchedCount: result.fetchedCount,
        savedCount,
        errorSummary: result.errorSummary,
      });
    } catch (error) {
      const safe = safeError(error);
      failedSeeds += 1;
      permissionLimitedHint = permissionLimitedHint || safe.permissionLimitedHint;
      seedResults.push({
        seed,
        status: safe.status,
        fetchedCount: 0,
        savedCount: 0,
        errorSummary: safe.message,
      });
    }
  }

  if (zeroResultSeeds > 0 && uniquePosts.length === 0) {
    permissionLimitedHint = true;
  }

  res.json({
    status: failedSeeds === seeds.length ? 'error' : 'success',
    seedsProcessed: seeds.length,
    fetchedTotal,
    detailFetchedTotal,
    textAvailableTotal,
    savedUniqueTotal: uniquePosts.length,
    duplicatesSkipped,
    zeroResultSeeds,
    failedSeeds,
    detectedEntities: detectEntities(uniquePosts),
    samplePosts: uniquePosts.slice(0, 12),
    seedResults,
    permissionLimitedHint,
    errorSummary:
      permissionLimitedHint && uniquePosts.length === 0
        ? 'Crawler may be limited to authenticated user posts until threads_keyword_search is approved for public search.'
        : '',
  });
});

app.listen(PORT, () => {
  console.log(`${APP_NAME} listening on port ${PORT}`);
});

function loadLocalEnv() {
  const envPath = path.join(__dirname, '.env');
  if (!fs.existsSync(envPath)) return;

  const lines = fs.readFileSync(envPath, 'utf8').split(/\r?\n/);
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const separatorIndex = trimmed.indexOf('=');
    if (separatorIndex <= 0) continue;
    const key = trimmed.slice(0, separatorIndex).trim();
    const value = trimmed.slice(separatorIndex + 1).trim();
    if (!process.env[key]) {
      process.env[key] = value.replace(/^["']|["']$/g, '');
    }
  }
}

function isConfigured(value) {
  return Boolean(String(value || '').trim());
}

function readServerToken() {
  const token = String(process.env.THREADS_ACCESS_TOKEN || '').trim();
  if (!token) {
    const error = new Error('Server token is not configured.');
    error.status = 'missing_token';
    error.httpStatus = 500;
    throw error;
  }
  return token;
}

async function runSeed(keyword, maxPerSeed) {
  const searchJson = await keywordSearch(keyword);
  const searchPosts = Array.isArray(searchJson.data) ? searchJson.data : [];
  const limitedPosts = searchPosts.slice(0, maxPerSeed);
  const samplePosts = [];
  let detailFetchedCount = 0;
  let textAvailableCount = 0;
  const errors = [];

  for (const post of limitedPosts) {
    if (!post?.id) continue;
    let detail = post;
    if (!hasText(post)) {
      try {
        detail = await fetchPostDetail(post.id);
        detailFetchedCount += 1;
      } catch (error) {
        errors.push(safeError(error).message);
      }
    }

    const safePost = toSafePost(detail);
    if (safePost.textSnippet) textAvailableCount += 1;
    samplePosts.push(safePost);
  }

  return {
    status: limitedPosts.length === 0 ? 'zero_result' : 'success',
    fetchedCount: limitedPosts.length,
    detailFetchedCount,
    textAvailableCount,
    samplePosts,
    errorSummary:
      limitedPosts.length === 0
        ? 'Keyword search succeeded but no posts were returned for this seed.'
        : errors.join(' | '),
  };
}

async function keywordSearch(keyword) {
  const token = readServerToken();
  const url = new URL(THREADS_KEYWORD_SEARCH_ENDPOINT);
  url.searchParams.set('q', keyword);
  url.searchParams.set('media_type', 'TEXT');

  const response = await fetch(url, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  return readThreadsResponse(response, 'Threads keyword search failed.');
}

async function fetchPostDetail(postId) {
  const token = readServerToken();
  const url = new URL(`${THREADS_DETAIL_ENDPOINT_BASE}/${encodeURIComponent(postId)}`);
  url.searchParams.set('fields', THREADS_DETAIL_FIELDS);

  const response = await fetch(url, {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
  return readThreadsResponse(response, 'Threads post detail fetch failed.');
}

async function readThreadsResponse(response, fallbackMessage) {
  const text = await response.text();
  let json = {};
  try {
    json = text ? JSON.parse(text) : {};
  } catch (_error) {
    const error = new Error(`${fallbackMessage} Threads returned a non-JSON response.`);
    error.httpStatus = response.status || 502;
    throw error;
  }

  if (!response.ok || json.error) {
    const apiError = json.error || {};
    const message = safeApiMessage(apiError, fallbackMessage);
    const error = new Error(message);
    error.httpStatus = response.status || 502;
    error.status = apiError.code === 10 ? 'permission_error' : 'api_error';
    error.permissionLimitedHint = apiError.code === 10;
    throw error;
  }

  return json;
}

function safeApiMessage(apiError, fallbackMessage) {
  if (
    apiError.code === 10 ||
    String(apiError.message || '').includes('Application does not have permission')
  ) {
    return 'Token is valid, but keyword search is not authorized. Check threads_keyword_search approval/scope.';
  }

  const code = apiError.code ? ` code=${apiError.code}` : '';
  const type = apiError.type ? ` type=${apiError.type}` : '';
  const message = apiError.message ? ` message=${apiError.message}` : '';
  return `${fallbackMessage}${code}${type}${message}`.trim();
}

function safeError(error) {
  return {
    status: error.status || 'error',
    httpStatus: error.httpStatus || 500,
    permissionLimitedHint: Boolean(error.permissionLimitedHint),
    message: String(error.message || 'Unexpected server error.').slice(0, 280),
  };
}

function hasText(post) {
  return Boolean(String(post?.text || '').trim());
}

function toSafePost(post) {
  return {
    id: String(post?.id || ''),
    textSnippet: snippet(post?.text || ''),
    mediaType: post?.media_type || '',
    timestamp: post?.timestamp || '',
    username: post?.username || post?.owner?.username || '',
    permalink: post?.permalink || '',
  };
}

function snippet(value) {
  return String(value || '').replace(/\s+/g, ' ').trim().slice(0, 180);
}

function detectEntities(posts) {
  const found = new Map();
  for (const post of posts) {
    const haystack = `${post.textSnippet || ''}`.toLowerCase();
    for (const entity of ENTITY_ALIASES) {
      if (entity.aliases.some((alias) => hasAlias(haystack, alias))) {
        const current = found.get(entity.canonical) || 0;
        found.set(entity.canonical, current + 1);
      }
    }
  }

  return [...found.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
}

function hasAlias(text, alias) {
  const escaped = alias.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`(^|[^a-z0-9])${escaped}([^a-z0-9]|$)`, 'i').test(text);
}

function normalizeSeeds(inputSeeds) {
  const seen = new Set();
  const seeds = [];
  for (const value of inputSeeds) {
    const seed = String(value || '').trim();
    const key = seed.toLowerCase();
    if (!seed || seen.has(key)) continue;
    seen.add(key);
    seeds.push(seed);
    if (seeds.length >= MAX_SEEDS) break;
  }
  return seeds;
}

function clampNumber(value, min, max, fallback) {
  const number = Number(value);
  if (!Number.isFinite(number)) return fallback;
  return Math.min(max, Math.max(min, Math.trunc(number)));
}
