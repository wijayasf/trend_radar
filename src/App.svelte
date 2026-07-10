<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  const phases = [
    'Foundation scaffold',
    'Local DuckDB storage',
    'Threads API ingestion',
    'Entity detection',
    'Weekly export',
  ];

  let databaseHealth = 'Checking local database...';
  let keyword = 'AI Agent';
  let collectStatus = 'Idle';
  let fetchedCount = 0;
  let savedCount = 0;
  let rawPostsCount = 0;
  let isCollecting = false;
  let isImportingSamples = false;
  let sampleLoadedCount = 0;
  let sampleSavedCount = 0;
  let threadsTokenConfigured = false;
  let threadsUserIdConfigured = false;
  let appEnv = 'local';
  let envFileLoaded = false;
  let lastApiErrorCode = 'none';
  let lastApiErrorType = 'none';
  let lastApiErrorMessage = 'none';
  let detectStatus = 'Idle';
  let analyzedPosts = 0;
  let mentionsFound = 0;
  let savedMentions = 0;
  let isDetecting = false;
  let detectionPreview: AgentMentionPreview[] = [];
  let regionStatus = 'Idle';
  let regionPostsAnalyzed = 0;
  let indonesiaCount = 0;
  let globalCount = 0;
  let unknownCount = 0;
  let regionUpdatedMentions = 0;
  let isClassifyingRegions = false;

  type ThreadsCollectionResult = {
    keyword: string;
    fetched_count: number;
    saved_count: number;
    message: string;
  };

  type SampleThreadsImportResult = {
    loaded_count: number;
    saved_count: number;
    message: string;
  };

  type AgentMentionPreview = {
    agent_name: string;
    category: string;
    region: string;
    region_confidence: number;
    region_reason: string;
    confidence: number;
    source_snippet: string;
  };

  type EntityDetectionResult = {
    analyzed_posts: number;
    mentions_found: number;
    saved_count: number;
    message: string;
    preview: AgentMentionPreview[];
  };

  type RegionClassificationResult = {
    posts_analyzed: number;
    indonesia_count: number;
    global_count: number;
    unknown_count: number;
    updated_mentions_count: number;
    message: string;
    preview: AgentMentionPreview[];
  };

  type EnvConfigStatus = {
    threads_access_token_configured: boolean;
    threads_user_id_configured: boolean;
    app_env: string;
    app_env_configured: boolean;
    database_path: string;
    database_path_configured: boolean;
    env_file_loaded: boolean;
  };

  onMount(async () => {
    try {
      databaseHealth = await invoke<string>('check_database_health');
      await refreshRawPostsCount();
    } catch (error) {
      databaseHealth = `error: ${String(error)}`;
    }

    try {
      const status = await invoke<EnvConfigStatus>('env_config_status');
      threadsTokenConfigured = status.threads_access_token_configured;
      threadsUserIdConfigured = status.threads_user_id_configured;
      appEnv = status.app_env;
      envFileLoaded = status.env_file_loaded;
      collectStatus = threadsTokenConfigured
        ? 'Threads token configured'
        : 'Threads token missing';
    } catch (error) {
      collectStatus = `config error: ${String(error)}`;
    }
  });

  async function collectThreads() {
    const nextKeyword = keyword.trim();
    if (!nextKeyword || isCollecting) return;

    isCollecting = true;
    collectStatus = `Collecting "${nextKeyword}"...`;
    fetchedCount = 0;
    savedCount = 0;

    try {
      const result = await invoke<ThreadsCollectionResult>('collect_threads_by_keyword', {
        keyword: nextKeyword,
      });
      fetchedCount = result.fetched_count;
      savedCount = result.saved_count;
      collectStatus = result.message;
      await refreshRawPostsCount();
      clearLastApiError();
    } catch (error) {
      const parsedError = parseApiError(error);
      lastApiErrorCode = parsedError.code;
      lastApiErrorType = parsedError.type;
      lastApiErrorMessage = parsedError.message;
      collectStatus = `error: ${parsedError.friendly}`;
    } finally {
      isCollecting = false;
    }
  }

  async function importSamplePosts() {
    if (isImportingSamples) return;

    isImportingSamples = true;
    collectStatus = 'Importing sample posts...';
    sampleLoadedCount = 0;
    sampleSavedCount = 0;

    try {
      const result = await invoke<SampleThreadsImportResult>('import_sample_threads_posts');
      sampleLoadedCount = result.loaded_count;
      sampleSavedCount = result.saved_count;
      savedCount = result.saved_count;
      collectStatus = result.message;
      await refreshRawPostsCount();
      clearLastApiError();
    } catch (error) {
      const parsedError = parseApiError(error);
      lastApiErrorCode = parsedError.code;
      lastApiErrorType = parsedError.type;
      lastApiErrorMessage = parsedError.message;
      collectStatus = `sample import error: ${parsedError.friendly}`;
    } finally {
      isImportingSamples = false;
    }
  }

  function clearLastApiError() {
    lastApiErrorCode = 'none';
    lastApiErrorType = 'none';
    lastApiErrorMessage = 'none';
  }

  async function refreshRawPostsCount() {
    rawPostsCount = await invoke<number>('count_threads_raw_posts');
  }

  function parseApiError(error: unknown) {
    const raw = String(error);
    const code = raw.match(/\bcode=([^ ]+)/)?.[1] ?? 'none';
    const type = raw.match(/\btype=([^ ]+)/)?.[1] ?? 'none';
    const message = raw.match(/\bmessage=(.*)$/)?.[1]?.trim() ?? raw;
    const friendly = raw.split(' code=')[0];

    return { code, type, message, friendly };
  }

  async function detectAgentMentions() {
    if (isDetecting) return;

    isDetecting = true;
    detectStatus = 'Detecting agent mentions...';
    analyzedPosts = 0;
    mentionsFound = 0;
    savedMentions = 0;
    detectionPreview = [];

    try {
      const result = await invoke<EntityDetectionResult>('detect_agent_mentions');
      analyzedPosts = result.analyzed_posts;
      mentionsFound = result.mentions_found;
      savedMentions = result.saved_count;
      detectionPreview = result.preview;
      detectStatus = result.message;
    } catch (error) {
      detectStatus = `error: ${String(error)}`;
    } finally {
      isDetecting = false;
    }
  }

  async function classifyRegions() {
    if (isClassifyingRegions) return;

    isClassifyingRegions = true;
    regionStatus = 'Classifying regions...';
    regionPostsAnalyzed = 0;
    indonesiaCount = 0;
    globalCount = 0;
    unknownCount = 0;
    regionUpdatedMentions = 0;

    try {
      const result = await invoke<RegionClassificationResult>('classify_regions');
      regionPostsAnalyzed = result.posts_analyzed;
      indonesiaCount = result.indonesia_count;
      globalCount = result.global_count;
      unknownCount = result.unknown_count;
      regionUpdatedMentions = result.updated_mentions_count;
      detectionPreview = result.preview;
      regionStatus = result.message;
    } catch (error) {
      regionStatus = `error: ${String(error)}`;
    } finally {
      isClassifyingRegions = false;
    }
  }
</script>

<main class="app-shell">
  <section class="workspace">
    <div class="title-block">
      <p class="eyebrow">Local-first desktop intelligence</p>
      <h1>AI Agent Trend Radar</h1>
      <p>
        Foundation scaffold for tracking AI Agent trends from Threads and preparing weekly
        Indonesia/global reports.
      </p>
    </div>

    <div class="status-grid" aria-label="Project status">
      {#each phases as phase, index}
        <article class="status-card">
          <span>{String(index + 1).padStart(2, '0')}</span>
          <strong>{phase}</strong>
          <p>
            {index === 0
              ? 'Ready'
              : index === 1
                ? databaseHealth
                : index === 2
                  ? collectStatus
                  : index === 3
                    ? `${detectStatus} / ${regionStatus}`
                    : 'Planned'}
          </p>
        </article>
      {/each}
    </div>

    <section class="collector-panel" aria-label="Threads keyword collector">
      <div>
        <p class="panel-label">Threads keyword collector</p>
        <h2>Collect raw posts</h2>
      </div>

      <form on:submit|preventDefault={collectThreads}>
        <label for="keyword">Keyword</label>
        <div class="collector-row">
          <input
            id="keyword"
            bind:value={keyword}
            placeholder="AI Agent"
            autocomplete="off"
            disabled={isCollecting}
          />
          <button type="submit" disabled={isCollecting || !keyword.trim()}>
            {isCollecting ? 'Collecting' : 'Collect'}
          </button>
          <button type="button" on:click={importSamplePosts} disabled={isImportingSamples}>
            {isImportingSamples ? 'Importing' : 'Import Sample Posts'}
          </button>
        </div>
      </form>

      <div class="collector-result" aria-live="polite">
        <span>Collector status: {collectStatus}</span>
        <span>Threads token: {threadsTokenConfigured ? 'configured' : 'missing'}</span>
        <span>User ID: {threadsUserIdConfigured ? 'configured' : 'missing'}</span>
        <span>App env: {appEnv}</span>
        <span>.env: {envFileLoaded ? 'loaded' : 'not loaded'}</span>
        <span>Last API error code: {lastApiErrorCode}</span>
        <span>Last API error type: {lastApiErrorType}</span>
        <span>Last API error message: {lastApiErrorMessage}</span>
        <span>Fetched: {fetchedCount}</span>
        <span>Saved: {savedCount}</span>
        <span>Raw posts count: {rawPostsCount}</span>
        <span>Sample loaded: {sampleLoadedCount}</span>
        <span>Sample saved: {sampleSavedCount}</span>
      </div>
    </section>

    <section class="detector-panel" aria-label="Agent mention detector">
      <div class="detector-header">
        <div>
          <p class="panel-label">Entity detector</p>
          <h2>Detect agent mentions</h2>
        </div>
        <button type="button" on:click={detectAgentMentions} disabled={isDetecting}>
          {isDetecting ? 'Detecting' : 'Detect Agent Mentions'}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Raw posts analyzed: {analyzedPosts}</span>
        <span>Mentions found: {mentionsFound}</span>
        <span>Saved: {savedMentions}</span>
      </div>

      {#if detectionPreview.length > 0}
        <div class="mention-preview" aria-label="Detected mention preview">
          {#each detectionPreview as mention}
            <article class="mention-row">
              <div>
                <strong>{mention.agent_name}</strong>
                <span>{mention.category}</span>
                <span>
                  {mention.region}
                  {mention.region_confidence > 0
                    ? ` (${Math.round(mention.region_confidence * 100)}%)`
                    : ''}
                </span>
              </div>
              <p>{mention.source_snippet}</p>
              <span class="confidence">{Math.round(mention.confidence * 100)}%</span>
            </article>
          {/each}
        </div>
      {/if}
    </section>

    <section class="detector-panel" aria-label="Region classifier">
      <div class="detector-header">
        <div>
          <p class="panel-label">Region classifier</p>
          <h2>Classify Indonesia vs Global</h2>
        </div>
        <button type="button" on:click={classifyRegions} disabled={isClassifyingRegions}>
          {isClassifyingRegions ? 'Classifying' : 'Classify Regions'}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {regionStatus}</span>
        <span>Posts analyzed: {regionPostsAnalyzed}</span>
        <span>Indonesia: {indonesiaCount}</span>
        <span>Global: {globalCount}</span>
        <span>Unknown: {unknownCount}</span>
        <span>Mentions updated: {regionUpdatedMentions}</span>
      </div>
    </section>
  </section>
</main>
