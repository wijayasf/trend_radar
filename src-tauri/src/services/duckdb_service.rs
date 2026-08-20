use std::fs;
use std::path::{Path, PathBuf};

use duckdb::params;
use duckdb::Connection;
use duckdb::Transaction;

use crate::models::entities::{
    AgentMentionForCost, AgentMentionForIdentityLinkage, AgentMentionForSentiment,
    AgentMentionPreview, CandidateEntityReview, CostClassification, DetectedAgentMention,
    EntityReviewDecision, MentionIdentityResolution, RawPostForDetection, RegionClassification,
    SentimentClassification,
};
use crate::models::threads::{DiscoveryCrawlResult, ThreadPostRaw};
use crate::models::trend::{IdentityResolutionSkipCounts, WeeklyAgentMetric, WeeklyEntityMetric};
use crate::utils::config;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS threads_posts_raw (
    post_id TEXT PRIMARY KEY,
    thread_id TEXT,
    author_id TEXT,
    author_username TEXT,
    author_display_name TEXT,
    text TEXT NOT NULL,
    text_missing BOOLEAN DEFAULT FALSE,
    permalink TEXT,
    media_type TEXT,
    source_type TEXT,
    source_seed_keyword TEXT,
    keyword_match TEXT,
    language TEXT,
    region_hint TEXT,
    region_confidence DOUBLE DEFAULT 0.0,
    region_reason TEXT,
    like_count BIGINT DEFAULT 0,
    reply_count BIGINT DEFAULT 0,
    repost_count BIGINT DEFAULT 0,
    quote_count BIGINT DEFAULT 0,
    share_count BIGINT DEFAULT 0,
    view_count BIGINT DEFAULT 0,
    posted_at TIMESTAMP,
    collected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    raw_json TEXT
);

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS author_display_name TEXT;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS media_type TEXT;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS source_type TEXT;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS source_seed_keyword TEXT;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS keyword_match TEXT;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS text_missing BOOLEAN DEFAULT FALSE;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS region_confidence DOUBLE DEFAULT 0.0;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS region_reason TEXT;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS share_count BIGINT DEFAULT 0;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS view_count BIGINT DEFAULT 0;

CREATE TABLE IF NOT EXISTS crawl_runs (
    id TEXT PRIMARY KEY,
    mode TEXT,
    seed_group TEXT,
    max_per_seed BIGINT DEFAULT 0,
    seeds_processed BIGINT DEFAULT 0,
    fetched_total BIGINT DEFAULT 0,
    saved_total BIGINT DEFAULT 0,
    duplicates_skipped BIGINT DEFAULT 0,
    zero_result_seeds BIGINT DEFAULT 0,
    failed_seeds BIGINT DEFAULT 0,
    detail_fetched_total BIGINT DEFAULT 0,
    detail_failed_total BIGINT DEFAULT 0,
    text_missing_total BIGINT DEFAULT 0,
    started_at TEXT,
    finished_at TEXT,
    duration_ms BIGINT DEFAULT 0,
    status TEXT,
    error_summary TEXT
);

ALTER TABLE crawl_runs
    ADD COLUMN IF NOT EXISTS max_per_seed BIGINT DEFAULT 0;

ALTER TABLE crawl_runs
    ADD COLUMN IF NOT EXISTS zero_result_seeds BIGINT DEFAULT 0;

ALTER TABLE crawl_runs
    ADD COLUMN IF NOT EXISTS duration_ms BIGINT DEFAULT 0;

CREATE TABLE IF NOT EXISTS agent_mentions (
    mention_id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    agent_alias TEXT,
    category TEXT DEFAULT 'unknown',
    detection_source TEXT DEFAULT 'known_alias',
    needs_review BOOLEAN DEFAULT FALSE,
    review_status TEXT DEFAULT 'approved',
    reviewed_as TEXT,
    reviewed_category TEXT,
    review_note TEXT,
    reviewed_at TIMESTAMP,
    entity_id UUID,
    identity_resolution_status TEXT,
    identity_resolution_reason TEXT,
    identity_resolution_confidence DOUBLE,
    identity_resolved_at TIMESTAMP,
    region TEXT DEFAULT 'unknown',
    confidence DOUBLE DEFAULT 0.0,
    match_confidence DOUBLE DEFAULT 0.0,
    relevance_score DOUBLE DEFAULT 0.0,
    sentiment TEXT DEFAULT 'unknown',
    sentiment_confidence DOUBLE DEFAULT 0.0,
    sentiment_reason TEXT,
    cost_signal TEXT DEFAULT 'none',
    cost_confidence DOUBLE DEFAULT 0.0,
    cost_reason TEXT,
    source_snippet TEXT,
    region_confidence DOUBLE DEFAULT 0.0,
    region_reason TEXT,
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES threads_posts_raw(post_id),
    CHECK (category IN (
        'coding_agent',
        'coding_assistant',
        'generic_agent_framework',
        'skill_or_mode',
        'mcp_or_connector',
        'registry_or_discovery',
        'app_builder',
        'unknown_candidate',
        'unknown'
    )),
    CHECK (review_status IN ('pending', 'approved', 'ignored')),
    CHECK (region IN ('indonesia', 'global', 'unknown'))
);

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS category TEXT DEFAULT 'unknown';

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS detection_source TEXT DEFAULT 'known_alias';

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS needs_review BOOLEAN DEFAULT FALSE;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS review_status TEXT DEFAULT 'approved';

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS reviewed_as TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS reviewed_category TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS review_note TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMP;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS entity_id UUID;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS identity_resolution_status TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS identity_resolution_reason TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS identity_resolution_confidence DOUBLE;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS identity_resolved_at TIMESTAMP;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS match_confidence DOUBLE DEFAULT 0.0;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS relevance_score DOUBLE DEFAULT 0.0;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS sentiment TEXT DEFAULT 'unknown';

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS sentiment_confidence DOUBLE DEFAULT 0.0;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS sentiment_reason TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS cost_signal TEXT DEFAULT 'none';

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS cost_confidence DOUBLE DEFAULT 0.0;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS cost_reason TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS source_snippet TEXT;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS region_confidence DOUBLE DEFAULT 0.0;

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS region_reason TEXT;

CREATE TABLE IF NOT EXISTS entity_review_decisions (
    id TEXT PRIMARY KEY,
    candidate_name TEXT NOT NULL,
    normalized_name TEXT,
    category TEXT,
    status TEXT NOT NULL,
    note TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CHECK (status IN ('approved', 'ignored')),
    CHECK (
        status != 'approved'
        OR (
            normalized_name IS NOT NULL
            AND length(trim(normalized_name)) > 0
            AND category IS NOT NULL
            AND length(trim(category)) > 0
        )
    )
);

ALTER TABLE entity_review_decisions
    ADD COLUMN IF NOT EXISTS normalized_name TEXT;

ALTER TABLE entity_review_decisions
    ADD COLUMN IF NOT EXISTS category TEXT;

ALTER TABLE entity_review_decisions
    ADD COLUMN IF NOT EXISTS status TEXT DEFAULT 'ignored';

ALTER TABLE entity_review_decisions
    ADD COLUMN IF NOT EXISTS note TEXT;

ALTER TABLE entity_review_decisions
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE entity_review_decisions
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

CREATE TABLE IF NOT EXISTS weekly_agent_metrics (
    week_start DATE NOT NULL,
    week_end DATE,
    region TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    category TEXT DEFAULT 'unknown',
    mentions BIGINT DEFAULT 0,
    mention_count BIGINT DEFAULT 0,
    unique_author_count BIGINT DEFAULT 0,
    positive_count BIGINT DEFAULT 0,
    negative_count BIGINT DEFAULT 0,
    neutral_count BIGINT DEFAULT 0,
    mixed_count BIGINT DEFAULT 0,
    cost_not_mentioned_count BIGINT DEFAULT 0,
    cost_positive_count BIGINT DEFAULT 0,
    cost_negative_boros_count BIGINT DEFAULT 0,
    cost_mixed_count BIGINT DEFAULT 0,
    cost_expensive_count BIGINT DEFAULT 0,
    cost_token_heavy_count BIGINT DEFAULT 0,
    cost_quota_limited_count BIGINT DEFAULT 0,
    cost_worth_it_count BIGINT DEFAULT 0,
    positive_pct DOUBLE DEFAULT 0.0,
    negative_pct DOUBLE DEFAULT 0.0,
    cost_negative_boros_pct DOUBLE DEFAULT 0.0,
    trend_score DOUBLE DEFAULT 0.0,
    computed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (week_start, region, agent_name),
    CHECK (region IN ('indonesia', 'global', 'unknown'))
);

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS week_end DATE;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS category TEXT DEFAULT 'unknown';

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS mentions BIGINT DEFAULT 0;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS cost_not_mentioned_count BIGINT DEFAULT 0;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS cost_positive_count BIGINT DEFAULT 0;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS cost_negative_boros_count BIGINT DEFAULT 0;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS cost_mixed_count BIGINT DEFAULT 0;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS positive_pct DOUBLE DEFAULT 0.0;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS negative_pct DOUBLE DEFAULT 0.0;

ALTER TABLE weekly_agent_metrics
    ADD COLUMN IF NOT EXISTS cost_negative_boros_pct DOUBLE DEFAULT 0.0;

CREATE INDEX IF NOT EXISTS idx_threads_posts_raw_posted_at
    ON threads_posts_raw(posted_at);

CREATE INDEX IF NOT EXISTS idx_agent_mentions_agent_region
    ON agent_mentions(agent_name, region);

CREATE INDEX IF NOT EXISTS idx_weekly_agent_metrics_region_score
    ON weekly_agent_metrics(region, trend_score);
"#;

const MULTI_SOURCE_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS canonical_entities (
    entity_id UUID PRIMARY KEY DEFAULT uuid(),
    canonical_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    primary_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    description TEXT,
    primary_website TEXT,
    primary_repository TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (status IN ('active', 'archived')),
    CHECK (primary_type IN (
        'agent_tool',
        'framework_sdk',
        'skill_mode',
        'protocol',
        'connector_plugin',
        'registry_discovery',
        'app_builder',
        'other'
    ))
);

CREATE TABLE IF NOT EXISTS weekly_entity_metrics (
    id UUID PRIMARY KEY DEFAULT uuid(),
    week_start DATE NOT NULL,
    week_end DATE NOT NULL,
    entity_id UUID NOT NULL,
    canonical_name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    region TEXT NOT NULL,
    mention_count BIGINT NOT NULL DEFAULT 0,
    positive_count BIGINT NOT NULL DEFAULT 0,
    neutral_count BIGINT NOT NULL DEFAULT 0,
    negative_count BIGINT NOT NULL DEFAULT 0,
    mixed_count BIGINT NOT NULL DEFAULT 0,
    cost_positive_count BIGINT NOT NULL DEFAULT 0,
    cost_negative_boros_count BIGINT NOT NULL DEFAULT 0,
    cost_mixed_count BIGINT NOT NULL DEFAULT 0,
    cost_not_mentioned_count BIGINT NOT NULL DEFAULT 0,
    source_count BIGINT NOT NULL DEFAULT 0,
    first_seen_at TIMESTAMP,
    last_seen_at TIMESTAMP,
    trend_score DOUBLE NOT NULL DEFAULT 0.0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (week_start, entity_id, region),
    CHECK (region IN ('indonesia', 'global', 'unknown'))
);

CREATE INDEX IF NOT EXISTS idx_weekly_entity_metrics_region_score
    ON weekly_entity_metrics(week_start, region, trend_score);

CREATE INDEX IF NOT EXISTS idx_weekly_entity_metrics_entity
    ON weekly_entity_metrics(entity_id, week_start);

CREATE TABLE IF NOT EXISTS source_collection_runs (
    collection_run_id UUID PRIMARY KEY DEFAULT uuid(),
    source TEXT NOT NULL,
    collection_mode TEXT NOT NULL,
    scope_json TEXT,
    started_at TIMESTAMP NOT NULL,
    finished_at TIMESTAMP,
    status TEXT NOT NULL,
    records_seen BIGINT NOT NULL DEFAULT 0,
    observations_saved BIGINT NOT NULL DEFAULT 0,
    error_summary TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (status IN ('running', 'completed', 'partial', 'failed')),
    CHECK (collection_mode IN ('scheduled', 'manual', 'import', 'replay'))
);

CREATE TABLE IF NOT EXISTS source_records (
    source_record_id UUID PRIMARY KEY DEFAULT uuid(),
    source TEXT NOT NULL,
    source_record_key TEXT NOT NULL,
    record_type TEXT NOT NULL,
    resolution_state TEXT NOT NULL DEFAULT 'unresolved',
    title TEXT,
    external_url TEXT,
    publisher TEXT,
    description TEXT,
    source_category TEXT,
    repository_url TEXT,
    published_at TIMESTAMP,
    listed_at TIMESTAMP,
    metadata_json TEXT,
    first_seen_at TIMESTAMP NOT NULL,
    last_seen_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (source, source_record_key),
    CHECK (resolution_state IN (
        'single_entity',
        'multiple_entities',
        'no_product_entity',
        'unresolved'
    ))
);

CREATE TABLE IF NOT EXISTS source_observations (
    observation_id UUID PRIMARY KEY DEFAULT uuid(),
    collection_run_id UUID NOT NULL,
    source_record_id UUID NOT NULL,
    observed_at TIMESTAMP NOT NULL,
    surface TEXT NOT NULL DEFAULT 'record',
    observation_kind TEXT NOT NULL,
    time_window TEXT NOT NULL DEFAULT 'none',
    rank BIGINT,
    source_score DOUBLE,
    views BIGINT,
    installs_total BIGINT,
    installs_period BIGINT,
    github_stars BIGINT,
    upvotes BIGINT,
    payload_hash TEXT,
    source_payload_json TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (collection_run_id)
        REFERENCES source_collection_runs(collection_run_id),
    FOREIGN KEY (source_record_id)
        REFERENCES source_records(source_record_id),
    UNIQUE (
        collection_run_id,
        source_record_id,
        surface,
        observation_kind,
        time_window
    ),
    CHECK (rank IS NULL OR rank > 0),
    CHECK (views IS NULL OR views >= 0),
    CHECK (installs_total IS NULL OR installs_total >= 0),
    CHECK (installs_period IS NULL OR installs_period >= 0),
    CHECK (github_stars IS NULL OR github_stars >= 0),
    CHECK (upvotes IS NULL OR upvotes >= 0)
);

CREATE TABLE IF NOT EXISTS source_record_entity_links (
    link_id UUID PRIMARY KEY DEFAULT uuid(),
    source_record_id UUID NOT NULL,
    entity_id UUID NOT NULL,
    relationship_type TEXT NOT NULL,
    match_method TEXT NOT NULL,
    match_confidence DOUBLE,
    review_state TEXT NOT NULL DEFAULT 'pending',
    evidence_json TEXT,
    reviewed_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_record_id)
        REFERENCES source_records(source_record_id),
    FOREIGN KEY (entity_id)
        REFERENCES canonical_entities(entity_id),
    UNIQUE (source_record_id, entity_id),
    CHECK (relationship_type IN (
        'same_entity',
        'child_resource',
        'related_entity',
        'mentioned_entity'
    )),
    CHECK (review_state IN ('pending', 'approved', 'rejected', 'ambiguous')),
    CHECK (
        match_confidence IS NULL
        OR (match_confidence >= 0.0 AND match_confidence <= 1.0)
    )
);

CREATE INDEX IF NOT EXISTS idx_canonical_entities_normalized_name
    ON canonical_entities(normalized_name);

CREATE INDEX IF NOT EXISTS idx_source_collection_runs_source_started
    ON source_collection_runs(source, started_at);

CREATE INDEX IF NOT EXISTS idx_source_records_source_type
    ON source_records(source, record_type);

CREATE INDEX IF NOT EXISTS idx_source_observations_record_time
    ON source_observations(source_record_id, observed_at);

CREATE INDEX IF NOT EXISTS idx_source_entity_links_entity
    ON source_record_entity_links(entity_id, review_state);
"#;

const IDENTITY_PERSISTENCE_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS entity_aliases (
    entity_alias_id UUID PRIMARY KEY DEFAULT uuid(),
    entity_id UUID NOT NULL,
    alias TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    source_scope TEXT NOT NULL,
    provenance TEXT NOT NULL,
    is_ambiguous BOOLEAN NOT NULL DEFAULT FALSE,
    context_terms_json TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (entity_id) REFERENCES canonical_entities(entity_id),
    UNIQUE (entity_id, normalized_alias, source_scope),
    CHECK (source_scope IN (
        'global',
        'threads',
        'explainx',
        'github',
        'hacker_news',
        'product_hunt'
    )),
    CHECK (provenance IN (
        'bootstrap_yaml',
        'candidate_review',
        'source_review',
        'manual'
    )),
    CHECK (status IN ('active', 'archived'))
);

CREATE TABLE IF NOT EXISTS external_identity_reviews (
    review_id UUID PRIMARY KEY DEFAULT uuid(),
    link_id UUID NOT NULL,
    source_record_id UUID NOT NULL,
    entity_id UUID NOT NULL,
    proposed_relationship_type TEXT NOT NULL,
    decision TEXT NOT NULL,
    match_method TEXT NOT NULL,
    match_confidence DOUBLE,
    evidence_json TEXT,
    review_note TEXT,
    reviewer TEXT NOT NULL,
    reviewed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (proposed_relationship_type IN (
        'same_entity',
        'child_resource',
        'related_entity',
        'mentioned_entity'
    )),
    CHECK (decision IN ('approved', 'rejected', 'ambiguous')),
    CHECK (
        match_confidence IS NULL
        OR (match_confidence >= 0.0 AND match_confidence <= 1.0)
    )
);

CREATE INDEX IF NOT EXISTS idx_entity_aliases_lookup
    ON entity_aliases(normalized_alias, source_scope, status);

CREATE INDEX IF NOT EXISTS idx_entity_aliases_entity
    ON entity_aliases(entity_id, status);

CREATE INDEX IF NOT EXISTS idx_external_identity_reviews_link_time
    ON external_identity_reviews(link_id, reviewed_at);

CREATE INDEX IF NOT EXISTS idx_external_identity_reviews_record_entity
    ON external_identity_reviews(source_record_id, entity_id);
"#;

const LEGACY_COMPATIBILITY_OBJECT: &str = "agent_mentions_compatible";
const LEGACY_LOCAL_DATABASE_MESSAGE: &str = "Legacy local DuckDB metadata detected. Stop the app and remove data/app.duckdb only if you want a clean local demo database.";

const THREADS_POST_INSERT_SQL: &str = r#"
INSERT OR REPLACE INTO threads_posts_raw (
    post_id,
    thread_id,
    author_id,
    author_username,
    author_display_name,
    text,
    text_missing,
    permalink,
    media_type,
    source_type,
    source_seed_keyword,
    keyword_match,
    language,
    region_hint,
    like_count,
    reply_count,
    repost_count,
    quote_count,
    share_count,
    view_count,
    posted_at,
    raw_json
) VALUES (
    ?1,
    NULL,
    ?2,
    ?3,
    ?4,
    ?5,
    ?6,
    ?7,
    ?8,
    ?9,
    ?10,
    ?11,
    NULL,
    NULL,
    ?12,
    ?13,
    ?14,
    ?15,
    ?16,
    ?17,
    TRY_CAST(?18 AS TIMESTAMP),
    ?19
);
"#;

const CRAWL_RUN_INSERT_SQL: &str = r#"
INSERT OR REPLACE INTO crawl_runs (
    id,
    mode,
    seed_group,
    max_per_seed,
    seeds_processed,
    fetched_total,
    saved_total,
    duplicates_skipped,
    zero_result_seeds,
    failed_seeds,
    detail_fetched_total,
    detail_failed_total,
    text_missing_total,
    started_at,
    finished_at,
    duration_ms,
    status,
    error_summary
) VALUES (
    ?1,
    ?2,
    ?3,
    ?4,
    ?5,
    ?6,
    ?7,
    ?8,
    ?9,
    ?10,
    ?11,
    ?12,
    ?13,
    ?14,
    ?15,
    ?16,
    ?17,
    ?18
);
"#;

const AGENT_MENTION_INSERT_SQL: &str = r#"
INSERT OR REPLACE INTO agent_mentions (
    mention_id,
    post_id,
    agent_name,
    agent_alias,
    category,
    detection_source,
    needs_review,
    review_status,
    reviewed_as,
    reviewed_category,
    review_note,
    reviewed_at,
    entity_id,
    identity_resolution_status,
    identity_resolution_reason,
    identity_resolution_confidence,
    identity_resolved_at,
    region,
    confidence,
    match_confidence,
    relevance_score,
    sentiment,
    cost_signal,
    source_snippet
) VALUES (
    ?1,
    ?2,
    ?3,
    ?4,
    ?5,
    ?6,
    ?7,
    ?15,
    ?16,
    ?17,
    (SELECT review_note FROM agent_mentions WHERE mention_id = ?1),
    CASE
        WHEN ?15 IN ('approved', 'ignored')
            THEN COALESCE((SELECT reviewed_at FROM agent_mentions WHERE mention_id = ?1), CURRENT_TIMESTAMP)
        ELSE (SELECT reviewed_at FROM agent_mentions WHERE mention_id = ?1)
    END,
    (SELECT entity_id FROM agent_mentions WHERE mention_id = ?1),
    (SELECT identity_resolution_status FROM agent_mentions WHERE mention_id = ?1),
    (SELECT identity_resolution_reason FROM agent_mentions WHERE mention_id = ?1),
    (SELECT identity_resolution_confidence FROM agent_mentions WHERE mention_id = ?1),
    (SELECT identity_resolved_at FROM agent_mentions WHERE mention_id = ?1),
    ?8,
    ?9,
    ?10,
    ?11,
    ?12,
    ?13,
    ?14
);
"#;

const THREADS_POST_REGION_UPDATE_SQL: &str = r#"
UPDATE threads_posts_raw
SET
    region_hint = ?2,
    region_confidence = ?3,
    region_reason = ?4
WHERE post_id = ?1;
"#;

const AGENT_MENTION_REGION_UPDATE_SQL: &str = r#"
UPDATE agent_mentions
SET
    region = ?2,
    region_confidence = ?3,
    region_reason = ?4
WHERE post_id = ?1;
"#;

const AGENT_MENTION_SENTIMENT_UPDATE_SQL: &str = r#"
UPDATE agent_mentions
SET
    sentiment = ?2,
    sentiment_confidence = ?3,
    sentiment_reason = ?4
WHERE mention_id = ?1;
"#;

const AGENT_MENTION_COST_UPDATE_SQL: &str = r#"
UPDATE agent_mentions
SET
    cost_signal = ?2,
    cost_confidence = ?3,
    cost_reason = ?4
WHERE mention_id = ?1;
"#;

const WEEKLY_AGENT_METRICS_RECREATE_SQL: &str = r#"
DROP TABLE IF EXISTS weekly_agent_metrics;

CREATE TABLE weekly_agent_metrics (
    week_start DATE NOT NULL,
    week_end DATE NOT NULL,
    region TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    category TEXT DEFAULT 'unknown',
    mentions BIGINT DEFAULT 0,
    mention_count BIGINT DEFAULT 0,
    unique_author_count BIGINT DEFAULT 0,
    positive_count BIGINT DEFAULT 0,
    neutral_count BIGINT DEFAULT 0,
    negative_count BIGINT DEFAULT 0,
    mixed_count BIGINT DEFAULT 0,
    cost_not_mentioned_count BIGINT DEFAULT 0,
    cost_positive_count BIGINT DEFAULT 0,
    cost_negative_boros_count BIGINT DEFAULT 0,
    cost_mixed_count BIGINT DEFAULT 0,
    cost_expensive_count BIGINT DEFAULT 0,
    cost_token_heavy_count BIGINT DEFAULT 0,
    cost_quota_limited_count BIGINT DEFAULT 0,
    cost_worth_it_count BIGINT DEFAULT 0,
    positive_pct DOUBLE DEFAULT 0.0,
    negative_pct DOUBLE DEFAULT 0.0,
    cost_negative_boros_pct DOUBLE DEFAULT 0.0,
    trend_score DOUBLE DEFAULT 0.0,
    computed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (week_start, region, agent_name),
    CHECK (region IN ('indonesia', 'global', 'unknown'))
);

CREATE INDEX IF NOT EXISTS idx_weekly_agent_metrics_region_score
    ON weekly_agent_metrics(region, trend_score);
"#;

const WEEKLY_AGENT_METRICS_INSERT_SQL: &str = r#"
INSERT INTO weekly_agent_metrics (
    week_start,
    week_end,
    region,
    agent_name,
    category,
    mentions,
    mention_count,
    unique_author_count,
    positive_count,
    neutral_count,
    negative_count,
    mixed_count,
    cost_not_mentioned_count,
    cost_positive_count,
    cost_negative_boros_count,
    cost_mixed_count,
    positive_pct,
    negative_pct,
    cost_negative_boros_pct,
    trend_score
)
WITH base AS (
    SELECT
        CAST(COALESCE(p.posted_at, p.collected_at) AS DATE)
            - CAST(((EXTRACT(dow FROM CAST(COALESCE(p.posted_at, p.collected_at) AS DATE)) + 6) % 7) AS INTEGER)
            AS week_start,
        COALESCE(m.region, 'unknown') AS region,
        lower(trim(m.agent_name)) AS canonical_entity_key,
        trim(m.agent_name) AS agent_name,
        COALESCE(m.category, 'unknown') AS category,
        COALESCE(m.sentiment, 'unknown') AS sentiment,
        COALESCE(m.cost_signal, 'not_mentioned') AS cost_signal
    FROM agent_mentions m
    JOIN threads_posts_raw p ON p.post_id = m.post_id
    WHERE m.agent_name IS NOT NULL AND length(trim(m.agent_name)) > 0
        AND COALESCE(
            m.review_status,
            CASE WHEN COALESCE(m.needs_review, FALSE) THEN 'pending' ELSE 'approved' END
        ) = 'approved'
        AND COALESCE(m.detection_source, 'known_alias') IN ('known_alias', 'reviewed_candidate')
        AND COALESCE(m.category, 'unknown') != 'unknown_candidate'
        AND lower(trim(m.agent_name)) NOT IN ('ai agent', 'html', 'llm', 'llms', 'mcp')
),
grouped AS (
    SELECT
        week_start,
        CAST(week_start + INTERVAL 6 DAY AS DATE) AS week_end,
        region,
        MIN(agent_name) AS agent_name,
        category,
        COUNT(*) AS mentions,
        SUM(CASE WHEN sentiment = 'positive' THEN 1 ELSE 0 END) AS positive_count,
        SUM(CASE WHEN sentiment = 'neutral' THEN 1 ELSE 0 END) AS neutral_count,
        SUM(CASE WHEN sentiment = 'negative' THEN 1 ELSE 0 END) AS negative_count,
        SUM(CASE WHEN sentiment = 'mixed' THEN 1 ELSE 0 END) AS mixed_count,
        SUM(CASE WHEN cost_signal IN ('not_mentioned', 'none') THEN 1 ELSE 0 END) AS cost_not_mentioned_count,
        SUM(CASE WHEN cost_signal = 'cost_positive' THEN 1 ELSE 0 END) AS cost_positive_count,
        SUM(CASE WHEN cost_signal = 'cost_negative_boros' THEN 1 ELSE 0 END) AS cost_negative_boros_count,
        SUM(CASE WHEN cost_signal = 'cost_mixed' THEN 1 ELSE 0 END) AS cost_mixed_count
    FROM base
    GROUP BY week_start, week_end, region, canonical_entity_key, category
)
SELECT
    week_start,
    week_end,
    region,
    agent_name,
    category,
    mentions,
    mentions AS mention_count,
    0 AS unique_author_count,
    positive_count,
    neutral_count,
    negative_count,
    mixed_count,
    cost_not_mentioned_count,
    cost_positive_count,
    cost_negative_boros_count,
    cost_mixed_count,
    ROUND(100.0 * positive_count / mentions, 2) AS positive_pct,
    ROUND(100.0 * negative_count / mentions, 2) AS negative_pct,
    ROUND(100.0 * cost_negative_boros_count / mentions, 2) AS cost_negative_boros_pct,
    -- TODO: Move MVP trend scoring weights to config/scoring.yml when scoring stabilizes.
    (mentions * 10)
        + (positive_count * 3)
        + (mixed_count * 1)
        - (negative_count * 2)
        - (cost_negative_boros_count * 1) AS trend_score
FROM grouped;
"#;

const WEEKLY_ENTITY_METRICS_INSERT_SQL: &str = r#"
INSERT INTO weekly_entity_metrics (
    week_start,
    week_end,
    entity_id,
    canonical_name,
    entity_type,
    region,
    mention_count,
    positive_count,
    neutral_count,
    negative_count,
    mixed_count,
    cost_positive_count,
    cost_negative_boros_count,
    cost_mixed_count,
    cost_not_mentioned_count,
    source_count,
    first_seen_at,
    last_seen_at,
    trend_score
)
WITH base AS (
    SELECT
        CAST(COALESCE(p.posted_at, p.collected_at) AS DATE)
            - CAST(((EXTRACT(dow FROM CAST(COALESCE(p.posted_at, p.collected_at) AS DATE)) + 6) % 7) AS INTEGER)
            AS week_start,
        m.entity_id,
        entities.canonical_name,
        entities.primary_type AS entity_type,
        COALESCE(m.region, 'unknown') AS region,
        COALESCE(m.sentiment, 'unknown') AS sentiment,
        COALESCE(m.cost_signal, 'not_mentioned') AS cost_signal,
        COALESCE(NULLIF(trim(p.source_type), ''), 'threads') AS source_name,
        COALESCE(p.posted_at, p.collected_at) AS observed_at
    FROM agent_mentions m
    JOIN threads_posts_raw p ON p.post_id = m.post_id
    JOIN canonical_entities entities ON entities.entity_id = m.entity_id
    WHERE m.entity_id IS NOT NULL
        AND m.identity_resolution_status = 'resolved'
        AND entities.status = 'active'
),
grouped AS (
    SELECT
        week_start,
        CAST(week_start + INTERVAL 6 DAY AS DATE) AS week_end,
        entity_id,
        canonical_name,
        entity_type,
        region,
        COUNT(*) AS mention_count,
        SUM(CASE WHEN sentiment = 'positive' THEN 1 ELSE 0 END) AS positive_count,
        SUM(CASE WHEN sentiment = 'neutral' THEN 1 ELSE 0 END) AS neutral_count,
        SUM(CASE WHEN sentiment = 'negative' THEN 1 ELSE 0 END) AS negative_count,
        SUM(CASE WHEN sentiment = 'mixed' THEN 1 ELSE 0 END) AS mixed_count,
        SUM(CASE WHEN cost_signal = 'cost_positive' THEN 1 ELSE 0 END) AS cost_positive_count,
        SUM(CASE WHEN cost_signal = 'cost_negative_boros' THEN 1 ELSE 0 END) AS cost_negative_boros_count,
        SUM(CASE WHEN cost_signal = 'cost_mixed' THEN 1 ELSE 0 END) AS cost_mixed_count,
        SUM(CASE WHEN cost_signal IN ('not_mentioned', 'none') THEN 1 ELSE 0 END) AS cost_not_mentioned_count,
        COUNT(DISTINCT source_name) AS source_count,
        MIN(observed_at) AS first_seen_at,
        MAX(observed_at) AS last_seen_at
    FROM base
    GROUP BY week_start, week_end, entity_id, canonical_name, entity_type, region
)
SELECT
    week_start,
    week_end,
    entity_id,
    canonical_name,
    entity_type,
    region,
    mention_count,
    positive_count,
    neutral_count,
    negative_count,
    mixed_count,
    cost_positive_count,
    cost_negative_boros_count,
    cost_mixed_count,
    cost_not_mentioned_count,
    source_count,
    first_seen_at,
    last_seen_at,
    -- Keep the existing MVP score unchanged; IMP-04 changes only the grouping identity.
    (mention_count * 10)
        + (positive_count * 3)
        + (mixed_count * 1)
        - (negative_count * 2)
        - (cost_negative_boros_count * 1) AS trend_score
FROM grouped;
"#;

pub fn configured_database_path() -> Result<PathBuf, String> {
    config::resolved_database_path()
}

pub fn initialize_database() -> Result<PathBuf, String> {
    let database_path = configured_database_path()?;
    initialize_database_at(&database_path)?;
    Ok(database_path)
}

pub fn check_database_health() -> Result<String, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let health_value: i32 = connection
        .query_row("SELECT 1", [], |row| row.get(0))
        .map_err(|error| format!("DuckDB health query failed: {error}"))?;

    if health_value == 1 {
        Ok(format!("ok: {}", database_path.display()))
    } else {
        Err(format!(
            "DuckDB returned unexpected health value: {health_value}"
        ))
    }
}

pub fn save_threads_raw_posts(posts: &[ThreadPostRaw]) -> Result<usize, String> {
    if posts.is_empty() {
        return Ok(0);
    }

    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;

    let mut saved_count = 0;
    {
        let mut statement = transaction
            .prepare(THREADS_POST_INSERT_SQL)
            .map_err(|error| format!("DuckDB insert preparation failed: {error}"))?;

        for post in posts {
            if post.post_id.trim().is_empty() {
                continue;
            }

            statement
                .execute(params![
                    &post.post_id,
                    &post.author_id,
                    &post.author_username,
                    &post.author_display_name,
                    &post.text,
                    post.text_missing,
                    &post.permalink,
                    &post.media_type,
                    &post.source_type,
                    &post.source_seed_keyword,
                    &post.keyword_match,
                    post.like_count,
                    post.reply_count,
                    post.repost_count,
                    post.quote_count,
                    post.share_count,
                    post.view_count,
                    &post.posted_at,
                    &post.raw_json
                ])
                .map_err(|error| format!("DuckDB raw post insert failed: {error}"))?;
            saved_count += 1;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(saved_count)
}

pub fn count_threads_raw_posts() -> Result<usize, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM threads_posts_raw", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("DuckDB raw post count query failed: {error}"))?;

    usize::try_from(count).map_err(|error| format!("DuckDB raw post count is invalid: {error}"))
}

pub fn reset_local_pipeline_data() -> Result<String, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;

    connection
        .execute("DELETE FROM agent_mentions", [])
        .map_err(|error| reset_error("agent mention", error))?;
    connection
        .execute("DELETE FROM weekly_agent_metrics", [])
        .map_err(|error| reset_error("weekly metrics", error))?;
    connection
        .execute("DELETE FROM weekly_entity_metrics", [])
        .map_err(|error| reset_error("canonical weekly metrics", error))?;
    if table_exists(&connection, "crawl_seed_results")? {
        connection
            .execute("DELETE FROM crawl_seed_results", [])
            .map_err(|error| reset_error("crawl seed result", error))?;
    }
    connection
        .execute("DELETE FROM crawl_runs", [])
        .map_err(|error| reset_error("crawl run", error))?;
    connection
        .execute("DELETE FROM threads_posts_raw", [])
        .map_err(|error| reset_error("raw post", error))?;

    Ok("Cleared local demo data: raw posts, mentions, crawl runs, weekly metrics, and canonical weekly metrics. Candidate decisions were preserved.".to_string())
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, String> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?1",
            params![table_name],
            |row| row.get(0),
        )
        .map_err(|error| format!("DuckDB table existence check failed: {error}"))?;

    Ok(count > 0)
}

pub fn save_crawl_run(result: &DiscoveryCrawlResult) -> Result<usize, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let status = if result.failed_seeds > 0 && result.saved_total == 0 {
        "needs_attention"
    } else if result.failed_seeds > 0 || result.zero_result_seeds > 0 {
        "completed_with_diagnostics"
    } else {
        "completed"
    };
    let error_summary = if result.last_error_summary.trim().is_empty() {
        result.errors.join(" | ")
    } else {
        result.last_error_summary.clone()
    };

    connection
        .execute(
            CRAWL_RUN_INSERT_SQL,
            params![
                &result.run_id,
                &result.mode,
                &result.seed_group,
                result.max_per_seed as i64,
                result.seeds_processed as i64,
                result.fetched_total as i64,
                result.saved_total as i64,
                result.duplicates_skipped as i64,
                result.zero_result_seeds as i64,
                result.failed_seeds as i64,
                result.detail_fetched_total as i64,
                result.detail_failed_total as i64,
                result.text_missing_total as i64,
                &result.started_at,
                &result.finished_at,
                result.duration_ms as i64,
                status,
                &error_summary,
            ],
        )
        .map_err(|error| format!("DuckDB crawl run insert failed: {error}"))
}

pub fn load_raw_posts_for_detection() -> Result<Vec<RawPostForDetection>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT post_id, text
            FROM threads_posts_raw
            WHERE text IS NOT NULL AND length(trim(text)) > 0
            ORDER BY collected_at DESC
            LIMIT 5000
            "#,
        )
        .map_err(|error| format!("DuckDB raw post query preparation failed: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(RawPostForDetection {
                post_id: row.get(0)?,
                text: row.get(1)?,
            })
        })
        .map_err(|error| format!("DuckDB raw post query failed: {error}"))?;

    let mut posts = Vec::new();
    for row in rows {
        posts.push(row.map_err(|error| format!("DuckDB raw post row read failed: {error}"))?);
    }

    Ok(posts)
}

pub fn load_agent_mentions_for_identity_linkage(
) -> Result<Vec<AgentMentionForIdentityLinkage>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                mention_id,
                agent_name,
                COALESCE(category, 'unknown'),
                COALESCE(source_snippet, '')
            FROM agent_mentions
            WHERE entity_id IS NULL
                OR identity_resolution_status IS NULL
                OR identity_resolution_status IN ('unresolved', 'missing_alias')
            ORDER BY detected_at, mention_id
            "#,
        )
        .map_err(|error| format!("DuckDB identity linkage query preparation failed: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(AgentMentionForIdentityLinkage {
                mention_id: row.get(0)?,
                agent_name: row.get(1)?,
                category: row.get(2)?,
                source_snippet: row.get(3)?,
            })
        })
        .map_err(|error| format!("DuckDB identity linkage query failed: {error}"))?;

    let mut mentions = Vec::new();
    for row in rows {
        mentions.push(
            row.map_err(|error| format!("DuckDB identity linkage row read failed: {error}"))?,
        );
    }
    Ok(mentions)
}

pub fn save_mention_identity_resolutions(
    resolutions: &[MentionIdentityResolution],
) -> Result<usize, String> {
    if resolutions.is_empty() {
        return Ok(0);
    }

    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB identity linkage transaction failed: {error}"))?;
    let mut updated_count = 0;

    {
        let mut statement = transaction
            .prepare(
                r#"
                UPDATE agent_mentions
                SET
                    entity_id = CASE
                        WHEN ?2 IS NULL THEN NULL
                        ELSE CAST(?2 AS UUID)
                    END,
                    identity_resolution_status = ?3,
                    identity_resolution_reason = ?4,
                    identity_resolution_confidence = ?5,
                    identity_resolved_at = CASE
                        WHEN ?3 = 'resolved' THEN CURRENT_TIMESTAMP
                        ELSE NULL
                    END
                WHERE mention_id = ?1
                "#,
            )
            .map_err(|error| {
                format!("DuckDB identity linkage update preparation failed: {error}")
            })?;

        for resolution in resolutions {
            updated_count += statement
                .execute(params![
                    &resolution.mention_id,
                    &resolution.entity_id,
                    resolution.status.as_str(),
                    &resolution.reason,
                    resolution.confidence,
                ])
                .map_err(|error| format!("DuckDB identity linkage update failed: {error}"))?;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("DuckDB identity linkage commit failed: {error}"))?;
    Ok(updated_count)
}

pub fn save_agent_mentions(mentions: &[DetectedAgentMention]) -> Result<usize, String> {
    if mentions.is_empty() {
        return Ok(0);
    }

    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;

    let mut saved_count = 0;
    {
        let mut statement = transaction
            .prepare(AGENT_MENTION_INSERT_SQL)
            .map_err(|error| format!("DuckDB mention insert preparation failed: {error}"))?;

        for mention in mentions {
            if mention.mention_id.trim().is_empty() || mention.post_id.trim().is_empty() {
                continue;
            }

            statement
                .execute(params![
                    &mention.mention_id,
                    &mention.post_id,
                    &mention.agent_name,
                    &mention.agent_alias,
                    &mention.category,
                    &mention.detection_source,
                    mention.needs_review,
                    &mention.region,
                    mention.confidence,
                    mention.match_confidence,
                    mention.relevance_score,
                    &mention.sentiment,
                    &mention.cost_signal,
                    &mention.source_snippet,
                    &mention.review_status,
                    &mention.reviewed_as,
                    &mention.reviewed_category,
                ])
                .map_err(|error| format!("DuckDB agent mention insert failed: {error}"))?;
            saved_count += 1;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(saved_count)
}

pub fn save_region_classifications(
    classifications: &[RegionClassification],
) -> Result<usize, String> {
    if classifications.is_empty() {
        return Ok(0);
    }

    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;

    let mut updated_mentions_count = 0;
    {
        let mut post_statement = transaction
            .prepare(THREADS_POST_REGION_UPDATE_SQL)
            .map_err(|error| format!("DuckDB post region update preparation failed: {error}"))?;
        let mut mention_statement = transaction
            .prepare(AGENT_MENTION_REGION_UPDATE_SQL)
            .map_err(|error| format!("DuckDB mention region update preparation failed: {error}"))?;

        for classification in classifications {
            if classification.post_id.trim().is_empty() {
                continue;
            }

            post_statement
                .execute(params![
                    &classification.post_id,
                    &classification.region,
                    classification.region_confidence,
                    &classification.region_reason,
                ])
                .map_err(|error| format!("DuckDB post region update failed: {error}"))?;

            updated_mentions_count += mention_statement
                .execute(params![
                    &classification.post_id,
                    &classification.region,
                    classification.region_confidence,
                    &classification.region_reason,
                ])
                .map_err(|error| format!("DuckDB mention region update failed: {error}"))?;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(updated_mentions_count)
}

pub fn load_agent_mentions_for_sentiment() -> Result<Vec<AgentMentionForSentiment>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT mention_id, COALESCE(source_snippet, '')
            FROM agent_mentions
            WHERE mention_id IS NOT NULL AND length(trim(mention_id)) > 0
            ORDER BY detected_at DESC
            LIMIT 5000
            "#,
        )
        .map_err(|error| format!("DuckDB sentiment mention query preparation failed: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(AgentMentionForSentiment {
                mention_id: row.get(0)?,
                source_snippet: row.get(1)?,
            })
        })
        .map_err(|error| format!("DuckDB sentiment mention query failed: {error}"))?;

    let mut mentions = Vec::new();
    for row in rows {
        mentions.push(
            row.map_err(|error| format!("DuckDB sentiment mention row read failed: {error}"))?,
        );
    }

    Ok(mentions)
}

pub fn save_sentiment_classifications(
    classifications: &[SentimentClassification],
) -> Result<usize, String> {
    if classifications.is_empty() {
        return Ok(0);
    }

    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;

    let mut updated_mentions_count = 0;
    {
        let mut statement = transaction
            .prepare(AGENT_MENTION_SENTIMENT_UPDATE_SQL)
            .map_err(|error| {
                format!("DuckDB mention sentiment update preparation failed: {error}")
            })?;

        for classification in classifications {
            if classification.mention_id.trim().is_empty() {
                continue;
            }

            updated_mentions_count += statement
                .execute(params![
                    &classification.mention_id,
                    &classification.sentiment,
                    classification.sentiment_confidence,
                    &classification.sentiment_reason,
                ])
                .map_err(|error| format!("DuckDB mention sentiment update failed: {error}"))?;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(updated_mentions_count)
}

pub fn load_agent_mentions_for_cost() -> Result<Vec<AgentMentionForCost>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT mention_id, COALESCE(source_snippet, '')
            FROM agent_mentions
            WHERE mention_id IS NOT NULL AND length(trim(mention_id)) > 0
            ORDER BY detected_at DESC
            LIMIT 5000
            "#,
        )
        .map_err(|error| format!("DuckDB cost mention query preparation failed: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(AgentMentionForCost {
                mention_id: row.get(0)?,
                source_snippet: row.get(1)?,
            })
        })
        .map_err(|error| format!("DuckDB cost mention query failed: {error}"))?;

    let mut mentions = Vec::new();
    for row in rows {
        mentions
            .push(row.map_err(|error| format!("DuckDB cost mention row read failed: {error}"))?);
    }

    Ok(mentions)
}

pub fn save_cost_classifications(classifications: &[CostClassification]) -> Result<usize, String> {
    if classifications.is_empty() {
        return Ok(0);
    }

    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;

    let mut updated_mentions_count = 0;
    {
        let mut statement = transaction
            .prepare(AGENT_MENTION_COST_UPDATE_SQL)
            .map_err(|error| format!("DuckDB mention cost update preparation failed: {error}"))?;

        for classification in classifications {
            if classification.mention_id.trim().is_empty() {
                continue;
            }

            updated_mentions_count += statement
                .execute(params![
                    &classification.mention_id,
                    &classification.cost_signal,
                    classification.cost_confidence,
                    &classification.cost_reason,
                ])
                .map_err(|error| format!("DuckDB mention cost update failed: {error}"))?;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(updated_mentions_count)
}

pub fn load_agent_mentions_preview(limit: usize) -> Result<Vec<AgentMentionPreview>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                agent_name,
                category,
                COALESCE(detection_source, 'known_alias'),
                COALESCE(needs_review, FALSE),
                region,
                COALESCE(region_confidence, 0.0),
                COALESCE(region_reason, ''),
                sentiment,
                COALESCE(sentiment_confidence, 0.0),
                COALESCE(sentiment_reason, ''),
                cost_signal,
                COALESCE(cost_confidence, 0.0),
                COALESCE(cost_reason, ''),
                match_confidence,
                COALESCE(source_snippet, '')
            FROM agent_mentions
            ORDER BY detected_at DESC, agent_name ASC
            LIMIT ?1
            "#,
        )
        .map_err(|error| format!("DuckDB mention preview query preparation failed: {error}"))?;

    let rows = statement
        .query_map(params![limit], |row| {
            Ok(AgentMentionPreview {
                agent_name: row.get(0)?,
                category: row.get(1)?,
                detection_source: row.get(2)?,
                needs_review: row.get(3)?,
                region: row.get(4)?,
                region_confidence: row.get(5)?,
                region_reason: row.get(6)?,
                sentiment: row.get(7)?,
                sentiment_confidence: row.get(8)?,
                sentiment_reason: row.get(9)?,
                cost_signal: row.get(10)?,
                cost_confidence: row.get(11)?,
                cost_reason: row.get(12)?,
                confidence: row.get(13)?,
                source_snippet: row.get(14)?,
            })
        })
        .map_err(|error| format!("DuckDB mention preview query failed: {error}"))?;

    let mut preview = Vec::new();
    for row in rows {
        preview
            .push(row.map_err(|error| format!("DuckDB mention preview row read failed: {error}"))?);
    }

    Ok(preview)
}

pub fn list_candidate_entities() -> Result<Vec<CandidateEntityReview>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            WITH candidates AS (
                SELECT
                    COALESCE(NULLIF(agent_alias, ''), agent_name) AS candidate_name,
                    COALESCE(
                        review_status,
                        CASE WHEN COALESCE(needs_review, FALSE) THEN 'pending' ELSE 'approved' END
                    ) AS current_status,
                    COALESCE(reviewed_as, '') AS reviewed_as,
                    COALESCE(reviewed_category, '') AS reviewed_category,
                    COALESCE(source_snippet, '') AS source_snippet,
                    detected_at
                FROM agent_mentions
                WHERE
                    COALESCE(detection_source, '') IN ('candidate_pattern', 'reviewed_candidate')
                    OR COALESCE(category, '') = 'unknown_candidate'
            )
            SELECT
                candidate_name,
                COUNT(*) AS mention_count,
                CAST(MIN(detected_at) AS VARCHAR) AS first_seen,
                CAST(MAX(detected_at) AS VARCHAR) AS latest_seen,
                COALESCE(MAX(current_status), 'pending') AS current_status,
                COALESCE(MAX(reviewed_as), '') AS reviewed_as,
                COALESCE(MAX(reviewed_category), '') AS reviewed_category,
                COALESCE(string_agg(DISTINCT source_snippet, '|||'), '') AS sample_snippets
            FROM candidates
            WHERE candidate_name IS NOT NULL AND length(trim(candidate_name)) > 0
            GROUP BY candidate_name
            ORDER BY
                CASE COALESCE(MAX(current_status), 'pending')
                    WHEN 'pending' THEN 0
                    WHEN 'approved' THEN 1
                    WHEN 'ignored' THEN 2
                    ELSE 3
                END,
                mention_count DESC,
                candidate_name ASC
            "#,
        )
        .map_err(|error| format!("DuckDB candidate query preparation failed: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            let mention_count: i64 = row.get(1)?;
            let snippets_text: String = row.get(7)?;
            Ok(CandidateEntityReview {
                candidate_name: row.get(0)?,
                mention_count: i64_to_usize(mention_count)?,
                first_seen: row.get(2)?,
                latest_seen: row.get(3)?,
                current_status: row.get(4)?,
                reviewed_as: row.get(5)?,
                reviewed_category: row.get(6)?,
                sample_snippets: split_sample_snippets(&snippets_text),
            })
        })
        .map_err(|error| format!("DuckDB candidate query failed: {error}"))?;

    let mut candidates = Vec::new();
    for row in rows {
        candidates.push(row.map_err(|error| format!("DuckDB candidate row read failed: {error}"))?);
    }

    Ok(candidates)
}

pub fn load_entity_review_decisions() -> Result<Vec<EntityReviewDecision>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                id,
                candidate_name,
                COALESCE(normalized_name, ''),
                COALESCE(category, ''),
                status,
                COALESCE(note, ''),
                CAST(created_at AS VARCHAR),
                CAST(updated_at AS VARCHAR)
            FROM entity_review_decisions
            ORDER BY updated_at DESC, candidate_name ASC
            "#,
        )
        .map_err(|error| format!("DuckDB decision query preparation failed: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(EntityReviewDecision {
                id: row.get(0)?,
                candidate_name: row.get(1)?,
                normalized_name: row.get(2)?,
                category: row.get(3)?,
                status: row.get(4)?,
                note: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|error| format!("DuckDB decision query failed: {error}"))?;

    let mut decisions = Vec::new();
    for row in rows {
        decisions.push(row.map_err(|error| format!("DuckDB decision row read failed: {error}"))?);
    }

    Ok(decisions)
}

pub fn approve_candidate_entity(
    candidate_name: &str,
    reviewed_as: &str,
    reviewed_category: &str,
    note: Option<String>,
) -> Result<usize, String> {
    validate_candidate_name(candidate_name)?;
    validate_reviewed_as(reviewed_as)?;
    validate_reviewed_category(reviewed_category)?;

    let decision_id = normalize_entity_decision_id(candidate_name)?;
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;
    let note = normalize_optional_note(note);
    upsert_entity_review_decision(
        &transaction,
        &decision_id,
        candidate_name,
        Some(reviewed_as),
        Some(reviewed_category),
        "approved",
        note.as_deref(),
    )?;

    let updated_count = transaction
        .execute(
            r#"
            UPDATE agent_mentions
            SET
                agent_name = ?2,
                category = ?3,
                needs_review = FALSE,
                detection_source = 'reviewed_candidate',
                review_status = 'approved',
                reviewed_as = ?2,
                reviewed_category = ?3,
                review_note = ?4,
                reviewed_at = CURRENT_TIMESTAMP
            WHERE lower(trim(COALESCE(NULLIF(agent_alias, ''), agent_name))) = lower(trim(?1))
                AND (
                    COALESCE(detection_source, '') IN ('candidate_pattern', 'reviewed_candidate')
                    OR COALESCE(category, '') = 'unknown_candidate'
                )
            "#,
            params![candidate_name, reviewed_as, reviewed_category, note],
        )
        .map_err(|error| format!("DuckDB candidate approval update failed: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(updated_count)
}

pub fn ignore_candidate_entity(
    candidate_name: &str,
    note: Option<String>,
) -> Result<usize, String> {
    validate_candidate_name(candidate_name)?;

    let decision_id = normalize_entity_decision_id(candidate_name)?;
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;
    let note = normalize_optional_note(note);
    upsert_entity_review_decision(
        &transaction,
        &decision_id,
        candidate_name,
        None,
        None,
        "ignored",
        note.as_deref(),
    )?;

    let updated_count = transaction
        .execute(
            r#"
            UPDATE agent_mentions
            SET
                needs_review = FALSE,
                review_status = 'ignored',
                review_note = ?2,
                reviewed_at = CURRENT_TIMESTAMP
            WHERE lower(trim(COALESCE(NULLIF(agent_alias, ''), agent_name))) = lower(trim(?1))
                AND (
                    COALESCE(detection_source, '') IN ('candidate_pattern', 'reviewed_candidate')
                    OR COALESCE(category, '') = 'unknown_candidate'
                )
            "#,
            params![candidate_name, note],
        )
        .map_err(|error| format!("DuckDB candidate ignore update failed: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(updated_count)
}

pub fn reset_candidate_review(candidate_name: &str) -> Result<usize, String> {
    validate_candidate_name(candidate_name)?;

    let decision_id = normalize_entity_decision_id(candidate_name)?;
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB transaction failed: {error}"))?;
    transaction
        .execute(
            "DELETE FROM entity_review_decisions WHERE id = ?1",
            params![decision_id],
        )
        .map_err(|error| format!("DuckDB decision reset failed: {error}"))?;

    let updated_count = transaction
        .execute(
            r#"
            UPDATE agent_mentions
            SET
                agent_name = COALESCE(NULLIF(agent_alias, ''), agent_name),
                category = 'unknown_candidate',
                detection_source = 'candidate_pattern',
                needs_review = TRUE,
                review_status = 'pending',
                reviewed_as = NULL,
                reviewed_category = NULL,
                review_note = NULL,
                reviewed_at = NULL
            WHERE lower(trim(COALESCE(NULLIF(agent_alias, ''), agent_name))) = lower(trim(?1))
                AND (
                    COALESCE(detection_source, '') IN ('candidate_pattern', 'reviewed_candidate')
                    OR COALESCE(category, '') = 'unknown_candidate'
                )
            "#,
            params![candidate_name],
        )
        .map_err(|error| format!("DuckDB candidate reset update failed: {error}"))?;

    transaction
        .commit()
        .map_err(|error| format!("DuckDB transaction commit failed: {error}"))?;

    Ok(updated_count)
}

pub fn rebuild_weekly_agent_metrics() -> Result<usize, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;

    connection
        .execute_batch(WEEKLY_AGENT_METRICS_RECREATE_SQL)
        .map_err(|error| format!("DuckDB weekly metrics table rebuild failed: {error}"))?;

    connection
        .execute(WEEKLY_AGENT_METRICS_INSERT_SQL, [])
        .map_err(|error| format!("DuckDB weekly metrics aggregation failed: {error}"))?;

    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM weekly_agent_metrics", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("DuckDB weekly metrics count query failed: {error}"))?;

    usize::try_from(count)
        .map_err(|error| format!("DuckDB weekly metrics count is invalid: {error}"))
}

pub fn rebuild_weekly_entity_metrics() -> Result<usize, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("DuckDB canonical weekly transaction failed: {error}"))?;

    transaction
        .execute("DELETE FROM weekly_entity_metrics", [])
        .map_err(|error| format!("DuckDB canonical weekly reset failed: {error}"))?;
    transaction
        .execute(WEEKLY_ENTITY_METRICS_INSERT_SQL, [])
        .map_err(|error| format!("DuckDB canonical weekly aggregation failed: {error}"))?;
    let count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM weekly_entity_metrics", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("DuckDB canonical weekly count failed: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("DuckDB canonical weekly commit failed: {error}"))?;

    usize::try_from(count)
        .map_err(|error| format!("DuckDB canonical weekly count is invalid: {error}"))
}

pub fn count_weekly_entity_metric_entities() -> Result<usize, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(DISTINCT entity_id) FROM weekly_entity_metrics",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("DuckDB canonical entity count failed: {error}"))?;
    usize::try_from(count)
        .map_err(|error| format!("DuckDB canonical entity count is invalid: {error}"))
}

pub fn count_identity_resolution_skips() -> Result<IdentityResolutionSkipCounts, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let counts: (i64, i64, i64, i64, i64) = connection
        .query_row(
            r#"
            SELECT
                COUNT(*) FILTER (
                    WHERE identity_resolution_status IS NULL
                        OR identity_resolution_status = 'unresolved'
                        OR (identity_resolution_status = 'resolved' AND entity_id IS NULL)
                ),
                COUNT(*) FILTER (WHERE identity_resolution_status = 'ambiguous'),
                COUNT(*) FILTER (WHERE identity_resolution_status = 'missing_alias'),
                COUNT(*) FILTER (WHERE identity_resolution_status = 'skipped'),
                COUNT(*) FILTER (
                    WHERE identity_resolution_status = 'resolved'
                        AND entity_id IS NOT NULL
                        AND NOT EXISTS (
                            SELECT 1
                            FROM canonical_entities entities
                            WHERE entities.entity_id = agent_mentions.entity_id
                                AND entities.status = 'active'
                        )
                )
            FROM agent_mentions
            "#,
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|error| format!("DuckDB identity skip count failed: {error}"))?;

    Ok(IdentityResolutionSkipCounts {
        unresolved: i64_to_usize(counts.0)
            .map_err(|error| format!("Invalid unresolved mention count: {error}"))?,
        ambiguous: i64_to_usize(counts.1)
            .map_err(|error| format!("Invalid ambiguous mention count: {error}"))?,
        missing_alias: i64_to_usize(counts.2)
            .map_err(|error| format!("Invalid missing alias count: {error}"))?,
        skipped: i64_to_usize(counts.3)
            .map_err(|error| format!("Invalid skipped mention count: {error}"))?,
        invalid_resolved: i64_to_usize(counts.4)
            .map_err(|error| format!("Invalid resolved reference count: {error}"))?,
    })
}

pub fn load_weekly_entity_metrics_by_region(
    region: &str,
    limit: usize,
) -> Result<Vec<WeeklyEntityMetric>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                CAST(id AS VARCHAR),
                CAST(week_start AS VARCHAR),
                CAST(week_end AS VARCHAR),
                CAST(entity_id AS VARCHAR),
                canonical_name,
                entity_type,
                region,
                mention_count,
                positive_count,
                neutral_count,
                negative_count,
                mixed_count,
                cost_positive_count,
                cost_negative_boros_count,
                cost_mixed_count,
                cost_not_mentioned_count,
                source_count,
                CAST(first_seen_at AS VARCHAR),
                CAST(last_seen_at AS VARCHAR),
                trend_score
            FROM weekly_entity_metrics
            WHERE region = ?1
                AND week_start = (SELECT MAX(week_start) FROM weekly_entity_metrics)
            ORDER BY trend_score DESC, mention_count DESC, canonical_name ASC
            LIMIT ?2
            "#,
        )
        .map_err(|error| format!("DuckDB canonical weekly query preparation failed: {error}"))?;
    let rows = statement
        .query_map(params![region, limit], |row| {
            Ok(WeeklyEntityMetric {
                rank: 0,
                id: row.get(0)?,
                week_start: row.get(1)?,
                week_end: row.get(2)?,
                entity_id: row.get(3)?,
                canonical_name: row.get(4)?,
                entity_type: row.get(5)?,
                region: row.get(6)?,
                mention_count: i64_to_usize(row.get(7)?)?,
                positive_count: i64_to_usize(row.get(8)?)?,
                neutral_count: i64_to_usize(row.get(9)?)?,
                negative_count: i64_to_usize(row.get(10)?)?,
                mixed_count: i64_to_usize(row.get(11)?)?,
                cost_positive_count: i64_to_usize(row.get(12)?)?,
                cost_negative_boros_count: i64_to_usize(row.get(13)?)?,
                cost_mixed_count: i64_to_usize(row.get(14)?)?,
                cost_not_mentioned_count: i64_to_usize(row.get(15)?)?,
                source_count: i64_to_usize(row.get(16)?)?,
                first_seen_at: row.get(17)?,
                last_seen_at: row.get(18)?,
                trend_score: row.get(19)?,
            })
        })
        .map_err(|error| format!("DuckDB canonical weekly query failed: {error}"))?;

    let mut metrics = Vec::new();
    for row in rows {
        let mut metric =
            row.map_err(|error| format!("DuckDB canonical weekly row read failed: {error}"))?;
        metric.rank = metrics.len() + 1;
        metrics.push(metric);
    }
    Ok(metrics)
}

pub fn load_weekly_agent_metrics_by_region(
    region: &str,
    limit: usize,
) -> Result<Vec<WeeklyAgentMetric>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                CAST(week_start AS VARCHAR),
                CAST(week_end AS VARCHAR),
                region,
                agent_name,
                category,
                mentions,
                positive_count,
                neutral_count,
                negative_count,
                mixed_count,
                cost_not_mentioned_count,
                cost_positive_count,
                cost_negative_boros_count,
                cost_mixed_count,
                positive_pct,
                negative_pct,
                cost_negative_boros_pct,
                trend_score
            FROM weekly_agent_metrics
            WHERE region = ?1
              AND week_start = (SELECT MAX(week_start) FROM weekly_agent_metrics)
            ORDER BY trend_score DESC, mentions DESC, agent_name ASC
            LIMIT ?2
            "#,
        )
        .map_err(|error| format!("DuckDB weekly metrics query preparation failed: {error}"))?;

    let rows = statement
        .query_map(params![region, limit], |row| {
            Ok(WeeklyAgentMetric {
                rank: 0,
                week_start: row.get(0)?,
                week_end: row.get(1)?,
                region: row.get(2)?,
                agent_name: row.get(3)?,
                category: row.get(4)?,
                mentions: i64_to_usize(row.get(5)?)?,
                positive_count: i64_to_usize(row.get(6)?)?,
                neutral_count: i64_to_usize(row.get(7)?)?,
                negative_count: i64_to_usize(row.get(8)?)?,
                mixed_count: i64_to_usize(row.get(9)?)?,
                cost_not_mentioned_count: i64_to_usize(row.get(10)?)?,
                cost_positive_count: i64_to_usize(row.get(11)?)?,
                cost_negative_boros_count: i64_to_usize(row.get(12)?)?,
                cost_mixed_count: i64_to_usize(row.get(13)?)?,
                positive_pct: row.get(14)?,
                negative_pct: row.get(15)?,
                cost_negative_boros_pct: row.get(16)?,
                trend_score: row.get(17)?,
            })
        })
        .map_err(|error| format!("DuckDB weekly metrics query failed: {error}"))?;

    let mut metrics = Vec::new();
    for row in rows {
        let mut metric =
            row.map_err(|error| format!("DuckDB weekly metrics row read failed: {error}"))?;
        metric.rank = metrics.len() + 1;
        metrics.push(metric);
    }

    Ok(metrics)
}

pub fn load_weekly_agent_metrics(limit: usize) -> Result<Vec<WeeklyAgentMetric>, String> {
    let database_path = initialize_database()?;
    let connection = open_connection(&database_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                CAST(week_start AS VARCHAR),
                CAST(week_end AS VARCHAR),
                region,
                agent_name,
                category,
                mentions,
                positive_count,
                neutral_count,
                negative_count,
                mixed_count,
                cost_not_mentioned_count,
                cost_positive_count,
                cost_negative_boros_count,
                cost_mixed_count,
                positive_pct,
                negative_pct,
                cost_negative_boros_pct,
                trend_score
            FROM weekly_agent_metrics
            WHERE week_start = (SELECT MAX(week_start) FROM weekly_agent_metrics)
            ORDER BY region ASC, trend_score DESC, mentions DESC, agent_name ASC
            LIMIT ?1
            "#,
        )
        .map_err(|error| {
            format!("DuckDB weekly metrics export query preparation failed: {error}")
        })?;

    let rows = statement
        .query_map(params![limit], |row| {
            Ok(WeeklyAgentMetric {
                rank: 0,
                week_start: row.get(0)?,
                week_end: row.get(1)?,
                region: row.get(2)?,
                agent_name: row.get(3)?,
                category: row.get(4)?,
                mentions: i64_to_usize(row.get(5)?)?,
                positive_count: i64_to_usize(row.get(6)?)?,
                neutral_count: i64_to_usize(row.get(7)?)?,
                negative_count: i64_to_usize(row.get(8)?)?,
                mixed_count: i64_to_usize(row.get(9)?)?,
                cost_not_mentioned_count: i64_to_usize(row.get(10)?)?,
                cost_positive_count: i64_to_usize(row.get(11)?)?,
                cost_negative_boros_count: i64_to_usize(row.get(12)?)?,
                cost_mixed_count: i64_to_usize(row.get(13)?)?,
                positive_pct: row.get(14)?,
                negative_pct: row.get(15)?,
                cost_negative_boros_pct: row.get(16)?,
                trend_score: row.get(17)?,
            })
        })
        .map_err(|error| format!("DuckDB weekly metrics export query failed: {error}"))?;

    let mut metrics = Vec::new();
    let mut current_region = String::new();
    let mut current_rank = 0;

    for row in rows {
        let mut metric =
            row.map_err(|error| format!("DuckDB weekly metrics export row read failed: {error}"))?;
        if metric.region != current_region {
            current_region = metric.region.clone();
            current_rank = 1;
        } else {
            current_rank += 1;
        }
        metric.rank = current_rank;
        metrics.push(metric);
    }

    Ok(metrics)
}

pub(crate) fn initialize_database_at(database_path: &Path) -> Result<(), String> {
    ensure_parent_directory(database_path)?;
    let connection = open_connection(database_path)?;
    remove_legacy_compatibility_object(&connection)?;
    run_schema_initialization(&connection)
}

fn remove_legacy_compatibility_object(connection: &Connection) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            "SELECT table_type FROM information_schema.tables WHERE lower(table_name) = lower(?1) LIMIT 1",
        )
        .map_err(|error| format!("DuckDB legacy schema inspection failed: {error}"))?;
    let mut rows = statement
        .query(params![LEGACY_COMPATIBILITY_OBJECT])
        .map_err(|error| format!("DuckDB legacy schema query failed: {error}"))?;
    let object_type = rows
        .next()
        .map_err(|error| format!("DuckDB legacy schema row read failed: {error}"))?
        .map(|row| row.get::<_, String>(0))
        .transpose()
        .map_err(|error| format!("DuckDB legacy schema type read failed: {error}"))?;
    drop(rows);
    drop(statement);

    let Some(object_type) = object_type else {
        return Ok(());
    };
    let drop_sql = if object_type.eq_ignore_ascii_case("VIEW") {
        "DROP VIEW IF EXISTS agent_mentions_compatible"
    } else {
        "DROP TABLE IF EXISTS agent_mentions_compatible"
    };

    connection
        .execute_batch(drop_sql)
        .map_err(|_| LEGACY_LOCAL_DATABASE_MESSAGE.to_string())
}

fn reset_error(operation: &str, error: duckdb::Error) -> String {
    if error.to_string().contains(LEGACY_COMPATIBILITY_OBJECT) {
        LEGACY_LOCAL_DATABASE_MESSAGE.to_string()
    } else {
        format!("DuckDB {operation} reset failed: {error}")
    }
}

fn i64_to_usize(value: i64) -> Result<usize, duckdb::Error> {
    usize::try_from(value).map_err(|error| duckdb::Error::ToSqlConversionFailure(Box::new(error)))
}

fn split_sample_snippets(snippets_text: &str) -> Vec<String> {
    snippets_text
        .split("|||")
        .map(str::trim)
        .filter(|snippet| !snippet.is_empty())
        .take(3)
        .map(ToString::to_string)
        .collect()
}

fn upsert_entity_review_decision(
    transaction: &Transaction<'_>,
    id: &str,
    candidate_name: &str,
    normalized_name: Option<&str>,
    category: Option<&str>,
    status: &str,
    note: Option<&str>,
) -> Result<(), String> {
    let updated_count = transaction
        .execute(
            r#"
            UPDATE entity_review_decisions
            SET
                candidate_name = ?2,
                normalized_name = ?3,
                category = ?4,
                status = ?5,
                note = ?6,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?1
            "#,
            params![id, candidate_name, normalized_name, category, status, note],
        )
        .map_err(|error| format!("DuckDB decision update failed: {error}"))?;

    if updated_count == 0 {
        transaction
            .execute(
                r#"
                INSERT INTO entity_review_decisions (
                    id,
                    candidate_name,
                    normalized_name,
                    category,
                    status,
                    note
                ) VALUES (
                    ?1,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6
                )
                "#,
                params![id, candidate_name, normalized_name, category, status, note],
            )
            .map_err(|error| format!("DuckDB decision insert failed: {error}"))?;
    }

    Ok(())
}

fn validate_candidate_name(candidate_name: &str) -> Result<(), String> {
    if candidate_name.trim().is_empty() {
        Err("Candidate name is required.".to_string())
    } else {
        Ok(())
    }
}

fn normalize_entity_decision_id(candidate_name: &str) -> Result<String, String> {
    let mut normalized = String::with_capacity(candidate_name.len());
    let mut previous_was_space = true;

    for character in candidate_name.chars() {
        if character.is_alphanumeric() {
            for lowercase in character.to_lowercase() {
                normalized.push(lowercase);
            }
            previous_was_space = false;
        } else if !previous_was_space {
            normalized.push(' ');
            previous_was_space = true;
        }
    }

    let normalized = normalized.trim().to_string();
    if normalized.is_empty() {
        Err("Candidate name cannot be normalized.".to_string())
    } else {
        Ok(normalized)
    }
}

fn validate_reviewed_as(reviewed_as: &str) -> Result<(), String> {
    if reviewed_as.trim().is_empty() {
        Err("Canonical reviewed_as name is required.".to_string())
    } else {
        Ok(())
    }
}

fn validate_reviewed_category(reviewed_category: &str) -> Result<(), String> {
    const ALLOWED_CATEGORIES: &[&str] = &[
        "coding_agent",
        "coding_assistant",
        "generic_agent_framework",
        "skill_or_mode",
        "mcp_or_connector",
        "registry_or_discovery",
        "app_builder",
        "unknown",
    ];

    if ALLOWED_CATEGORIES.contains(&reviewed_category) {
        Ok(())
    } else {
        Err(format!(
            "Invalid reviewed category: {reviewed_category}. Choose a supported entity category."
        ))
    }
}

fn normalize_optional_note(note: Option<String>) -> Option<String> {
    note.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn run_schema_initialization(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(SCHEMA_SQL)
        .map_err(|error| format!("DuckDB schema initialization failed: {error}"))?;
    connection
        .execute_batch(MULTI_SOURCE_SCHEMA_SQL)
        .map_err(|error| format!("DuckDB multi-source schema initialization failed: {error}"))?;
    connection
        .execute_batch(IDENTITY_PERSISTENCE_SCHEMA_SQL)
        .map_err(|error| format!("DuckDB identity schema initialization failed: {error}"))
}

#[cfg(test)]
pub(crate) fn initialize_legacy_schema_at(database_path: &Path) -> Result<(), String> {
    ensure_parent_directory(database_path)?;
    let connection = open_connection(database_path)?;
    connection
        .execute_batch(SCHEMA_SQL)
        .map_err(|error| format!("DuckDB legacy test schema initialization failed: {error}"))
}

#[cfg(test)]
pub(crate) fn initialize_imp01_schema_at(database_path: &Path) -> Result<(), String> {
    ensure_parent_directory(database_path)?;
    let connection = open_connection(database_path)?;
    connection
        .execute_batch(SCHEMA_SQL)
        .map_err(|error| format!("DuckDB legacy schema initialization failed: {error}"))?;
    connection
        .execute_batch(MULTI_SOURCE_SCHEMA_SQL)
        .map_err(|error| format!("DuckDB IMP-01 schema initialization failed: {error}"))
}

fn open_connection(database_path: &Path) -> Result<Connection, String> {
    Connection::open(database_path).map_err(|error| {
        format!(
            "DuckDB connection failed at {}: {error}",
            database_path.display()
        )
    })
}

fn ensure_parent_directory(database_path: &Path) -> Result<(), String> {
    if let Some(parent) = database_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to create database directory {}: {error}",
                    parent.display()
                )
            })?;
        }
    }

    Ok(())
}
