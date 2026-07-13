use std::fs;
use std::path::{Path, PathBuf};

use duckdb::params;
use duckdb::Connection;

use crate::models::entities::{
    AgentMentionForCost, AgentMentionForSentiment, AgentMentionPreview, CostClassification,
    DetectedAgentMention, RawPostForDetection, RegionClassification, SentimentClassification,
};
use crate::models::threads::ThreadPostRaw;
use crate::models::trend::WeeklyAgentMetric;
use crate::utils::config;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS threads_posts_raw (
    post_id TEXT PRIMARY KEY,
    thread_id TEXT,
    author_id TEXT,
    author_username TEXT,
    text TEXT NOT NULL,
    permalink TEXT,
    media_type TEXT,
    language TEXT,
    region_hint TEXT,
    region_confidence DOUBLE DEFAULT 0.0,
    region_reason TEXT,
    like_count BIGINT DEFAULT 0,
    reply_count BIGINT DEFAULT 0,
    repost_count BIGINT DEFAULT 0,
    quote_count BIGINT DEFAULT 0,
    posted_at TIMESTAMP,
    collected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    raw_json TEXT
);

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS media_type TEXT;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS region_confidence DOUBLE DEFAULT 0.0;

ALTER TABLE threads_posts_raw
    ADD COLUMN IF NOT EXISTS region_reason TEXT;

CREATE TABLE IF NOT EXISTS agent_mentions (
    mention_id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL,
    agent_name TEXT NOT NULL,
    agent_alias TEXT,
    category TEXT DEFAULT 'unknown',
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
        'unknown'
    )),
    CHECK (region IN ('indonesia', 'global', 'unknown'))
);

ALTER TABLE agent_mentions
    ADD COLUMN IF NOT EXISTS category TEXT DEFAULT 'unknown';

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

const THREADS_POST_INSERT_SQL: &str = r#"
INSERT OR REPLACE INTO threads_posts_raw (
    post_id,
    thread_id,
    author_id,
    author_username,
    text,
    permalink,
    media_type,
    language,
    region_hint,
    like_count,
    reply_count,
    repost_count,
    quote_count,
    posted_at,
    raw_json
) VALUES (
    ?1,
    NULL,
    NULL,
    NULL,
    ?2,
    ?3,
    ?4,
    NULL,
    NULL,
    0,
    0,
    0,
    0,
    TRY_CAST(?5 AS TIMESTAMP),
    ?6
);
"#;

const AGENT_MENTION_INSERT_SQL: &str = r#"
INSERT OR REPLACE INTO agent_mentions (
    mention_id,
    post_id,
    agent_name,
    agent_alias,
    category,
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
    ?8,
    ?9,
    ?10,
    ?11,
    ?12
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
        m.agent_name,
        COALESCE(m.category, 'unknown') AS category,
        COALESCE(m.sentiment, 'unknown') AS sentiment,
        COALESCE(m.cost_signal, 'not_mentioned') AS cost_signal
    FROM agent_mentions m
    JOIN threads_posts_raw p ON p.post_id = m.post_id
    WHERE m.agent_name IS NOT NULL AND length(trim(m.agent_name)) > 0
),
grouped AS (
    SELECT
        week_start,
        CAST(week_start + INTERVAL 6 DAY AS DATE) AS week_end,
        region,
        agent_name,
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
    GROUP BY week_start, week_end, region, agent_name, category
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
                    &post.text,
                    &post.permalink,
                    &post.media_type,
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
                    &mention.region,
                    mention.confidence,
                    mention.match_confidence,
                    mention.relevance_score,
                    &mention.sentiment,
                    &mention.cost_signal,
                    &mention.source_snippet,
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
                region: row.get(2)?,
                region_confidence: row.get(3)?,
                region_reason: row.get(4)?,
                sentiment: row.get(5)?,
                sentiment_confidence: row.get(6)?,
                sentiment_reason: row.get(7)?,
                cost_signal: row.get(8)?,
                cost_confidence: row.get(9)?,
                cost_reason: row.get(10)?,
                confidence: row.get(11)?,
                source_snippet: row.get(12)?,
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

fn initialize_database_at(database_path: &Path) -> Result<(), String> {
    ensure_parent_directory(database_path)?;
    let connection = open_connection(database_path)?;
    run_schema_initialization(&connection)
}

fn i64_to_usize(value: i64) -> Result<usize, duckdb::Error> {
    usize::try_from(value).map_err(|error| duckdb::Error::ToSqlConversionFailure(Box::new(error)))
}

fn run_schema_initialization(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(SCHEMA_SQL)
        .map_err(|error| format!("DuckDB schema initialization failed: {error}"))
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
