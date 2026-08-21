<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  const flowSteps = [
    {
      number: '1',
      key: 'discovery',
      label: 'Discovery',
      description: 'Keep posts only when a concrete named entity can be extracted.',
    },
    {
      number: '2',
      key: 'entities',
      label: 'Entity Detection',
      description: 'Extract named AI agents, skills, MCP servers, frameworks, and tools.',
    },
    {
      number: '3',
      key: 'candidates',
      label: 'Candidate Review',
      description: 'Review meaningful newly discovered product or tool names.',
    },
    {
      number: '4',
      key: 'classification',
      label: 'Classification',
      description: 'Classify region, sentiment, and cost/boros signals.',
    },
    {
      number: '5',
      key: 'weekly',
      label: 'Weekly Metrics',
      description: 'Rank agents by region and trend score.',
    },
    {
      number: '6',
      key: 'export',
      label: 'Export Report',
      description: 'Write Markdown and CSV outputs for the weekly report.',
    },
  ];

  let databaseHealth = 'Checking local database...';
  let discoveryStatus = 'Idle';
  let discoverySource = 'official_threads';
  let discoverySeedGroup = 'all';
  let discoveryMaxPerSeed = 10;
  let apifySeedsText = [
    'AI Agent',
    'Agentic AI',
    'Claude Code',
    'Claude Code plugin',
    'MCP server',
    'Ponytail Claude Code',
    'Cavemen Claude Code',
    'Astryx AI',
  ].join('\n');
  let discoverySeedsProcessed = 0;
  let discoveryFetchedTotal = 0;
  let discoveryDetailFetchedTotal = 0;
  let discoveryDetailFailedTotal = 0;
  let discoveryIdOnlyResultsCount = 0;
  let discoveryTextMissingTotal = 0;
  let discoverySavedTotal = 0;
  let discoveryDuplicatesSkipped = 0;
  let discoveryFailedSeeds = 0;
  let discoveryMode = 'none';
  let discoveryRunId = 'none';
  let discoveryStartedAt = 'none';
  let discoveryFinishedAt = 'none';
  let discoveryDurationMs = 0;
  let discoveryZeroResultSeeds = 0;
  let discoveryPermissionLimitedHint = false;
  let discoveryLastSuccessfulSeed = 'none';
  let discoveryLastErrorSummary = 'none';
  let discoveryErrors: string[] = [];
  let discoverySeedResults: DiscoverySeedDiagnostics[] = [];
  let discoverySeedTestKeyword = 'AI Agent';
  let discoverySeedTestStatus = 'Idle';
  let discoverySeedTestResult: DiscoverySeedTestResult | null = null;
  let isTestingDiscoverySeed = false;
  let isRunningDiscovery = false;
  let isReplayingApify = false;
  let isImportingApifyCache = false;
  let apifyDatasetFilePath = '';
  let explainxFilePath = '';
  let explainxImportStatus = 'Idle';
  let explainxImported = 0;
  let explainxInserted = 0;
  let explainxUpdated = 0;
  let explainxSkipped = 0;
  let explainxInvalid = 0;
  let explainxLinkedExactAlias = 0;
  let explainxReviewNeeded = 0;
  let explainxUnlinked = 0;
  let explainxImportPreview: ExplainXImportPreview[] = [];
  let isImportingExplainX = false;
  let externalIdentityReviewStatus = 'Idle';
  let externalIdentityReviewItems: ExternalIdentityReviewItem[] = [];
  let externalIdentityReviewPending = 0;
  let externalIdentityReviewApproved = 0;
  let externalIdentityReviewRejected = 0;
  let externalIdentityReviewAmbiguous = 0;
  let isLoadingExternalIdentityReviews = false;
  let activeExternalIdentityReviewAction = '';
  let activeExternalIdentityReviewDecision = '';
  let externalIdentityReviewHistory: ExternalIdentityReviewHistoryEntry[] = [];
  let externalIdentityReviewHistoryLinkId = '';
  let externalIdentityReviewHistoryStatus = 'Idle';
  let apifyActorId = 'none';
  let apifyActorRunId = 'none';
  let apifyFilteredOutTotal = 0;
  let apifyEntityGateIncludedTotal = 0;
  let apifyEntityGateFilteredTotal = 0;
  let apifyFilterNoNamedEntity = 0;
  let apifyFilterRecruitmentOrJobPost = 0;
  let apifyFilterGenericMcpOnly = 0;
  let apifyFilterGenericAiAgentOnly = 0;
  let apifyFilterGenericThreadbait = 0;
  let apifyFilterAmbiguousWithoutEntity = 0;
  let apifyFilterEmptyText = 0;
  let apifyFilterDuplicate = 0;
  let apifySampleIncluded: ApifyIncludedPostSample[] = [];
  let apifySampleFilteredOut: ApifyFilteredPostSample[] = [];
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
  let identityLinkageStatus = 'Idle';
  let identityResolvedCount = 0;
  let identityMissingAliasCount = 0;
  let identityAmbiguousCount = 0;
  let identitySkippedCount = 0;
  let identityErrorCount = 0;
  let identityLinkagePreview: MentionIdentityLinkagePreview[] = [];
  let isLinkingIdentities = false;
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
  let canonicalWeeklyStatus = 'Idle';
  let canonicalWeeklyRows = 0;
  let canonicalResolvedEntities = 0;
  let canonicalUnresolvedSkipped = 0;
  let canonicalAmbiguousSkipped = 0;
  let canonicalMissingAliasSkipped = 0;
  let canonicalSkippedMentions = 0;
  let canonicalTopIndonesia: WeeklyEntityMetric[] = [];
  let canonicalTopGlobal: WeeklyEntityMetric[] = [];
  let canonicalWeeklyErrors: string[] = [];
  let isAggregatingCanonicalWeeklyMetrics = false;
  let markdownExportStatus = 'Idle';
  let csvExportStatus = 'Idle';
  let markdownExportPath = '';
  let csvExportPath = '';
  let markdownExportPreview = '';
  let csvExportPreview = '';
  let isExportingMarkdown = false;
  let isExportingCsv = false;
  let demoStatus = 'Idle';
  let isRunningFullSampleDemo = false;
  let isRunningFullRealFlow = false;
  let fullFlowStep = '';
  let candidateReviewStatus = 'Idle';
  let candidateEntities: CandidateEntityReview[] = [];
  let pendingCandidateCount = 0;
  let approvedCandidateCount = 0;
  let ignoredCandidateCount = 0;
  let entityReviewDecisions: EntityReviewDecision[] = [];
  let approvedDecisionCount = 0;
  let ignoredDecisionCount = 0;
  let isLoadingCandidates = false;
  let activeCandidateAction = '';
  let activeCandidateActionType = '';
  let resetStatus = 'Idle';
  let isResettingLocalData = false;

  const candidateCategoryOptions = [
    'coding_agent',
    'coding_assistant',
    'generic_agent_framework',
    'skill_or_mode',
    'mcp_or_connector',
    'registry_or_discovery',
    'app_builder',
    'unknown',
  ];

  const externalRelationshipOptions = [
    'same_entity',
    'child_resource',
    'related_entity',
    'mentioned_entity',
  ];

  type StepStatus =
    | 'Not started'
    | 'Ready'
    | 'Running'
    | 'Completed'
    | 'Needs attention'
    | 'Error';

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
    run_id: string;
    seed_group: string;
    max_per_seed: number;
    mode: string;
    started_at: string;
    finished_at: string;
    duration_ms: number;
    seeds_processed: number;
    fetched_total: number;
    detail_fetched_total: number;
    detail_failed_total: number;
    text_missing_total: number;
    saved_total: number;
    duplicates_skipped: number;
    zero_result_seeds: number;
    failed_seeds: number;
    id_only_results_count: number;
    permission_limited_hint: boolean;
    last_successful_seed: string;
    last_error_summary: string;
    seed_results: DiscoverySeedDiagnostics[];
    errors: string[];
    message: string;
  };

  type DiscoverySeedDiagnostics = {
    seed_keyword: string;
    region_group: string;
    search_status: string;
    fetched_count: number;
    saved_count: number;
    duplicate_count: number;
    detail_failed_count: number;
    text_missing_count: number;
    pages_fetched: number;
    pagination_stopped_reason: string;
    error_code: string;
    error_message_safe: string;
  };

  type DiscoverySeedTestResult = {
    seed_keyword: string;
    status: string;
    fetched_count: number;
    detail_fetched_count: number;
    text_available_count: number;
    sample_text_snippet: string;
    error_summary: string;
  };

  type ApifyDiscoveryResult = {
    mode: string;
    actor_id: string;
    actor_run_id: string;
    fetched_total: number;
    filtered_out_total: number;
    saved_total: number;
    duplicates_skipped: number;
    entity_gate_included_total: number;
    entity_gate_filtered_total: number;
    filtered_out_by_reason: ApifyFilterReasons;
    sample_filtered_out: ApifyFilteredPostSample[];
    sample_included: ApifyIncludedPostSample[];
    safe_error_summary: string;
  };

  type ApifyCacheImportResult = {
    imported_count: number;
    cache_path: string;
    message: string;
  };

  type ExplainXImportPreview = {
    source_record_key: string;
    name: string;
    category: string;
    tags: string[];
    identity_status: string;
    matched_canonical_entity: string;
    reason: string;
  };

  type ExplainXImportResult = {
    imported: number;
    inserted: number;
    updated: number;
    skipped: number;
    invalid: number;
    linked_exact_alias: number;
    review_needed: number;
    unlinked: number;
    ingestion_batch_id: string;
    message: string;
    sample_records: ExplainXImportPreview[];
  };

  type ExternalIdentityReviewItem = {
    link_id: string;
    source: string;
    source_record_id: string;
    source_record_key: string;
    source_record_name: string;
    source_record_url: string | null;
    source_record_description: string | null;
    canonical_entity_id: string;
    canonical_entity_name: string;
    canonical_entity_type: string;
    relationship_type: string;
    current_status: string;
    match_method: string;
    match_confidence: number | null;
    match_reason: string;
    evidence: string | null;
    created_at: string;
    updated_at: string;
    latest_decision: string | null;
    latest_reviewer: string | null;
    latest_review_note: string | null;
    latest_reviewed_at: string | null;
    draft_relationship_type?: string;
    draft_reviewer?: string;
    draft_note?: string;
  };

  type ExternalIdentityReviewListResult = {
    total: number;
    pending_count: number;
    approved_count: number;
    rejected_count: number;
    ambiguous_count: number;
    items: ExternalIdentityReviewItem[];
    message: string;
  };

  type ExternalIdentityReviewHistoryEntry = {
    review_id: string;
    link_id: string;
    source_record_id: string;
    canonical_entity_id: string;
    previous_state: string;
    decision: string;
    proposed_relationship_type: string;
    match_method: string;
    match_confidence: number | null;
    evidence: string | null;
    review_note: string | null;
    reviewer: string;
    reviewed_at: string;
  };

  type ExternalIdentityReviewHistoryResult = {
    link_id: string;
    total: number;
    history: ExternalIdentityReviewHistoryEntry[];
    message: string;
  };

  type ExternalIdentityReviewSubmissionResult = {
    item: ExternalIdentityReviewItem;
    history_count: number;
    message: string;
  };

  type ApifyFilterReasons = {
    no_named_entity: number;
    recruitment_or_job_post: number;
    generic_mcp_only: number;
    generic_ai_agent_only: number;
    generic_threadbait: number;
    ambiguous_without_entity: number;
    empty_text: number;
    duplicate: number;
  };

  type ApifyIncludedPostSample = {
    post_id: string;
    text_snippet: string;
    source_seed_keyword: string;
    permalink: string;
    detected_entities: string[];
  };

  type ApifyFilteredPostSample = {
    text_snippet: string;
    reason: string;
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

  type MentionIdentityLinkagePreview = {
    mention_name: string;
    resolution_status: string;
    canonical_entity_name: string;
    reason: string;
  };

  type MentionIdentityLinkageResult = {
    resolved_count: number;
    missing_alias_count: number;
    ambiguous_count: number;
    skipped_count: number;
    error_count: number;
    message: string;
    preview: MentionIdentityLinkagePreview[];
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

  type WeeklyEntityMetric = {
    rank: number;
    id: string;
    week_start: string;
    week_end: string;
    entity_id: string;
    canonical_name: string;
    entity_type: string;
    region: string;
    mention_count: number;
    positive_count: number;
    neutral_count: number;
    negative_count: number;
    mixed_count: number;
    cost_positive_count: number;
    cost_negative_boros_count: number;
    cost_mixed_count: number;
    cost_not_mentioned_count: number;
    source_count: number;
    first_seen_at: string;
    last_seen_at: string;
    trend_score: number;
  };

  type WeeklyEntityAggregationResult = {
    canonical_rows_generated: number;
    resolved_entities_included: number;
    unresolved_mentions_skipped: number;
    ambiguous_mentions_skipped: number;
    missing_alias_mentions_skipped: number;
    skipped_mentions_skipped: number;
    top_indonesia: WeeklyEntityMetric[];
    top_global: WeeklyEntityMetric[];
    errors: string[];
    message: string;
  };

  type ReportExportResult = {
    file_path: string;
    rows_exported: number;
    message: string;
    preview: string;
  };

  type CandidateEntityReview = {
    candidate_name: string;
    mention_count: number;
    first_seen: string;
    latest_seen: string;
    sample_snippets: string[];
    current_status: string;
    reviewed_as: string;
    reviewed_category: string;
    draft_reviewed_as?: string;
    draft_reviewed_category?: string;
    draft_note?: string;
  };

  type CandidateEntityListResult = {
    total_candidates: number;
    pending_count: number;
    approved_count: number;
    ignored_count: number;
    candidates: CandidateEntityReview[];
    message: string;
  };

  type EntityReviewDecision = {
    id: string;
    candidate_name: string;
    normalized_name: string;
    category: string;
    status: string;
    note: string;
    created_at: string;
    updated_at: string;
  };

  type EntityReviewDecisionListResult = {
    total_decisions: number;
    approved_count: number;
    ignored_count: number;
    decisions: EntityReviewDecision[];
    message: string;
  };

  type CandidateReviewActionResult = {
    candidate_name: string;
    status: string;
    updated_mentions_count: number;
    message: string;
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
      await loadCandidateEntities();
      await loadExternalIdentityReviewItems();
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

  function latestExportPath() {
    return markdownExportPath || csvExportPath || 'None yet';
  }

  function stepStatus(step: string): StepStatus {
    if (step === 'discovery') {
      if (
        isRunningDiscovery ||
        isReplayingApify ||
        isImportingApifyCache ||
        isCollecting ||
        isImportingSamples
      )
        return 'Running';
      if (isErrorStatus(discoveryStatus) || isErrorStatus(collectStatus)) return 'Error';
      if (discoveryTextMissingTotal > 0) return 'Needs attention';
      if (rawPostsCount > 0) return 'Completed';
      return threadsTokenConfigured || envFileLoaded ? 'Ready' : 'Not started';
    }

    if (step === 'entities') {
      if (isDetecting) return 'Running';
      if (isErrorStatus(detectStatus)) return 'Error';
      if (mentionsFound > 0 || savedMentions > 0) return 'Completed';
      return rawPostsCount > 0 ? 'Ready' : 'Not started';
    }

    if (step === 'candidates') {
      if (isLoadingCandidates || activeCandidateAction) return 'Running';
      if (isErrorStatus(candidateReviewStatus)) return 'Error';
      if (pendingCandidateCount > 0) return 'Needs attention';
      if (approvedDecisionCount > 0 || ignoredDecisionCount > 0 || approvedCandidateCount > 0) {
        return 'Completed';
      }
      return savedMentions > 0 ? 'Ready' : 'Not started';
    }

    if (step === 'classification') {
      if (isClassifyingRegions || isClassifyingSentiments || isClassifyingCostSignals) {
        return 'Running';
      }
      if (isErrorStatus(regionStatus) || isErrorStatus(sentimentStatus) || isErrorStatus(costStatus)) {
        return 'Error';
      }
      if (regionUpdatedMentions > 0 && sentimentUpdatedMentions > 0 && costUpdatedMentions > 0) {
        return 'Completed';
      }
      return savedMentions > 0 ? 'Ready' : 'Not started';
    }

    if (step === 'weekly') {
      if (isAggregatingWeeklyMetrics) return 'Running';
      if (isErrorStatus(weeklyStatus)) return 'Error';
      if (weeklyMetricsCount > 0) return 'Completed';
      return costUpdatedMentions > 0 ? 'Ready' : 'Not started';
    }

    if (step === 'export') {
      if (isExportingMarkdown || isExportingCsv) return 'Running';
      if (isErrorStatus(markdownExportStatus) || isErrorStatus(csvExportStatus)) return 'Error';
      if (markdownExportPath || csvExportPath) return 'Completed';
      return weeklyMetricsCount > 0 ? 'Ready' : 'Not started';
    }

    return 'Not started';
  }

  function statusClass(status: StepStatus) {
    return `step-status ${status.toLowerCase().replace(/\s+/g, '-')}`;
  }

  function isErrorStatus(status: string) {
    return status.toLowerCase().startsWith('error');
  }

  function isFullFlowRunning() {
    return isRunningFullSampleDemo || isRunningFullRealFlow;
  }

  function friendlyError(error: unknown) {
    const raw = String(error);
    if (raw.includes('code=10') || raw.includes('Application does not have permission')) {
      return 'Threads keyword search permission missing. Add threads_keyword_search in Meta Developer Permissions, regenerate the token, and try again.';
    }
    if (
      raw.includes('agent_mentions_compatible') ||
      raw.toLowerCase().includes('legacy local duckdb metadata detected')
    ) {
      return 'Legacy local DuckDB metadata detected. Stop the app and remove data/app.duckdb only if you want a clean local demo database.';
    }
    if (raw.includes('Catalog Error')) {
      return 'Local DuckDB schema needs attention. Restart the app so schema initialization can run, then try again.';
    }
    if (raw.toLowerCase().includes('apify actor is still running or timed out')) {
      return 'Apify actor is still running or timed out. Try again with fewer seeds or wait longer.';
    }
    if (raw.includes('Live Apify crawl is disabled to protect trial usage')) {
      return 'Live Apify crawl is disabled to protect trial usage. Use replay mode or enable APIFY_LIVE_CRAWL_ENABLED=true.';
    }
    if (raw.includes('Live Apify crawl session limit reached')) {
      return raw.replace(/^Error:\s*/, '');
    }
    if (raw.toLowerCase().includes('text is unavailable') || raw.toLowerCase().includes('text_missing')) {
      return 'Post detail was fetched, but text is unavailable for one or more posts.';
    }
    return raw;
  }

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
      collectStatus =
        result.fetched_count === 0
          ? `Keyword search completed for "${nextKeyword}", but no posts were returned. Try a broader keyword or confirm Threads search scope.`
          : result.message;
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

  async function resetLocalPipelineData() {
    if (isResettingLocalData || isFullFlowRunning()) return;

    isResettingLocalData = true;
    resetStatus = 'Clearing local demo data...';

    try {
      const result = await invoke<string>('reset_local_pipeline_data');
      resetStatus = result;
      rawPostsCount = 0;
      fetchedCount = 0;
      savedCount = 0;
      sampleLoadedCount = 0;
      sampleSavedCount = 0;
      analyzedPosts = 0;
      mentionsFound = 0;
      savedMentions = 0;
      detectionPreview = [];
      identityLinkageStatus = 'Idle';
      identityResolvedCount = 0;
      identityMissingAliasCount = 0;
      identityAmbiguousCount = 0;
      identitySkippedCount = 0;
      identityErrorCount = 0;
      identityLinkagePreview = [];
      regionPostsAnalyzed = 0;
      indonesiaCount = 0;
      globalCount = 0;
      unknownCount = 0;
      regionUpdatedMentions = 0;
      sentimentMentionsAnalyzed = 0;
      positiveCount = 0;
      neutralCount = 0;
      negativeCount = 0;
      mixedCount = 0;
      sentimentUpdatedMentions = 0;
      costMentionsAnalyzed = 0;
      notMentionedCount = 0;
      costPositiveCount = 0;
      costNegativeBorosCount = 0;
      costMixedCount = 0;
      costUpdatedMentions = 0;
      weeklyMetricsCount = 0;
      weeklyIndonesiaCount = 0;
      weeklyGlobalCount = 0;
      weeklyUnknownCount = 0;
      topIndonesia = [];
      topGlobal = [];
      topUnknown = [];
      canonicalWeeklyStatus = 'Idle';
      canonicalWeeklyRows = 0;
      canonicalResolvedEntities = 0;
      canonicalUnresolvedSkipped = 0;
      canonicalAmbiguousSkipped = 0;
      canonicalMissingAliasSkipped = 0;
      canonicalSkippedMentions = 0;
      canonicalTopIndonesia = [];
      canonicalTopGlobal = [];
      canonicalWeeklyErrors = [];
      resetDiscoveryDiagnostics();
      await loadCandidateEntities();
    } catch (error) {
      resetStatus = `error: ${friendlyError(error)}`;
    } finally {
      isResettingLocalData = false;
    }
  }

  function resetDiscoveryDiagnostics() {
    discoverySeedsProcessed = 0;
    discoveryFetchedTotal = 0;
    discoveryDetailFetchedTotal = 0;
    discoveryDetailFailedTotal = 0;
    discoveryIdOnlyResultsCount = 0;
    discoveryTextMissingTotal = 0;
    discoverySavedTotal = 0;
    discoveryDuplicatesSkipped = 0;
    discoveryFailedSeeds = 0;
    discoveryZeroResultSeeds = 0;
    discoveryPermissionLimitedHint = false;
    discoveryLastSuccessfulSeed = 'none';
    discoveryLastErrorSummary = 'none';
    discoveryMode = 'none';
    discoveryRunId = 'none';
    discoveryStartedAt = 'none';
    discoveryFinishedAt = 'none';
    discoveryDurationMs = 0;
    discoveryErrors = [];
    discoverySeedResults = [];
    apifyActorId = 'none';
    apifyActorRunId = 'none';
    apifyFilteredOutTotal = 0;
    apifyEntityGateIncludedTotal = 0;
    apifyEntityGateFilteredTotal = 0;
    apifyFilterNoNamedEntity = 0;
    apifyFilterRecruitmentOrJobPost = 0;
    apifyFilterGenericMcpOnly = 0;
    apifyFilterGenericAiAgentOnly = 0;
    apifyFilterGenericThreadbait = 0;
    apifyFilterAmbiguousWithoutEntity = 0;
    apifyFilterEmptyText = 0;
    apifyFilterDuplicate = 0;
    apifySampleIncluded = [];
    apifySampleFilteredOut = [];
  }

  async function loadCandidateEntities() {
    if (isLoadingCandidates) return;

    isLoadingCandidates = true;
    try {
      const result = await invoke<CandidateEntityListResult>('list_candidate_entities');
      const decisions = await invoke<EntityReviewDecisionListResult>('list_entity_review_decisions');
      pendingCandidateCount = result.pending_count;
      approvedCandidateCount = result.approved_count;
      ignoredCandidateCount = result.ignored_count;
      approvedDecisionCount = decisions.approved_count;
      ignoredDecisionCount = decisions.ignored_count;
      entityReviewDecisions = decisions.decisions;
      candidateEntities = result.candidates.map((candidate) => ({
        ...candidate,
        draft_reviewed_as: candidate.reviewed_as || candidate.candidate_name,
        draft_reviewed_category: candidate.reviewed_category || 'coding_agent',
        draft_note: '',
      }));
      candidateReviewStatus = `${result.message} ${decisions.message}`;
    } catch (error) {
      candidateReviewStatus = `error: ${friendlyError(error)}`;
    } finally {
      isLoadingCandidates = false;
    }
  }

  async function approveCandidate(candidate: CandidateEntityReview) {
    if (activeCandidateAction) return;

    const reviewedAs = (candidate.draft_reviewed_as || candidate.candidate_name).trim();
    const reviewedCategory = (candidate.draft_reviewed_category || 'coding_agent').trim();
    if (!reviewedAs || !reviewedCategory) return;

    activeCandidateAction = candidate.candidate_name;
    activeCandidateActionType = 'approve';
    candidateReviewStatus = `Approving ${candidate.candidate_name}...`;

    try {
      const result = await invoke<CandidateReviewActionResult>('approve_candidate_entity', {
        candidateName: candidate.candidate_name,
        reviewedAs,
        reviewedCategory,
        note: candidate.draft_note?.trim() || null,
      });
      candidateReviewStatus = result.message;
      await loadCandidateEntities();
    } catch (error) {
      candidateReviewStatus = `error: ${friendlyError(error)}`;
    } finally {
      activeCandidateAction = '';
      activeCandidateActionType = '';
    }
  }

  async function ignoreCandidate(candidate: CandidateEntityReview) {
    if (activeCandidateAction) return;

    activeCandidateAction = candidate.candidate_name;
    activeCandidateActionType = 'ignore';
    candidateReviewStatus = `Ignoring ${candidate.candidate_name}...`;

    try {
      const result = await invoke<CandidateReviewActionResult>('ignore_candidate_entity', {
        candidateName: candidate.candidate_name,
        note: candidate.draft_note?.trim() || null,
      });
      candidateReviewStatus = result.message;
      await loadCandidateEntities();
    } catch (error) {
      candidateReviewStatus = `error: ${friendlyError(error)}`;
    } finally {
      activeCandidateAction = '';
      activeCandidateActionType = '';
    }
  }

  async function resetCandidate(candidate: CandidateEntityReview) {
    if (activeCandidateAction) return;

    activeCandidateAction = candidate.candidate_name;
    activeCandidateActionType = 'reset';
    candidateReviewStatus = `Resetting ${candidate.candidate_name}...`;

    try {
      const result = await invoke<CandidateReviewActionResult>('reset_candidate_review', {
        candidateName: candidate.candidate_name,
      });
      candidateReviewStatus = result.message;
      await loadCandidateEntities();
    } catch (error) {
      candidateReviewStatus = `error: ${friendlyError(error)}`;
    } finally {
      activeCandidateAction = '';
      activeCandidateActionType = '';
    }
  }

  async function runDiscoveryCrawl() {
    if (isRunningDiscovery) return;

    isRunningDiscovery = true;
    discoveryStatus =
      discoverySource === 'apify_threads_scraper'
        ? 'Running Apify fallback discovery crawl...'
        : discoverySource === 'sample_mock'
          ? 'Importing sample/mock discovery posts...'
          : 'Running AI Agent discovery crawl...';
    resetDiscoveryDiagnostics();

    try {
      if (discoverySource === 'apify_threads_scraper') {
        const seeds = apifySeedsText
          .split('\n')
          .map((seed) => seed.trim())
          .filter(Boolean);
        const apifyMaxPosts = Math.max(10, Number(discoveryMaxPerSeed) || 10);
        if (apifyMaxPosts !== discoveryMaxPerSeed) {
          discoveryMaxPerSeed = apifyMaxPosts;
          discoveryStatus = 'Apify actor requires at least 10 max posts. Using 10.';
        }
        const result = await invoke<ApifyDiscoveryResult>('run_apify_discovery_crawl', {
          seeds,
          maxPerSeed: apifyMaxPosts,
        });
        applyApifyResult(result, seeds.length, 'Apify live crawl');
        await refreshRawPostsCount();
        return;
      }

      if (discoverySource === 'sample_mock') {
        const result = await invoke<SampleThreadsImportResult>('import_sample_threads_posts');
        sampleLoadedCount = result.loaded_count;
        sampleSavedCount = result.saved_count;
        discoveryMode = 'sample_mock';
        discoveryFetchedTotal = result.loaded_count;
        discoverySavedTotal = result.saved_count;
        discoveryStatus = result.message;
        await refreshRawPostsCount();
        return;
      }

      const result = await invoke<DiscoveryCrawlResult>('run_discovery_crawl', {
        regionSeedGroup: discoverySeedGroup,
        maxPerSeed: discoveryMaxPerSeed,
        dryRun: false,
      });
      discoverySeedsProcessed = result.seeds_processed;
      discoveryFetchedTotal = result.fetched_total;
      discoveryDetailFetchedTotal = result.detail_fetched_total;
      discoveryDetailFailedTotal = result.detail_failed_total;
      discoveryIdOnlyResultsCount = result.id_only_results_count;
      discoveryTextMissingTotal = result.text_missing_total;
      discoverySavedTotal = result.saved_total;
      discoveryDuplicatesSkipped = result.duplicates_skipped;
      discoveryFailedSeeds = result.failed_seeds;
      discoveryZeroResultSeeds = result.zero_result_seeds;
      discoveryPermissionLimitedHint = result.permission_limited_hint;
      discoveryLastSuccessfulSeed = result.last_successful_seed || 'none';
      discoveryLastErrorSummary = result.last_error_summary || 'none';
      discoveryMode = result.mode;
      discoveryRunId = result.run_id;
      discoveryStartedAt = result.started_at;
      discoveryFinishedAt = result.finished_at;
      discoveryDurationMs = result.duration_ms;
      discoverySeedResults = result.seed_results;
      discoveryErrors = result.errors;
      discoveryStatus =
        result.permission_limited_hint
          ? `${result.message} Crawler may be limited to authenticated user posts until threads_keyword_search is approved for public search.`
          : result.fetched_total === 0
          ? 'Discovery search completed, but no posts were returned. Tester/public search scope or seed keywords may need adjustment.'
          : result.text_missing_total > 0
            ? `${result.message} ${result.text_missing_total} posts had no text available after detail fetch.`
            : result.message;
      await refreshRawPostsCount();
    } catch (error) {
      discoveryStatus = `error: ${friendlyError(error)}`;
    } finally {
      isRunningDiscovery = false;
    }
  }

  function applyApifyResult(
    result: ApifyDiscoveryResult,
    seedsProcessed: number,
    label: string,
  ) {
    discoveryMode = result.mode;
    discoveryFetchedTotal = result.fetched_total;
    discoverySavedTotal = result.saved_total;
    discoveryDuplicatesSkipped = result.duplicates_skipped;
    discoverySeedsProcessed = seedsProcessed;
    discoveryStatus = `${label} kept ${result.saved_total} posts with named entities and filtered ${result.filtered_out_total} of ${result.fetched_total} fetched items.${result.safe_error_summary ? ` ${result.safe_error_summary}` : ''}`;
    discoveryLastErrorSummary = result.safe_error_summary || 'none';
    apifyActorId = result.actor_id;
    apifyActorRunId = result.actor_run_id;
    apifyFilteredOutTotal = result.filtered_out_total;
    apifyEntityGateIncludedTotal = result.entity_gate_included_total;
    apifyEntityGateFilteredTotal = result.entity_gate_filtered_total;
    apifyFilterNoNamedEntity = result.filtered_out_by_reason.no_named_entity;
    apifyFilterRecruitmentOrJobPost = result.filtered_out_by_reason.recruitment_or_job_post;
    apifyFilterGenericMcpOnly = result.filtered_out_by_reason.generic_mcp_only;
    apifyFilterGenericAiAgentOnly = result.filtered_out_by_reason.generic_ai_agent_only;
    apifyFilterGenericThreadbait = result.filtered_out_by_reason.generic_threadbait;
    apifyFilterAmbiguousWithoutEntity =
      result.filtered_out_by_reason.ambiguous_without_entity;
    apifyFilterEmptyText = result.filtered_out_by_reason.empty_text;
    apifyFilterDuplicate = result.filtered_out_by_reason.duplicate;
    apifySampleIncluded = result.sample_included;
    apifySampleFilteredOut = result.sample_filtered_out;
  }

  async function replayLastApifyCrawl() {
    if (
      isReplayingApify ||
      isImportingApifyCache ||
      isRunningDiscovery ||
      isFullFlowRunning()
    ) return;

    isReplayingApify = true;
    discoveryStatus = 'Reprocessing cached Apify output...';
    resetDiscoveryDiagnostics();

    try {
      const result = await invoke<ApifyDiscoveryResult>('replay_last_apify_crawl');
      applyApifyResult(result, 0, 'Apify cache replay');
      await refreshRawPostsCount();
    } catch (error) {
      discoveryStatus = `error: ${friendlyError(error)}`;
    } finally {
      isReplayingApify = false;
    }
  }

  async function importApifyDatasetCache() {
    const filePath = apifyDatasetFilePath.trim();
    if (
      !filePath ||
      isImportingApifyCache ||
      isReplayingApify ||
      isRunningDiscovery ||
      isFullFlowRunning()
    ) return;

    isImportingApifyCache = true;
    discoveryStatus = 'Importing Apify dataset into the local cache...';

    try {
      const result = await invoke<ApifyCacheImportResult>('import_apify_dataset_cache', {
        filePath,
      });
      discoveryStatus = `${result.message} Cache: ${result.cache_path}`;
      discoveryLastErrorSummary = 'none';
      apifyActorId = 'manual_import';
      apifyActorRunId = 'manual_import';
    } catch (error) {
      discoveryStatus = `error: ${friendlyError(error)}`;
    } finally {
      isImportingApifyCache = false;
    }
  }

  async function importExplainXRecords() {
    const filePath = explainxFilePath.trim();
    if (!filePath || isImportingExplainX || isFullFlowRunning()) return;

    isImportingExplainX = true;
    explainxImportStatus = 'Importing ExplainX records...';
    explainxImportPreview = [];

    try {
      const result = await invoke<ExplainXImportResult>('import_explainx_records', {
        filePath,
      });
      explainxImported = result.imported;
      explainxInserted = result.inserted;
      explainxUpdated = result.updated;
      explainxSkipped = result.skipped;
      explainxInvalid = result.invalid;
      explainxLinkedExactAlias = result.linked_exact_alias;
      explainxReviewNeeded = result.review_needed;
      explainxUnlinked = result.unlinked;
      explainxImportPreview = result.sample_records;
      explainxImportStatus = result.message;
      await loadExternalIdentityReviewItems();
    } catch (error) {
      explainxImportStatus = `error: ${friendlyError(error)}`;
    } finally {
      isImportingExplainX = false;
    }
  }

  async function loadExternalIdentityReviewItems() {
    if (isLoadingExternalIdentityReviews) return false;

    isLoadingExternalIdentityReviews = true;
    externalIdentityReviewStatus = 'Loading ExplainX identity links...';
    try {
      const result = await invoke<ExternalIdentityReviewListResult>(
        'list_external_identity_review_items',
      );
      externalIdentityReviewPending = result.pending_count;
      externalIdentityReviewApproved = result.approved_count;
      externalIdentityReviewRejected = result.rejected_count;
      externalIdentityReviewAmbiguous = result.ambiguous_count;
      externalIdentityReviewItems = result.items.map((item) => ({
        ...item,
        draft_relationship_type: item.relationship_type,
        draft_reviewer: '',
        draft_note: '',
      }));
      externalIdentityReviewStatus = result.message;
      return true;
    } catch (error) {
      externalIdentityReviewStatus = `error: ${friendlyError(error)}`;
      return false;
    } finally {
      isLoadingExternalIdentityReviews = false;
    }
  }

  async function submitExternalIdentityReview(
    item: ExternalIdentityReviewItem,
    decision: 'approved' | 'rejected' | 'ambiguous',
  ) {
    if (activeExternalIdentityReviewAction) return;

    const reviewer = item.draft_reviewer?.trim() || '';
    const relationshipType = item.draft_relationship_type || item.relationship_type;
    if (!reviewer) {
      externalIdentityReviewStatus = 'error: Reviewer is required before recording a decision.';
      return;
    }

    activeExternalIdentityReviewAction = item.link_id;
    activeExternalIdentityReviewDecision = decision;
    externalIdentityReviewStatus = `Recording ${decision} decision for ${item.source_record_name}...`;
    try {
      await invoke<ExternalIdentityReviewSubmissionResult>(
        'submit_external_identity_review',
        {
          linkId: item.link_id,
          relationshipType,
          decision,
          reviewer,
          evidenceNote: item.draft_note?.trim() || null,
        },
      );
      const listRefreshed = await loadExternalIdentityReviewItems();
      await loadExternalIdentityReviewHistory(item.link_id);
      if (listRefreshed) {
        externalIdentityReviewStatus = 'Review saved successfully. List refreshed.';
      }
    } catch (error) {
      externalIdentityReviewStatus = `error: ${friendlyError(error)}`;
    } finally {
      activeExternalIdentityReviewAction = '';
      activeExternalIdentityReviewDecision = '';
    }
  }

  async function loadExternalIdentityReviewHistory(linkId: string) {
    externalIdentityReviewHistoryLinkId = linkId;
    externalIdentityReviewHistoryStatus = 'Loading review history...';
    try {
      const result = await invoke<ExternalIdentityReviewHistoryResult>(
        'get_external_identity_review_history',
        { linkId },
      );
      externalIdentityReviewHistory = result.history;
      externalIdentityReviewHistoryStatus = result.message;
    } catch (error) {
      externalIdentityReviewHistory = [];
      externalIdentityReviewHistoryStatus = `error: ${friendlyError(error)}`;
    }
  }

  function externalReviewStateLabel(state: string | null) {
    switch (state) {
      case 'initial_state':
        return 'Initial state';
      case 'pending':
      case 'review_needed':
        return 'Review needed';
      case 'approved':
        return 'Approved';
      case 'rejected':
        return 'Rejected';
      case 'ambiguous':
        return 'Marked ambiguous';
      case null:
        return 'No reviewer decision yet';
      default:
        return state;
    }
  }

  async function testDiscoverySeed() {
    const keyword = discoverySeedTestKeyword.trim();
    if (!keyword || isTestingDiscoverySeed || isReplayingApify || isFullFlowRunning()) return;

    isTestingDiscoverySeed = true;
    discoverySeedTestStatus = `Testing seed "${keyword}"...`;
    discoverySeedTestResult = null;

    try {
      if (discoverySource === 'apify_threads_scraper') {
        const result = await invoke<ApifyDiscoveryResult>('run_apify_discovery_crawl', {
          seeds: [keyword],
          maxPerSeed: 10,
        });
        applyApifyResult(result, 1, 'Apify single-seed debug run');
        discoverySeedTestResult = {
          seed_keyword: keyword,
          status: result.fetched_total === 0 ? 'zero_result' : 'success',
          fetched_count: result.fetched_total,
          detail_fetched_count: result.fetched_total,
          text_available_count:
            result.fetched_total - result.filtered_out_by_reason.empty_text,
          sample_text_snippet: result.sample_included[0]?.text_snippet || '',
          error_summary: result.safe_error_summary,
        };
        discoverySeedTestStatus = `Apify seed test completed. This consumed one live actor run.`;
        await refreshRawPostsCount();
        return;
      }

      const result = await invoke<DiscoverySeedTestResult>('test_discovery_seed', { keyword });
      discoverySeedTestResult = result;
      discoverySeedTestStatus =
        result.status === 'zero_result'
          ? 'Keyword search succeeded but no posts were returned for this seed.'
          : result.status === 'permission_error'
            ? 'Token is valid, but keyword search is not authorized. Check token scope and app approval.'
            : `Seed test ${result.status}.`;
    } catch (error) {
      discoverySeedTestStatus = `error: ${friendlyError(error)}`;
    } finally {
      isTestingDiscoverySeed = false;
    }
  }

  function parseApiError(error: unknown) {
    const raw = String(error);
    const code = raw.match(/\bcode=([^ ]+)/)?.[1] ?? 'none';
    const type = raw.match(/\btype=([^ ]+)/)?.[1] ?? 'none';
    const message = raw.match(/\bmessage=(.*)$/)?.[1]?.trim() ?? raw;
    const friendly = friendlyError(raw.split(' code=')[0]);

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
      await loadCandidateEntities();
    } catch (error) {
      detectStatus = `error: ${friendlyError(error)}`;
    } finally {
      isDetecting = false;
    }
  }

  async function linkAgentMentionsToEntities() {
    if (isLinkingIdentities) return;

    isLinkingIdentities = true;
    identityLinkageStatus = 'Linking mentions to canonical entities...';
    identityResolvedCount = 0;
    identityMissingAliasCount = 0;
    identityAmbiguousCount = 0;
    identitySkippedCount = 0;
    identityErrorCount = 0;
    identityLinkagePreview = [];

    try {
      const result = await invoke<MentionIdentityLinkageResult>(
        'link_agent_mentions_to_entities',
      );
      identityResolvedCount = result.resolved_count;
      identityMissingAliasCount = result.missing_alias_count;
      identityAmbiguousCount = result.ambiguous_count;
      identitySkippedCount = result.skipped_count;
      identityErrorCount = result.error_count;
      identityLinkagePreview = result.preview;
      identityLinkageStatus = result.message;
    } catch (error) {
      identityLinkageStatus = `error: ${friendlyError(error)}`;
    } finally {
      isLinkingIdentities = false;
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
      regionStatus = `error: ${friendlyError(error)}`;
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
      sentimentStatus = `error: ${friendlyError(error)}`;
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
      costStatus = `error: ${friendlyError(error)}`;
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
      weeklyStatus = `error: ${friendlyError(error)}`;
    } finally {
      isAggregatingWeeklyMetrics = false;
    }
  }

  async function aggregateCanonicalWeeklyMetrics() {
    if (isAggregatingCanonicalWeeklyMetrics) return;

    isAggregatingCanonicalWeeklyMetrics = true;
    canonicalWeeklyStatus = 'Aggregating canonical weekly metrics...';
    canonicalWeeklyRows = 0;
    canonicalResolvedEntities = 0;
    canonicalUnresolvedSkipped = 0;
    canonicalAmbiguousSkipped = 0;
    canonicalMissingAliasSkipped = 0;
    canonicalSkippedMentions = 0;
    canonicalTopIndonesia = [];
    canonicalTopGlobal = [];
    canonicalWeeklyErrors = [];

    try {
      const result = await invoke<WeeklyEntityAggregationResult>(
        'aggregate_weekly_entity_metrics',
      );
      canonicalWeeklyRows = result.canonical_rows_generated;
      canonicalResolvedEntities = result.resolved_entities_included;
      canonicalUnresolvedSkipped = result.unresolved_mentions_skipped;
      canonicalAmbiguousSkipped = result.ambiguous_mentions_skipped;
      canonicalMissingAliasSkipped = result.missing_alias_mentions_skipped;
      canonicalSkippedMentions = result.skipped_mentions_skipped;
      canonicalTopIndonesia = result.top_indonesia;
      canonicalTopGlobal = result.top_global;
      canonicalWeeklyErrors = result.errors;
      canonicalWeeklyStatus = result.message;
    } catch (error) {
      canonicalWeeklyStatus = `error: ${friendlyError(error)}`;
    } finally {
      isAggregatingCanonicalWeeklyMetrics = false;
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
      markdownExportStatus = `error: ${friendlyError(error)}`;
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
      csvExportStatus = `error: ${friendlyError(error)}`;
    } finally {
      isExportingCsv = false;
    }
  }

  async function runFullSampleDemo() {
    if (isRunningFullSampleDemo || isRunningFullRealFlow) return;

    isRunningFullSampleDemo = true;
    demoStatus = 'Running full sample demo...';
    fullFlowStep = 'Running step 1 of 6: Importing Sample Posts';

    try {
      await importSamplePosts();
      fullFlowStep = 'Running step 2 of 6: Detecting Agent Mentions';
      await detectAgentMentions();
      fullFlowStep = 'Running step 3 of 6: Classifying Regions';
      await classifyRegions();
      fullFlowStep = 'Running step 4 of 6: Classifying Sentiments';
      await classifySentiments();
      fullFlowStep = 'Running step 5 of 6: Classifying Cost Signals';
      await classifyCostSignals();
      fullFlowStep = 'Running step 6 of 6: Aggregating Weekly Metrics';
      await aggregateWeeklyMetrics();
      demoStatus = 'Sample demo completed. Review candidates manually, then export when ready.';
    } catch (error) {
      demoStatus = `error: ${friendlyError(error)}`;
    } finally {
      isRunningFullSampleDemo = false;
      fullFlowStep = '';
    }
  }

  async function runFullRealFlow() {
    if (isRunningFullSampleDemo || isRunningFullRealFlow) return;

    isRunningFullRealFlow = true;
    demoStatus = 'Running real discovery flow...';
    fullFlowStep = 'Running step 1 of 6: Running Discovery Crawl';

    try {
      await runDiscoveryCrawl();
      fullFlowStep = 'Running step 2 of 6: Detecting Agent Mentions';
      await detectAgentMentions();
      fullFlowStep = 'Running step 3 of 6: Classifying Regions';
      await classifyRegions();
      fullFlowStep = 'Running step 4 of 6: Classifying Sentiments';
      await classifySentiments();
      fullFlowStep = 'Running step 5 of 6: Classifying Cost Signals';
      await classifyCostSignals();
      fullFlowStep = 'Running step 6 of 6: Aggregating Weekly Metrics';
      await aggregateWeeklyMetrics();
      demoStatus = 'Real flow completed. Review pending candidates manually before relying on rankings.';
    } catch (error) {
      demoStatus = `error: ${friendlyError(error)}`;
    } finally {
      isRunningFullRealFlow = false;
      fullFlowStep = '';
    }
  }
</script>

<main class="app-shell">
  <section class="workspace">
    <div class="title-block">
      <p class="eyebrow">Local-first desktop intelligence</p>
      <h1>AI Agent Trend Radar</h1>
      <p>
        Named entity radar for discovering AI agents, skills, MCP servers, frameworks, and tools,
        then reviewing candidates, ranking weekly trends, and exporting local reports.
      </p>
    </div>

    <div class="demo-actions" aria-label="Demo actions">
      <button
        type="button"
        on:click={runFullSampleDemo}
        disabled={isRunningFullSampleDemo || isRunningFullRealFlow}
      >
        {#if isRunningFullSampleDemo}
          {@render LoadingLabel('Running Sample Demo...')}
        {:else}
          Run Full Sample Demo
        {/if}
      </button>
      <button
        type="button"
        on:click={runFullRealFlow}
        disabled={isRunningFullSampleDemo || isRunningFullRealFlow}
      >
        {#if isRunningFullRealFlow}
          {@render LoadingLabel('Running Real Flow...')}
        {:else}
          Run Full Real Flow
        {/if}
      </button>
      <span>{demoStatus}</span>
      {#if fullFlowStep}
        <span class="full-flow-step">{fullFlowStep}</span>
      {/if}
    </div>

    <section class="reset-panel" aria-label="Local demo reset">
      <div>
        <strong>Clear Local Demo Data</strong>
        <p>
          Clears raw posts, mentions, and weekly metrics for local demo reset. Candidate decisions
          are preserved.
        </p>
      </div>
      <button
        type="button"
        on:click={resetLocalPipelineData}
        disabled={isResettingLocalData || isFullFlowRunning()}
      >
        {#if isResettingLocalData}
          {@render LoadingLabel('Clearing...')}
        {:else}
          Clear Local Demo Data
        {/if}
      </button>
      <span>{resetStatus}</span>
    </section>

    <div class="status-grid" aria-label="Workflow status">
      {#each flowSteps as step}
        <article class="status-card">
          <span>{step.number}</span>
          <strong>{step.label}</strong>
          <p>{step.description}</p>
          <em class={statusClass(stepStatus(step.key))}>
            {stepStatus(step.key)}
          </em>
        </article>
      {/each}
    </div>

    <div class="summary-grid" aria-label="Dashboard summary">
      <article class="summary-card">
        <span>Raw posts</span>
        <strong>{rawPostsCount}</strong>
      </article>
      <article class="summary-card">
        <span>Mentions</span>
        <strong>{savedMentions || mentionsFound}</strong>
      </article>
      <article class="summary-card">
        <span>Pending candidates</span>
        <strong>{pendingCandidateCount}</strong>
      </article>
      <article class="summary-card">
        <span>Approved decisions</span>
        <strong>{approvedDecisionCount}</strong>
      </article>
      <article class="summary-card">
        <span>Weekly metrics rows</span>
        <strong>{weeklyMetricsCount}</strong>
      </article>
      <article class="summary-card wide">
        <span>Last export path</span>
        <strong>{latestExportPath()}</strong>
      </article>
    </div>

    <section class="collector-panel" aria-label="AI Agent discovery crawler">
      <div>
        <p class="panel-label">1. Discovery</p>
        <h2>Discover concrete named entities</h2>
        <p class="panel-note">
          Collect posts only when a concrete AI agent, skill, MCP server, framework, or tool name
          can be detected.
        </p>
      </div>

      <form on:submit|preventDefault={runDiscoveryCrawl}>
        <label for="discovery-source">Source</label>
        <div class="collector-row discovery-row">
          <select
            id="discovery-source"
            bind:value={discoverySource}
            disabled={isRunningDiscovery || isReplayingApify || isImportingApifyCache || isFullFlowRunning()}
          >
            <option value="official_threads">Official Threads API</option>
            <option value="apify_threads_scraper">Apify fallback</option>
            <option value="sample_mock">Sample/mock</option>
          </select>
        </div>

        <label for="discovery-seed-group">Seed group</label>
        <div class="collector-row discovery-row">
          <select
            id="discovery-seed-group"
            bind:value={discoverySeedGroup}
            disabled={isRunningDiscovery || isFullFlowRunning() || discoverySource !== 'official_threads'}
          >
            <option value="all">All</option>
            <option value="indonesia">Indonesia</option>
            <option value="global">Global</option>
          </select>
          <input
            type="number"
            min={discoverySource === 'apify_threads_scraper' ? 10 : 1}
            max="50"
            bind:value={discoveryMaxPerSeed}
            disabled={isRunningDiscovery || isReplayingApify || isImportingApifyCache || isFullFlowRunning()}
            aria-label="Max per seed"
          />
          <button
            type="submit"
            disabled={isRunningDiscovery || isReplayingApify || isImportingApifyCache || isFullFlowRunning()}
          >
            {#if isRunningDiscovery}
              {@render LoadingLabel('Running...')}
            {:else}
              Run Discovery Crawl
            {/if}
          </button>
        </div>

        {#if discoverySource === 'apify_threads_scraper'}
          <p class="panel-note">
            Apify fallback is experimental and should be reviewed for compliance before production
            use.
          </p>
          <p class="panel-note">
            Generic AI Agent discussion is filtered out. Posts are kept only when a named entity
            can be extracted.
          </p>
          <p class="panel-note">
            The Apify actor requires at least 10 max posts. Lower values are adjusted to 10.
          </p>
          <p class="panel-note">
            One live crawl sends all seed keywords in a single actor run. Live runs are disabled by
            default and limited per app session to protect trial usage.
          </p>
          <div class="export-actions">
            <button
              type="button"
              on:click={replayLastApifyCrawl}
              disabled={isReplayingApify || isImportingApifyCache || isRunningDiscovery || isFullFlowRunning()}
            >
              {#if isReplayingApify}
                {@render LoadingLabel('Reprocessing...')}
              {:else}
                Reprocess Last Apify Result
              {/if}
            </button>
          </div>
          <p class="panel-note">
            Uses cached Apify output and does not consume Apify usage.
          </p>
          <label for="apify-dataset-file">Exported Apify dataset JSON</label>
          <div class="collector-row import-row">
            <input
              id="apify-dataset-file"
              type="text"
              bind:value={apifyDatasetFilePath}
              placeholder="/path/to/apify-dataset.json"
              disabled={isImportingApifyCache || isReplayingApify || isRunningDiscovery || isFullFlowRunning()}
            />
            <button
              type="button"
              on:click={importApifyDatasetCache}
              disabled={!apifyDatasetFilePath.trim() || isImportingApifyCache || isReplayingApify || isRunningDiscovery || isFullFlowRunning()}
            >
              {#if isImportingApifyCache}
                {@render LoadingLabel('Importing...')}
              {:else}
                Import Apify Dataset JSON
              {/if}
            </button>
          </div>
          <p class="panel-note">
            Import stores the exported array locally. It does not call Apify or process posts until
            Reprocess Last Apify Result is clicked.
          </p>
          <label for="apify-seeds">Apify seeds</label>
          <textarea
            id="apify-seeds"
            bind:value={apifySeedsText}
            rows="7"
            disabled={isRunningDiscovery || isReplayingApify || isImportingApifyCache || isFullFlowRunning()}
          ></textarea>
        {/if}
      </form>

      <div class="collector-result" aria-live="polite">
        <span>Run ID: {discoveryRunId}</span>
        <span>Status: {discoveryStatus}</span>
        <span>Mode: {discoveryMode}</span>
        <span>Started: {discoveryStartedAt}</span>
        <span>Finished: {discoveryFinishedAt}</span>
        <span>Duration: {discoveryDurationMs} ms</span>
        <span>Seeds processed: {discoverySeedsProcessed}</span>
        <span>Fetched total: {discoveryFetchedTotal}</span>
        <span>ID-only results: {discoveryIdOnlyResultsCount}</span>
        <span>Detail fetched: {discoveryDetailFetchedTotal}</span>
        <span>Detail failed: {discoveryDetailFailedTotal}</span>
        <span>Text missing: {discoveryTextMissingTotal}</span>
        <span>Saved total: {discoverySavedTotal}</span>
        <span>Duplicates skipped: {discoveryDuplicatesSkipped}</span>
        <span>Zero-result seeds: {discoveryZeroResultSeeds}</span>
        <span>Failed seeds: {discoveryFailedSeeds}</span>
        <span>Last successful seed: {discoveryLastSuccessfulSeed}</span>
        <span>Last error: {discoveryLastErrorSummary}</span>
        <span>Permission limited hint: {discoveryPermissionLimitedHint ? 'yes' : 'no'}</span>
        {#if discoverySource === 'apify_threads_scraper' || discoveryMode === 'apify_threads_scraper' || discoveryMode === 'apify_cache_replay'}
          <span>Apify actor: {apifyActorId}</span>
          <span>Apify actor run ID: {apifyActorRunId}</span>
          <span>Entity gate included: {apifyEntityGateIncludedTotal}</span>
          <span>Entity gate filtered: {apifyEntityGateFilteredTotal}</span>
          <span>Apify filtered out: {apifyFilteredOutTotal}</span>
          <span>Filter no named entity: {apifyFilterNoNamedEntity}</span>
          <span>Filter recruitment/job post: {apifyFilterRecruitmentOrJobPost}</span>
          <span>Filter generic MCP only: {apifyFilterGenericMcpOnly}</span>
          <span>Filter generic AI Agent only: {apifyFilterGenericAiAgentOnly}</span>
          <span>Filter generic threadbait: {apifyFilterGenericThreadbait}</span>
          <span>Filter ambiguous without entity: {apifyFilterAmbiguousWithoutEntity}</span>
          <span>Filter empty text: {apifyFilterEmptyText}</span>
          <span>Filter duplicate: {apifyFilterDuplicate}</span>
        {/if}
        {#if discoveryPermissionLimitedHint}
          <span>
            Crawler may be limited to authenticated user posts until threads_keyword_search is
            approved for public search.
          </span>
        {/if}
        {#if discoveryErrors.length > 0}
          <span>Diagnostics: {discoveryErrors.join(' | ')}</span>
        {/if}
      </div>

      {#if apifySampleIncluded.length > 0}
        <div class="mention-preview apify-preview" aria-label="Apify included post preview">
          {#each apifySampleIncluded as post}
            <article class="mention-row">
              <div>
                <strong>{post.source_seed_keyword || 'Apify seed'}</strong>
                <span>{post.post_id}</span>
                <span>Entities: {post.detected_entities.join(', ')}</span>
                {#if post.permalink}
                  <span>{post.permalink}</span>
                {/if}
              </div>
              <p>{post.text_snippet}</p>
            </article>
          {/each}
        </div>
      {/if}

      {#if apifySampleFilteredOut.length > 0}
        <div class="mention-preview apify-preview" aria-label="Apify filtered post preview">
          {#each apifySampleFilteredOut as post}
            <article class="mention-row">
              <div>
                <strong>Filtered: {post.reason}</strong>
              </div>
              <p>{post.text_snippet || 'No text available'}</p>
            </article>
          {/each}
        </div>
      {/if}

      <form on:submit|preventDefault={testDiscoverySeed}>
        <label for="discovery-seed-test">Single seed test (debug only)</label>
        {#if discoverySource === 'apify_threads_scraper'}
          <p class="panel-note">
            Apify single-seed debug runs consume live usage and count toward the session limit.
          </p>
        {/if}
        <div class="collector-row">
          <input
            id="discovery-seed-test"
            bind:value={discoverySeedTestKeyword}
            placeholder="AI Agent"
            autocomplete="off"
            disabled={isTestingDiscoverySeed || isReplayingApify || isFullFlowRunning()}
          />
          <button
            type="submit"
            disabled={isTestingDiscoverySeed || isReplayingApify || isFullFlowRunning() || !discoverySeedTestKeyword.trim()}
          >
            {#if isTestingDiscoverySeed}
              {@render LoadingLabel('Testing...')}
            {:else}
              Test Seed
            {/if}
          </button>
        </div>
      </form>

      <div class="collector-result" aria-live="polite">
        <span>Seed test: {discoverySeedTestStatus}</span>
        {#if discoverySeedTestResult}
          <span>Fetched: {discoverySeedTestResult.fetched_count}</span>
          <span>Detail fetched: {discoverySeedTestResult.detail_fetched_count}</span>
          <span>Text available: {discoverySeedTestResult.text_available_count}</span>
          {#if discoverySeedTestResult.sample_text_snippet}
            <span>Sample text: {discoverySeedTestResult.sample_text_snippet}</span>
          {/if}
          {#if discoverySeedTestResult.error_summary}
            <span>Error summary: {discoverySeedTestResult.error_summary}</span>
          {/if}
        {/if}
      </div>

      {#if discoverySeedResults.length > 0}
        <div class="metrics-table-wrap">
          <table class="metrics-table">
            <thead>
              <tr>
                <th>Seed</th>
                <th>Status</th>
                <th>Fetched</th>
                <th>Saved</th>
                <th>Duplicates</th>
                <th>Detail Failed</th>
                <th>Pages</th>
                <th>Stop Reason</th>
                <th>Error Summary</th>
              </tr>
            </thead>
            <tbody>
              {#each discoverySeedResults as seed}
                <tr>
                  <td>{seed.seed_keyword}</td>
                  <td>{seed.search_status}</td>
                  <td>{seed.fetched_count}</td>
                  <td>{seed.saved_count}</td>
                  <td>{seed.duplicate_count}</td>
                  <td>{seed.detail_failed_count}</td>
                  <td>{seed.pages_fetched}</td>
                  <td>{seed.pagination_stopped_reason}</td>
                  <td>{seed.error_message_safe || '-'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <section class="collector-panel" aria-label="ExplainX import">
      <div>
        <p class="panel-label">1. Source Import</p>
        <h2>ExplainX Import</h2>
        <p class="panel-note">
          Import local ExplainX JSON as structured registry evidence. Exact aliases create pending
          identity links only; ambiguous and child-resource records remain reviewable.
        </p>
      </div>

      <form on:submit|preventDefault={importExplainXRecords}>
        <label for="explainx-file">ExplainX JSON file</label>
        <div class="collector-row import-row">
          <input
            id="explainx-file"
            type="text"
            bind:value={explainxFilePath}
            placeholder="/path/to/explainx-records.json"
            disabled={isImportingExplainX || isFullFlowRunning()}
          />
          <button
            type="submit"
            disabled={!explainxFilePath.trim() || isImportingExplainX || isFullFlowRunning()}
          >
            {#if isImportingExplainX}
              {@render LoadingLabel('Importing...')}
            {:else}
              Import ExplainX Records
            {/if}
          </button>
        </div>
      </form>

      <div class="collector-result" aria-live="polite">
        <span>Status: {explainxImportStatus}</span>
        <span>Imported: {explainxImported}</span>
        <span>Inserted: {explainxInserted}</span>
        <span>Updated: {explainxUpdated}</span>
        <span>Unchanged: {explainxSkipped}</span>
        <span>Invalid: {explainxInvalid}</span>
        <span>Linked exact alias: {explainxLinkedExactAlias}</span>
        <span>Review needed: {explainxReviewNeeded}</span>
        <span>Unlinked: {explainxUnlinked}</span>
      </div>

      {#if explainxImportPreview.length > 0}
        <div class="mention-preview" aria-label="ExplainX import preview">
          {#each explainxImportPreview as record}
            <article class="mention-row">
              <div>
                <strong>{record.name}</strong>
                <span>{record.category}</span>
                <span>{record.source_record_key}</span>
                <span>Tags: {record.tags.length > 0 ? record.tags.join(', ') : 'none'}</span>
              </div>
              <p>{record.reason}</p>
              <div>
                <strong>{record.identity_status}</strong>
                <span>Canonical: {record.matched_canonical_entity}</span>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </section>

    <section class="detector-panel" aria-label="External identity review">
      <div class="detector-header">
        <div>
          <p class="panel-label">1. External Identity Review</p>
          <h2>Review ExplainX Identity Links</h2>
          <p class="panel-note">
            Confirm whether an imported source record is the same canonical entity, a child
            resource, a related entity, or only a mention. Every decision is retained in history.
          </p>
        </div>
        <button
          type="button"
          on:click={loadExternalIdentityReviewItems}
          disabled={isLoadingExternalIdentityReviews || Boolean(activeExternalIdentityReviewAction)}
        >
          {#if isLoadingExternalIdentityReviews}
            {@render LoadingLabel('Refreshing...')}
          {:else}
            Refresh Identity Reviews
          {/if}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {externalIdentityReviewStatus}</span>
        <span>Review needed: {externalIdentityReviewPending}</span>
        <span>Approved: {externalIdentityReviewApproved}</span>
        <span>Rejected: {externalIdentityReviewRejected}</span>
        <span>Ambiguous: {externalIdentityReviewAmbiguous}</span>
      </div>

      {#if externalIdentityReviewItems.length > 0}
        <div class="external-review-list" aria-label="ExplainX identity review items">
          {#each externalIdentityReviewItems as item}
            <article class="external-review-item">
              <div class="external-review-summary">
                <div>
                  <strong>{item.source_record_name}</strong>
                  <span>{item.source} / {item.source_record_key}</span>
                  {#if item.source_record_url}
                    <span>{item.source_record_url}</span>
                  {/if}
                </div>
                <div>
                  <strong>{item.canonical_entity_name}</strong>
                  <span>{item.canonical_entity_type}</span>
                  <span>Relationship: {item.relationship_type}</span>
                </div>
                <div>
                  <strong class={`review-state review-state-${item.current_status}`}>
                    {externalReviewStateLabel(item.current_status)}
                  </strong>
                  <span>Latest: {externalReviewStateLabel(item.latest_decision)}</span>
                  {#if item.latest_reviewer}
                    <span>Reviewer: {item.latest_reviewer}</span>
                  {/if}
                </div>
              </div>

              {#if item.source_record_description}
                <p>{item.source_record_description}</p>
              {/if}
              <p>{item.match_reason}</p>
              <p class="review-evidence">Evidence: {item.evidence || 'none'}</p>

              <div class="external-review-controls">
                <label>
                  Relationship
                  <select
                    bind:value={item.draft_relationship_type}
                    disabled={Boolean(activeExternalIdentityReviewAction)}
                  >
                    {#each externalRelationshipOptions as relationship}
                      <option value={relationship}>{relationship}</option>
                    {/each}
                  </select>
                </label>
                <label>
                  Reviewer
                  <input
                    bind:value={item.draft_reviewer}
                    placeholder="Reviewer name"
                    disabled={Boolean(activeExternalIdentityReviewAction)}
                  />
                </label>
                <label>
                  Evidence note
                  <input
                    bind:value={item.draft_note}
                    placeholder="Optional note"
                    disabled={Boolean(activeExternalIdentityReviewAction)}
                  />
                </label>
              </div>

              <div class="export-actions external-review-actions">
                <button
                  type="button"
                  on:click={() => submitExternalIdentityReview(item, 'approved')}
                  disabled={Boolean(activeExternalIdentityReviewAction) || !item.draft_reviewer?.trim()}
                >
                  {#if activeExternalIdentityReviewAction === item.link_id && activeExternalIdentityReviewDecision === 'approved'}
                    {@render LoadingLabel('Approving...')}
                  {:else}
                    Approve
                  {/if}
                </button>
                <button
                  type="button"
                  on:click={() => submitExternalIdentityReview(item, 'rejected')}
                  disabled={Boolean(activeExternalIdentityReviewAction) || !item.draft_reviewer?.trim()}
                >
                  {#if activeExternalIdentityReviewAction === item.link_id && activeExternalIdentityReviewDecision === 'rejected'}
                    {@render LoadingLabel('Rejecting...')}
                  {:else}
                    Reject
                  {/if}
                </button>
                <button
                  type="button"
                  on:click={() => submitExternalIdentityReview(item, 'ambiguous')}
                  disabled={Boolean(activeExternalIdentityReviewAction) || !item.draft_reviewer?.trim()}
                >
                  {#if activeExternalIdentityReviewAction === item.link_id && activeExternalIdentityReviewDecision === 'ambiguous'}
                    {@render LoadingLabel('Saving...')}
                  {:else}
                    Mark Ambiguous
                  {/if}
                </button>
                <button
                  type="button"
                  on:click={() => loadExternalIdentityReviewHistory(item.link_id)}
                  disabled={Boolean(activeExternalIdentityReviewAction)}
                >
                  View History
                </button>
              </div>

              {#if externalIdentityReviewHistoryLinkId === item.link_id}
                <div class="review-history" aria-live="polite">
                  <strong>Review History</strong>
                  <span>{externalIdentityReviewHistoryStatus}</span>
                  {#if externalIdentityReviewHistory.length > 0}
                    <div class="metrics-table-wrap">
                      <table class="metrics-table review-history-table">
                        <thead>
                          <tr>
                            <th>Reviewed</th>
                            <th>Previous</th>
                            <th>Decision</th>
                            <th>Relationship</th>
                            <th>Reviewer</th>
                            <th>Note</th>
                          </tr>
                        </thead>
                        <tbody>
                          {#each externalIdentityReviewHistory as history}
                            <tr>
                              <td>{history.reviewed_at}</td>
                              <td>{externalReviewStateLabel(history.previous_state)}</td>
                              <td>{externalReviewStateLabel(history.decision)}</td>
                              <td>{history.proposed_relationship_type}</td>
                              <td>{history.reviewer}</td>
                              <td>{history.review_note || '-'}</td>
                            </tr>
                          {/each}
                        </tbody>
                      </table>
                    </div>
                  {/if}
                </div>
              {/if}
            </article>
          {/each}
        </div>
      {:else}
        <p class="empty-state">No ExplainX identity links are available for review.</p>
      {/if}
    </section>

    <section class="collector-panel" aria-label="Threads keyword collector">
      <div>
        <p class="panel-label">1. Discovery</p>
        <h2>Collect raw posts</h2>
        <p class="panel-note">
          Use a single keyword for focused debugging, or import sample posts for a repeatable demo.
        </p>
      </div>

      <form on:submit|preventDefault={collectThreads}>
        <label for="keyword">Keyword</label>
        <div class="collector-row">
          <input
            id="keyword"
            bind:value={keyword}
            placeholder="AI Agent"
            autocomplete="off"
            disabled={isCollecting || isFullFlowRunning()}
          />
          <button type="submit" disabled={isCollecting || isFullFlowRunning() || !keyword.trim()}>
            {#if isCollecting}
              {@render LoadingLabel('Collecting...')}
            {:else}
              Collect
            {/if}
          </button>
          <button type="button" on:click={importSamplePosts} disabled={isImportingSamples || isFullFlowRunning()}>
            {#if isImportingSamples}
              {@render LoadingLabel('Importing...')}
            {:else}
              Import Sample Posts
            {/if}
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
          <p class="panel-label">2. Entity Detection</p>
          <h2>Detect agent mentions</h2>
          <p class="panel-note">
            Find tools, agents, skills, MCP terms, and candidate names from collected posts.
          </p>
        </div>
        <button type="button" on:click={detectAgentMentions} disabled={isDetecting || isFullFlowRunning()}>
          {#if isDetecting}
            {@render LoadingLabel('Detecting...')}
          {:else}
            Detect Agent Mentions
          {/if}
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

    <section class="detector-panel" aria-label="Mention identity linkage">
      <div class="detector-header">
        <div>
          <p class="panel-label">2. Identity Linkage</p>
          <h2>Link mentions to canonical entities</h2>
          <p class="panel-note">
            Resolve detected names through active, source-aware aliases while preserving the
            original mention text.
          </p>
        </div>
        <button
          type="button"
          on:click={linkAgentMentionsToEntities}
          disabled={isLinkingIdentities || isFullFlowRunning()}
        >
          {#if isLinkingIdentities}
            {@render LoadingLabel('Linking...')}
          {:else}
            Link Mentions to Canonical Entities
          {/if}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {identityLinkageStatus}</span>
        <span>Resolved: {identityResolvedCount}</span>
        <span>Missing alias: {identityMissingAliasCount}</span>
        <span>Ambiguous: {identityAmbiguousCount}</span>
        <span>Skipped: {identitySkippedCount}</span>
        <span>Errors: {identityErrorCount}</span>
      </div>

      {#if identityLinkagePreview.length > 0}
        <div class="mention-preview" aria-label="Identity linkage preview">
          {#each identityLinkagePreview as item}
            <article class="mention-row">
              <div>
                <strong>{item.mention_name}</strong>
                <span>{item.resolution_status}</span>
                {#if item.canonical_entity_name}
                  <span>Canonical: {item.canonical_entity_name}</span>
                {/if}
              </div>
              <p>{item.reason}</p>
            </article>
          {/each}
        </div>
      {/if}
    </section>

    <section class="detector-panel" aria-label="Candidate review">
      <div class="detector-header">
        <div>
          <p class="panel-label">3. Candidate Review</p>
          <h2>Review Unknown Candidates</h2>
          <p class="panel-note">
            Review newly discovered names that may be AI agents, skills, MCP servers, frameworks, or
            tools.
          </p>
        </div>
        <button type="button" on:click={loadCandidateEntities} disabled={isLoadingCandidates || isFullFlowRunning()}>
          {#if isLoadingCandidates}
            {@render LoadingLabel('Refreshing...')}
          {:else}
            Refresh Candidates
          {/if}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {candidateReviewStatus}</span>
        <span>Pending: {pendingCandidateCount}</span>
        <span>Approved: {approvedCandidateCount}</span>
        <span>Ignored: {ignoredCandidateCount}</span>
        <span>Approved decisions: {approvedDecisionCount}</span>
        <span>Ignored decisions: {ignoredDecisionCount}</span>
      </div>

      {#if candidateEntities.length > 0}
        <div class="mention-preview" aria-label="Candidate entity review list">
          {#each candidateEntities as candidate}
            <article class="mention-row">
              <div>
                <strong>{candidate.candidate_name}</strong>
                <span>{candidate.current_status}</span>
                <span>{candidate.mention_count} mentions</span>
                {#if candidate.reviewed_as}
                  <span>reviewed as {candidate.reviewed_as}</span>
                {/if}
                {#if candidate.reviewed_category}
                  <span>{candidate.reviewed_category}</span>
                {/if}
              </div>

              {#if candidate.sample_snippets.length > 0}
                <p>{candidate.sample_snippets[0]}</p>
              {/if}

              <div class="collector-row discovery-row">
                <input
                  bind:value={candidate.draft_reviewed_as}
                  placeholder="Canonical name"
                  aria-label={`Canonical name for ${candidate.candidate_name}`}
                  disabled={activeCandidateAction === candidate.candidate_name || isFullFlowRunning()}
                />
                <select
                  bind:value={candidate.draft_reviewed_category}
                  aria-label={`Category for ${candidate.candidate_name}`}
                  disabled={activeCandidateAction === candidate.candidate_name || isFullFlowRunning()}
                >
                  {#each candidateCategoryOptions as category}
                    <option value={category}>{category}</option>
                  {/each}
                </select>
                <input
                  bind:value={candidate.draft_note}
                  placeholder="Review note"
                  aria-label={`Review note for ${candidate.candidate_name}`}
                  disabled={activeCandidateAction === candidate.candidate_name || isFullFlowRunning()}
                />
              </div>

              <div class="export-actions">
                <button
                  type="button"
                  on:click={() => approveCandidate(candidate)}
                  disabled={Boolean(activeCandidateAction) || isFullFlowRunning()}
                >
                  {#if activeCandidateAction === candidate.candidate_name && activeCandidateActionType === 'approve'}
                    {@render LoadingLabel('Approving...')}
                  {:else}
                    Approve
                  {/if}
                </button>
                <button
                  type="button"
                  on:click={() => ignoreCandidate(candidate)}
                  disabled={Boolean(activeCandidateAction) || isFullFlowRunning()}
                >
                  {#if activeCandidateAction === candidate.candidate_name && activeCandidateActionType === 'ignore'}
                    {@render LoadingLabel('Ignoring...')}
                  {:else}
                    Ignore
                  {/if}
                </button>
                <button
                  type="button"
                  on:click={() => resetCandidate(candidate)}
                  disabled={Boolean(activeCandidateAction) || isFullFlowRunning()}
                >
                  {#if activeCandidateAction === candidate.candidate_name && activeCandidateActionType === 'reset'}
                    {@render LoadingLabel('Resetting...')}
                  {:else}
                    Reset
                  {/if}
                </button>
              </div>
            </article>
          {/each}
        </div>
      {:else}
        <p class="empty-state">No candidate entities yet.</p>
      {/if}

      {#if entityReviewDecisions.length > 0}
        <div class="metrics-groups">
          <div>
            <h3>Decision Registry</h3>
            <div class="metrics-table-wrap">
              <table class="metrics-table">
                <thead>
                  <tr>
                    <th>Candidate</th>
                    <th>Normalized</th>
                    <th>Category</th>
                    <th>Status</th>
                    <th>Updated</th>
                    <th>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {#each entityReviewDecisions as decision}
                    <tr>
                      <td>{decision.candidate_name}</td>
                      <td>{decision.normalized_name || '-'}</td>
                      <td>{decision.category || '-'}</td>
                      <td>{decision.status}</td>
                      <td>{decision.updated_at}</td>
                      <td>
                        <button
                          type="button"
                          on:click={() =>
                            resetCandidate({
                              candidate_name: decision.candidate_name,
                              mention_count: 0,
                              first_seen: decision.created_at,
                              latest_seen: decision.updated_at,
                              sample_snippets: [],
                              current_status: decision.status,
                              reviewed_as: decision.normalized_name,
                              reviewed_category: decision.category,
                            })}
                          disabled={Boolean(activeCandidateAction) || isFullFlowRunning()}
                        >
                          {#if activeCandidateAction === decision.candidate_name && activeCandidateActionType === 'reset'}
                            {@render LoadingLabel('Resetting...')}
                          {:else}
                            Reset
                          {/if}
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      {:else}
        <p class="empty-state">No durable decisions yet.</p>
      {/if}
    </section>

    <section class="detector-panel" aria-label="Region classifier">
      <div class="detector-header">
        <div>
          <p class="panel-label">4. Classification</p>
          <h2>Classify Indonesia vs Global</h2>
          <p class="panel-note">
            Label posts as Indonesia, global, or unknown for regional trend ranking.
          </p>
        </div>
        <button type="button" on:click={classifyRegions} disabled={isClassifyingRegions || isFullFlowRunning()}>
          {#if isClassifyingRegions}
            {@render LoadingLabel('Classifying...')}
          {:else}
            Classify Regions
          {/if}
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
          <p class="panel-label">4. Classification</p>
          <h2>Classify Sentiments</h2>
          <p class="panel-note">
            Estimate positive, neutral, negative, or mixed opinion signals for detected mentions.
          </p>
        </div>
        <button
          type="button"
          on:click={classifySentiments}
          disabled={isClassifyingSentiments || isFullFlowRunning()}
        >
          {#if isClassifyingSentiments}
            {@render LoadingLabel('Classifying...')}
          {:else}
            Classify Sentiments
          {/if}
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
          <p class="panel-label">4. Classification</p>
          <h2>Classify Cost Signals</h2>
          <p class="panel-note">
            Detect cost, quota, and boros-token signals without sending data to an external model.
          </p>
        </div>
        <button
          type="button"
          on:click={classifyCostSignals}
          disabled={isClassifyingCostSignals || isFullFlowRunning()}
        >
          {#if isClassifyingCostSignals}
            {@render LoadingLabel('Classifying...')}
          {:else}
            Classify Cost Signals
          {/if}
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
          <p class="panel-label">5. Weekly Metrics</p>
          <h2>Aggregate Weekly Metrics</h2>
          <p class="panel-note">
            Rank approved and known entities by region, sentiment mix, boros signal, and trend score.
          </p>
        </div>
        <button
          type="button"
          on:click={aggregateWeeklyMetrics}
          disabled={isAggregatingWeeklyMetrics || isFullFlowRunning()}
        >
          {#if isAggregatingWeeklyMetrics}
            {@render LoadingLabel('Aggregating...')}
          {:else}
            Aggregate Weekly Metrics
          {/if}
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

    <section class="detector-panel" aria-label="Canonical weekly metrics">
      <div class="detector-header">
        <div>
          <p class="panel-label">5. Canonical Weekly Metrics</p>
          <h2>Aggregate Canonical Weekly Metrics</h2>
          <p class="panel-note">
            Roll resolved aliases into one canonical entity row per week and region. Ambiguous or
            unresolved mentions remain excluded.
          </p>
        </div>
        <button
          type="button"
          on:click={aggregateCanonicalWeeklyMetrics}
          disabled={isAggregatingCanonicalWeeklyMetrics || isFullFlowRunning()}
        >
          {#if isAggregatingCanonicalWeeklyMetrics}
            {@render LoadingLabel('Aggregating...')}
          {:else}
            Aggregate Canonical Weekly Metrics
          {/if}
        </button>
      </div>

      <div class="collector-result detector-result" aria-live="polite">
        <span>Status: {canonicalWeeklyStatus}</span>
        <span>Canonical rows: {canonicalWeeklyRows}</span>
        <span>Resolved entities: {canonicalResolvedEntities}</span>
        <span>Unresolved skipped: {canonicalUnresolvedSkipped}</span>
        <span>Missing alias skipped: {canonicalMissingAliasSkipped}</span>
        <span>Ambiguous skipped: {canonicalAmbiguousSkipped}</span>
        <span>Other skipped: {canonicalSkippedMentions}</span>
        {#if canonicalWeeklyErrors.length > 0}
          <span>Errors: {canonicalWeeklyErrors.join(' | ')}</span>
        {/if}
      </div>

      <div class="metrics-groups">
        <div>
          <h3>Top Indonesia</h3>
          {@render CanonicalMetricTable(canonicalTopIndonesia)}
        </div>
        <div>
          <h3>Top Global</h3>
          {@render CanonicalMetricTable(canonicalTopGlobal)}
        </div>
      </div>
    </section>

    <section class="detector-panel" aria-label="Report export">
      <div class="detector-header">
        <div>
          <p class="panel-label">6. Export Report</p>
          <h2>Export Weekly Report</h2>
          <p class="panel-note">
            Save Markdown and CSV outputs locally for weekly review or presentation.
          </p>
        </div>
        <div class="export-actions">
          <button type="button" on:click={exportMarkdownReport} disabled={isExportingMarkdown || isFullFlowRunning()}>
            {#if isExportingMarkdown}
              {@render LoadingLabel('Exporting...')}
            {:else}
              Export Markdown Report
            {/if}
          </button>
          <button type="button" on:click={exportCsvMetrics} disabled={isExportingCsv || isFullFlowRunning()}>
            {#if isExportingCsv}
              {@render LoadingLabel('Exporting...')}
            {:else}
              Export CSV Metrics
            {/if}
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

{#snippet LoadingLabel(label: string)}
  <span class="button-loading">
    <span class="loading-spinner" aria-hidden="true"></span>
    {label}
  </span>
{/snippet}

{#snippet CanonicalMetricTable(metrics: WeeklyEntityMetric[])}
  {#if metrics.length > 0}
    <div class="metrics-table-wrap">
      <table class="metrics-table">
        <thead>
          <tr>
            <th>Rank</th>
            <th>Canonical entity</th>
            <th>Type</th>
            <th>Region</th>
            <th>Mentions</th>
            <th>Sentiment + / 0 / - / mixed</th>
            <th>Cost + / boros / mixed / none</th>
            <th>Sources</th>
            <th>Score</th>
          </tr>
        </thead>
        <tbody>
          {#each metrics as metric}
            <tr>
              <td>{metric.rank}</td>
              <td>{metric.canonical_name}</td>
              <td>{metric.entity_type}</td>
              <td>{metric.region}</td>
              <td>{metric.mention_count}</td>
              <td>
                {metric.positive_count} / {metric.neutral_count} / {metric.negative_count} /
                {metric.mixed_count}
              </td>
              <td>
                {metric.cost_positive_count} / {metric.cost_negative_boros_count} /
                {metric.cost_mixed_count} / {metric.cost_not_mentioned_count}
              </td>
              <td>{metric.source_count}</td>
              <td>{metric.trend_score.toFixed(1)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="empty-state">No canonical metrics yet.</p>
  {/if}
{/snippet}

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
