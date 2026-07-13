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
  let discoveryStatus = 'Idle';
  let discoverySeedGroup = 'all';
  let discoveryMaxPerSeed = 10;
  let discoverySeedsProcessed = 0;
  let discoveryFetchedTotal = 0;
  let discoverySavedTotal = 0;
  let discoveryDuplicatesSkipped = 0;
  let discoveryFailedSeeds = 0;
  let discoveryMode = 'none';
  let discoveryErrors: string[] = [];
  let isRunningDiscovery = false;
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
  let sentimentStatus = 'Idle';
  let sentimentMentionsAnalyzed = 0;
  let positiveCount = 0;
  let neutralCount = 0;
  let negativeCount = 0;
  let mixedCount = 0;
  let sentimentUpdatedMentions = 0;
  let isClassifyingSentiments = false;
  let costStatus = 'Idle';
  let costMentionsAnalyzed = 0;
  let notMentionedCount = 0;
  let costPositiveCount = 0;
  let costNegativeBorosCount = 0;
  let costMixedCount = 0;
  let costUpdatedMentions = 0;
  let isClassifyingCostSignals = false;
  let weeklyStatus = 'Idle';
  let weeklyMetricsCount = 0;
  let weeklyIndonesiaCount = 0;
  let weeklyGlobalCount = 0;
  let weeklyUnknownCount = 0;
  let topIndonesia: WeeklyAgentMetric[] = [];
  let topGlobal: WeeklyAgentMetric[] = [];
  let topUnknown: WeeklyAgentMetric[] = [];
  let isAggregatingWeeklyMetrics = false;
  let markdownExportStatus = 'Idle';
  let csvExportStatus = 'Idle';
  let markdownExportPath = '';
  let csvExportPath = '';
  let markdownExportPreview = '';
  let csvExportPreview = '';
  let isExportingMarkdown = false;
  let isExportingCsv = false;

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

  type DiscoveryCrawlResult = {
    seed_group: string;
    mode: string;
    seeds_processed: number;
    fetched_total: number;
    saved_total: number;
    duplicates_skipped: number;
    failed_seeds: number;
    errors: string[];
    message: string;
  };

  type AgentMentionPreview = {
    agent_name: string;
    category: string;
    detection_source: string;
    needs_review: boolean;
    region: string;
    region_confidence: number;
    region_reason: string;
    sentiment: string;
    sentiment_confidence: number;
    sentiment_reason: string;
    cost_signal: string;
    cost_confidence: number;
    cost_reason: string;
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

  type SentimentClassificationResult = {
    mentions_analyzed: number;
    positive_count: number;
    neutral_count: number;
    negative_count: number;
    mixed_count: number;
    updated_mentions_count: number;
    message: string;
    preview: AgentMentionPreview[];
  };

  type CostClassificationResult = {
    mentions_analyzed: number;
    not_mentioned_count: number;
    cost_positive_count: number;
    cost_negative_boros_count: number;
    cost_mixed_count: number;
    updated_mentions_count: number;
    message: string;
    preview: AgentMentionPreview[];
  };

  type WeeklyAgentMetric = {
    rank: number;
    week_start: string;
    week_end: string;
    region: string;
    agent_name: string;
    category: string;
    mentions: number;
    positive_count: number;
    neutral_count: number;
    negative_count: number;
    mixed_count: number;
    cost_not_mentioned_count: number;
    cost_positive_count: number;
    cost_negative_boros_count: number;
    cost_mixed_count: number;
    positive_pct: number;
    negative_pct: number;
    cost_negative_boros_pct: number;
    trend_score: number;
  };

  type WeeklyAggregationResult = {
    metrics_count: number;
    indonesia_count: number;
    global_count: number;
    unknown_count: number;
    top_indonesia: WeeklyAgentMetric[];
    top_global: WeeklyAgentMetric[];
    top_unknown: WeeklyAgentMetric[];
    message: string;
  };

  type ReportExportResult = {
    file_path: string;
    rows_exported: number;
    message: string;
    preview: string;
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

  async function runDiscoveryCrawl() {
    if (isRunningDiscovery) return;

    isRunningDiscovery = true;
    discoveryStatus = 'Running AI Agent discovery crawl...';
    discoverySeedsProcessed = 0;
    discoveryFetchedTotal = 0;
    discoverySavedTotal = 0;
    discoveryDuplicatesSkipped = 0;
    discoveryFailedSeeds = 0;
    discoveryMode = 'none';
    discoveryErrors = [];

    try {
      const result = await invoke<DiscoveryCrawlResult>('run_discovery_crawl', {
        regionSeedGroup: discoverySeedGroup,
        maxPerSeed: discoveryMaxPerSeed,
        dryRun: false,
      });
      discoverySeedsProcessed = result.seeds_processed;
      discoveryFetchedTotal = result.fetched_total;
      discoverySavedTotal = result.saved_total;
      discoveryDuplicatesSkipped = result.duplicates_skipped;
      discoveryFailedSeeds = result.failed_seeds;
      discoveryMode = result.mode;
      discoveryErrors = result.errors;
      discoveryStatus = result.message;
      await refreshRawPostsCount();
    } catch (error) {
      discoveryStatus = `error: ${String(error)}`;
    } finally {
      isRunningDiscovery = false;
    }
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

  async function classifySentiments() {
    if (isClassifyingSentiments) return;

    isClassifyingSentiments = true;
    sentimentStatus = 'Classifying sentiments...';
    sentimentMentionsAnalyzed = 0;
    positiveCount = 0;
    neutralCount = 0;
    negativeCount = 0;
    mixedCount = 0;
    sentimentUpdatedMentions = 0;

    try {
      const result = await invoke<SentimentClassificationResult>('classify_sentiments');
      sentimentMentionsAnalyzed = result.mentions_analyzed;
      positiveCount = result.positive_count;
      neutralCount = result.neutral_count;
      negativeCount = result.negative_count;
      mixedCount = result.mixed_count;
      sentimentUpdatedMentions = result.updated_mentions_count;
      detectionPreview = result.preview;
      sentimentStatus = result.message;
    } catch (error) {
      sentimentStatus = `error: ${String(error)}`;
    } finally {
      isClassifyingSentiments = false;
    }
  }

  async function classifyCostSignals() {
    if (isClassifyingCostSignals) return;

    isClassifyingCostSignals = true;
    costStatus = 'Classifying cost signals...';
    costMentionsAnalyzed = 0;
    notMentionedCount = 0;
    costPositiveCount = 0;
    costNegativeBorosCount = 0;
    costMixedCount = 0;
    costUpdatedMentions = 0;

    try {
      const result = await invoke<CostClassificationResult>('classify_cost_signals');
      costMentionsAnalyzed = result.mentions_analyzed;
      notMentionedCount = result.not_mentioned_count;
      costPositiveCount = result.cost_positive_count;
      costNegativeBorosCount = result.cost_negative_boros_count;
      costMixedCount = result.cost_mixed_count;
      costUpdatedMentions = result.updated_mentions_count;
      detectionPreview = result.preview;
      costStatus = result.message;
    } catch (error) {
      costStatus = `error: ${String(error)}`;
    } finally {
      isClassifyingCostSignals = false;
    }
  }

  async function aggregateWeeklyMetrics() {
    if (isAggregatingWeeklyMetrics) return;

    isAggregatingWeeklyMetrics = true;
    weeklyStatus = 'Aggregating weekly metrics...';
    weeklyMetricsCount = 0;
    weeklyIndonesiaCount = 0;
    weeklyGlobalCount = 0;
    weeklyUnknownCount = 0;
    topIndonesia = [];
    topGlobal = [];
    topUnknown = [];

    try {
      const result = await invoke<WeeklyAggregationResult>('aggregate_weekly_metrics');
      weeklyMetricsCount = result.metrics_count;
      weeklyIndonesiaCount = result.indonesia_count;
      weeklyGlobalCount = result.global_count;
      weeklyUnknownCount = result.unknown_count;
      topIndonesia = result.top_indonesia;
      topGlobal = result.top_global;
      topUnknown = result.top_unknown;
      weeklyStatus = result.message;
    } catch (error) {
      weeklyStatus = `error: ${String(error)}`;
    } finally {
      isAggregatingWeeklyMetrics = false;
    }
  }

  async function exportMarkdownReport() {
    if (isExportingMarkdown) return;

    isExportingMarkdown = true;
    markdownExportStatus = 'Exporting Markdown report...';
    markdownExportPath = '';
    markdownExportPreview = '';

    try {
      const result = await invoke<ReportExportResult>('export_weekly_report_markdown');
      markdownExportStatus = result.message;
      markdownExportPath = result.file_path;
      markdownExportPreview = result.preview;
    } catch (error) {
      markdownExportStatus = `error: ${String(error)}`;
    } finally {
      isExportingMarkdown = false;
    }
  }

  async function exportCsvMetrics() {
    if (isExportingCsv) return;

    isExportingCsv = true;
    csvExportStatus = 'Exporting CSV metrics...';
    csvExportPath = '';
    csvExportPreview = '';

    try {
      const result = await invoke<ReportExportResult>('export_weekly_metrics_csv');
      csvExportStatus = result.message;
      csvExportPath = result.file_path;
      csvExportPreview = result.preview;
    } catch (error) {
      csvExportStatus = `error: ${String(error)}`;
    } finally {
      isExportingCsv = false;
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
                    ? `${detectStatus} / ${regionStatus} / ${sentimentStatus} / ${costStatus} / ${weeklyStatus}`
                    : 'Planned'}
          </p>
        </article>
      {/each}
    </div>

    <section class="collector-panel" aria-label="AI Agent discovery crawler">
      <div>
        <p class="panel-label">AI Agent discovery crawler</p>
        <h2>Run broad topic discovery</h2>
        <p class="panel-note">
          Discovery crawl searches broad AI Agent topics first, then entity detection extracts
          tool/agent names from collected posts.
        </p>
      </div>

      <form on:submit|preventDefault={runDiscoveryCrawl}>
        <label for="discovery-seed-group">Seed group</label>
        <div class="collector-row discovery-row">
          <select
            id="discovery-seed-group"
            bind:value={discoverySeedGroup}
            disabled={isRunningDiscovery}
          >
            <option value="all">All</option>
            <option value="indonesia">Indonesia</option>
            <option value="global">Global</option>
          </select>
          <input
            type="number"
            min="1"
            max="50"
            bind:value={discoveryMaxPerSeed}
            disabled={isRunningDiscovery}
            aria-label="Max per seed"
          />
          <button type="submit" disabled={isRunningDiscovery}>
            {isRunningDiscovery ? 'Running' : 'Run Discovery Crawl'}
          </button>
        </div>
      </form>

      <div class="collector-result" aria-live="polite">
        <span>Status: {discoveryStatus}</span>
        <span>Mode: {discoveryMode}</span>
        <span>Seeds processed: {discoverySeedsProcessed}</span>
        <span>Fetched total: {discoveryFetchedTotal}</span>
        <span>Saved total: {discoverySavedTotal}</span>
        <span>Duplicates skipped: {discoveryDuplicatesSkipped}</span>
        <span>Failed seeds: {discoveryFailedSeeds}</span>
        {#if discoveryErrors.length > 0}
          <span>Diagnostics: {discoveryErrors.join(' | ')}</span>
        {/if}
      </div>
    </section>

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
                  {mention.detection_source}
                  {mention.needs_review ? ' / review' : ''}
                </span>
                <span>
                  {mention.region}
                  {mention.region_confidence > 0
                    ? ` (${Math.round(mention.region_confidence * 100)}%)`
                    : ''}
                </span>
                <span>
                  {mention.sentiment}
                  {mention.sentiment_confidence > 0
                    ? ` (${Math.round(mention.sentiment_confidence * 100)}%)`
                    : ''}
                </span>
                <span>
                  {mention.cost_signal}
                  {mention.cost_confidence > 0
                    ? ` (${Math.round(mention.cost_confidence * 100)}%)`
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

    <section class="detector-panel" aria-label="Sentiment classifier">
      <div class="detector-header">
        <div>
          <p class="panel-label">Sentiment classifier</p>
          <h2>Classify Sentiments</h2>
        </div>
        <button
          type="button"
          on:click={classifySentiments}
          disabled={isClassifyingSentiments}
        >
          {isClassifyingSentiments ? 'Classifying' : 'Classify Sentiments'}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {sentimentStatus}</span>
        <span>Mentions analyzed: {sentimentMentionsAnalyzed}</span>
        <span>Positive: {positiveCount}</span>
        <span>Neutral: {neutralCount}</span>
        <span>Negative: {negativeCount}</span>
        <span>Mixed: {mixedCount}</span>
        <span>Mentions updated: {sentimentUpdatedMentions}</span>
      </div>
    </section>

    <section class="detector-panel" aria-label="Cost signal classifier">
      <div class="detector-header">
        <div>
          <p class="panel-label">Cost signal classifier</p>
          <h2>Classify Cost Signals</h2>
        </div>
        <button
          type="button"
          on:click={classifyCostSignals}
          disabled={isClassifyingCostSignals}
        >
          {isClassifyingCostSignals ? 'Classifying' : 'Classify Cost Signals'}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {costStatus}</span>
        <span>Mentions analyzed: {costMentionsAnalyzed}</span>
        <span>Not mentioned: {notMentionedCount}</span>
        <span>Cost positive: {costPositiveCount}</span>
        <span>Cost negative/boros: {costNegativeBorosCount}</span>
        <span>Cost mixed: {costMixedCount}</span>
        <span>Mentions updated: {costUpdatedMentions}</span>
      </div>
    </section>

    <section class="detector-panel" aria-label="Weekly trend metrics">
      <div class="detector-header">
        <div>
          <p class="panel-label">Weekly trend metrics</p>
          <h2>Aggregate Weekly Metrics</h2>
        </div>
        <button
          type="button"
          on:click={aggregateWeeklyMetrics}
          disabled={isAggregatingWeeklyMetrics}
        >
          {isAggregatingWeeklyMetrics ? 'Aggregating' : 'Aggregate Weekly Metrics'}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {weeklyStatus}</span>
        <span>Metric rows: {weeklyMetricsCount}</span>
        <span>Indonesia rows: {weeklyIndonesiaCount}</span>
        <span>Global rows: {weeklyGlobalCount}</span>
        <span>Unknown rows: {weeklyUnknownCount}</span>
      </div>

      <div class="metrics-groups">
        <div>
          <h3>Top Indonesia</h3>
          {@render MetricTable(topIndonesia)}
        </div>
        <div>
          <h3>Top Global</h3>
          {@render MetricTable(topGlobal)}
        </div>
        {#if topUnknown.length > 0}
          <div>
            <h3>Top Unknown</h3>
            {@render MetricTable(topUnknown)}
          </div>
        {/if}
      </div>
    </section>

    <section class="detector-panel" aria-label="Report export">
      <div class="detector-header">
        <div>
          <p class="panel-label">Report export</p>
          <h2>Export Weekly Report</h2>
        </div>
        <div class="export-actions">
          <button type="button" on:click={exportMarkdownReport} disabled={isExportingMarkdown}>
            {isExportingMarkdown ? 'Exporting' : 'Export Markdown Report'}
          </button>
          <button type="button" on:click={exportCsvMetrics} disabled={isExportingCsv}>
            {isExportingCsv ? 'Exporting' : 'Export CSV Metrics'}
          </button>
        </div>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Markdown: {markdownExportStatus}</span>
        {#if markdownExportPath}
          <span>Markdown path: {markdownExportPath}</span>
        {/if}
        <span>CSV: {csvExportStatus}</span>
        {#if csvExportPath}
          <span>CSV path: {csvExportPath}</span>
        {/if}
      </div>

      {#if markdownExportPreview || csvExportPreview}
        <div class="export-preview-grid">
          {#if markdownExportPreview}
            <div>
              <h3>Markdown preview</h3>
              <pre>{markdownExportPreview}</pre>
            </div>
          {/if}
          {#if csvExportPreview}
            <div>
              <h3>CSV preview</h3>
              <pre>{csvExportPreview}</pre>
            </div>
          {/if}
        </div>
      {/if}
    </section>
  </section>
</main>

{#snippet MetricTable(metrics: WeeklyAgentMetric[])}
  {#if metrics.length > 0}
    <div class="metrics-table-wrap">
      <table class="metrics-table">
        <thead>
          <tr>
            <th>Rank</th>
            <th>Agent</th>
            <th>Category</th>
            <th>Region</th>
            <th>Mentions</th>
            <th>Positive %</th>
            <th>Negative %</th>
            <th>Boros %</th>
            <th>Score</th>
          </tr>
        </thead>
        <tbody>
          {#each metrics as metric}
            <tr>
              <td>{metric.rank}</td>
              <td>{metric.agent_name}</td>
              <td>{metric.category}</td>
              <td>{metric.region}</td>
              <td>{metric.mentions}</td>
              <td>{metric.positive_pct.toFixed(1)}</td>
              <td>{metric.negative_pct.toFixed(1)}</td>
              <td>{metric.cost_negative_boros_pct.toFixed(1)}</td>
              <td>{metric.trend_score.toFixed(1)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="empty-state">No metrics yet.</p>
  {/if}
{/snippet}
