use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::models::entities::CandidateEntityReview;
use crate::models::trend::{ReportExportResult, WeeklyAgentMetric};
use crate::services::duckdb_service;
use crate::utils::config;

const EXPORT_LIMIT: usize = 10_000;
const TOP_REGION_LIMIT: usize = 20;

#[derive(Debug, Clone)]
struct ReportSummary {
    week_start: String,
    week_end: String,
    total_mentions: usize,
    total_agents_detected: usize,
    regions_covered: Vec<String>,
    positive_count: usize,
    neutral_count: usize,
    negative_count: usize,
    mixed_count: usize,
    cost_not_mentioned_count: usize,
    cost_positive_count: usize,
    cost_negative_boros_count: usize,
    cost_mixed_count: usize,
}

pub fn export_weekly_report_markdown() -> Result<ReportExportResult, String> {
    let metrics = load_export_metrics()?;
    let candidates = duckdb_service::list_candidate_entities()?;
    let summary = summarize_metrics(&metrics);
    let markdown = render_markdown_report(&summary, &metrics, &candidates);
    let file_path = export_path(&summary, "weekly-report", "md")?;

    fs::write(&file_path, &markdown).map_err(|error| {
        format!(
            "Failed to write Markdown report {}: {error}",
            file_path.display()
        )
    })?;

    Ok(ReportExportResult {
        file_path: file_path.display().to_string(),
        rows_exported: metrics.len(),
        message: format!(
            "Exported Markdown weekly report with {} metric rows.",
            metrics.len()
        ),
        preview: preview_text(&markdown),
    })
}

pub fn export_weekly_metrics_csv() -> Result<ReportExportResult, String> {
    let metrics = load_export_metrics()?;
    let summary = summarize_metrics(&metrics);
    let csv = render_csv_metrics(&metrics);
    let file_path = export_path(&summary, "weekly-metrics", "csv")?;

    fs::write(&file_path, &csv).map_err(|error| {
        format!(
            "Failed to write CSV metrics {}: {error}",
            file_path.display()
        )
    })?;

    Ok(ReportExportResult {
        file_path: file_path.display().to_string(),
        rows_exported: metrics.len(),
        message: format!("Exported CSV weekly metrics with {} rows.", metrics.len()),
        preview: preview_text(&csv),
    })
}

fn load_export_metrics() -> Result<Vec<WeeklyAgentMetric>, String> {
    let metrics = duckdb_service::load_weekly_agent_metrics(EXPORT_LIMIT)?;
    if metrics.is_empty() {
        Err(
            "No weekly metrics available. Run Aggregate Weekly Metrics before exporting."
                .to_string(),
        )
    } else {
        Ok(metrics)
    }
}

fn summarize_metrics(metrics: &[WeeklyAgentMetric]) -> ReportSummary {
    let mut agents = HashSet::new();
    let mut regions = BTreeSet::new();
    let mut week_start = String::new();
    let mut week_end = String::new();

    let mut summary = ReportSummary {
        week_start: String::new(),
        week_end: String::new(),
        total_mentions: 0,
        total_agents_detected: 0,
        regions_covered: Vec::new(),
        positive_count: 0,
        neutral_count: 0,
        negative_count: 0,
        mixed_count: 0,
        cost_not_mentioned_count: 0,
        cost_positive_count: 0,
        cost_negative_boros_count: 0,
        cost_mixed_count: 0,
    };

    for metric in metrics {
        if week_start.is_empty() || metric.week_start.as_str() < week_start.as_str() {
            week_start = metric.week_start.clone();
        }
        if week_end.is_empty() || metric.week_end.as_str() > week_end.as_str() {
            week_end = metric.week_end.clone();
        }

        agents.insert(metric.agent_name.clone());
        regions.insert(metric.region.clone());
        summary.total_mentions += metric.mentions;
        summary.positive_count += metric.positive_count;
        summary.neutral_count += metric.neutral_count;
        summary.negative_count += metric.negative_count;
        summary.mixed_count += metric.mixed_count;
        summary.cost_not_mentioned_count += metric.cost_not_mentioned_count;
        summary.cost_positive_count += metric.cost_positive_count;
        summary.cost_negative_boros_count += metric.cost_negative_boros_count;
        summary.cost_mixed_count += metric.cost_mixed_count;
    }

    summary.week_start = week_start;
    summary.week_end = week_end;
    summary.total_agents_detected = agents.len();
    summary.regions_covered = regions.into_iter().collect();
    summary
}

fn render_markdown_report(
    summary: &ReportSummary,
    metrics: &[WeeklyAgentMetric],
    candidates: &[CandidateEntityReview],
) -> String {
    let mut markdown = String::new();

    markdown.push_str("# AI Agent Trend Radar Weekly Report\n\n");
    markdown.push_str("## Summary\n\n");
    markdown.push_str(&format!("- Week start: {}\n", summary.week_start));
    markdown.push_str(&format!("- Week end: {}\n", summary.week_end));
    markdown.push_str(&format!("- Total mentions: {}\n", summary.total_mentions));
    markdown.push_str(&format!(
        "- Total agents detected: {}\n",
        summary.total_agents_detected
    ));
    markdown.push_str(&format!(
        "- Regions covered: {}\n\n",
        summary.regions_covered.join(", ")
    ));

    markdown.push_str("## Top AI Agents - Indonesia\n\n");
    markdown.push_str(&render_region_table(metrics, "indonesia"));
    markdown.push_str("\n## Top AI Agents - Global\n\n");
    markdown.push_str(&render_region_table(metrics, "global"));

    if metrics.iter().any(|metric| metric.region == "unknown") {
        markdown.push_str("\n## Unknown Region\n\n");
        markdown.push_str(&render_region_table(metrics, "unknown"));
    }

    markdown.push_str("\n## Sentiment Overview\n\n");
    markdown.push_str(&format!("- Positive: {}\n", summary.positive_count));
    markdown.push_str(&format!("- Neutral: {}\n", summary.neutral_count));
    markdown.push_str(&format!("- Negative: {}\n", summary.negative_count));
    markdown.push_str(&format!("- Mixed: {}\n", summary.mixed_count));

    markdown.push_str("\n## Cost / Boros Overview\n\n");
    markdown.push_str(&format!(
        "- Not mentioned: {}\n",
        summary.cost_not_mentioned_count
    ));
    markdown.push_str(&format!(
        "- Cost positive: {}\n",
        summary.cost_positive_count
    ));
    markdown.push_str(&format!(
        "- Cost negative / boros: {}\n",
        summary.cost_negative_boros_count
    ));
    markdown.push_str(&format!("- Cost mixed: {}\n", summary.cost_mixed_count));

    markdown.push_str("\n## Candidate Review Notes\n\n");
    markdown.push_str(&render_candidate_review_notes(candidates));

    markdown.push_str("\n## Research Notes\n\n");
    markdown.push_str(
        "- Threads signal represents public conversation signal, not actual usage telemetry.\n",
    );
    markdown.push_str("- Internal recommendation should be validated with benchmark and PoC.\n");
    markdown.push_str("- Trend score uses the current MVP formula from weekly aggregation.\n");

    markdown
}

fn render_candidate_review_notes(candidates: &[CandidateEntityReview]) -> String {
    if candidates.is_empty() {
        return "No candidate review notes yet.\n".to_string();
    }

    let mut notes = String::new();
    append_candidate_group(&mut notes, "Pending candidates", candidates, "pending");
    append_candidate_group(&mut notes, "Approved candidates", candidates, "approved");
    append_candidate_group(&mut notes, "Ignored candidates", candidates, "ignored");
    notes
}

fn append_candidate_group(
    notes: &mut String,
    title: &str,
    candidates: &[CandidateEntityReview],
    status: &str,
) {
    notes.push_str(&format!("{title}:\n"));
    let mut matched = false;

    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.current_status == status)
    {
        matched = true;
        let suffix = if candidate.reviewed_as.is_empty() {
            String::new()
        } else {
            format!(" as {}", candidate.reviewed_as)
        };
        notes.push_str(&format!(
            "- {}{} ({} mentions)\n",
            candidate.candidate_name, suffix, candidate.mention_count
        ));
    }

    if !matched {
        notes.push_str("- none\n");
    }
}

fn render_region_table(metrics: &[WeeklyAgentMetric], region: &str) -> String {
    let region_metrics: Vec<&WeeklyAgentMetric> = metrics
        .iter()
        .filter(|metric| metric.region == region)
        .take(TOP_REGION_LIMIT)
        .collect();

    if region_metrics.is_empty() {
        return "No metrics available.\n".to_string();
    }

    let mut table = String::from(
        "| Rank | Agent | Category | Mentions | Positive % | Negative % | Boros % | Trend Score |\n| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |\n",
    );

    for metric in region_metrics {
        table.push_str(&format!(
            "| {} | {} | {} | {} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
            metric.rank,
            escape_markdown_cell(&metric.agent_name),
            escape_markdown_cell(&metric.category),
            metric.mentions,
            metric.positive_pct,
            metric.negative_pct,
            metric.cost_negative_boros_pct,
            metric.trend_score
        ));
    }

    table
}

fn render_csv_metrics(metrics: &[WeeklyAgentMetric]) -> String {
    let mut csv = String::from(
        "week_start,week_end,region,rank,agent_name,category,mentions,positive_pct,negative_pct,cost_negative_boros_pct,trend_score\n",
    );

    for metric in metrics {
        let row = [
            metric.week_start.clone(),
            metric.week_end.clone(),
            metric.region.clone(),
            metric.rank.to_string(),
            metric.agent_name.clone(),
            metric.category.clone(),
            metric.mentions.to_string(),
            format!("{:.2}", metric.positive_pct),
            format!("{:.2}", metric.negative_pct),
            format!("{:.2}", metric.cost_negative_boros_pct),
            format!("{:.2}", metric.trend_score),
        ];

        csv.push_str(
            &row.iter()
                .map(|value| escape_csv_cell(value))
                .collect::<Vec<String>>()
                .join(","),
        );
        csv.push('\n');
    }

    csv
}

fn export_path(summary: &ReportSummary, prefix: &str, extension: &str) -> Result<PathBuf, String> {
    let export_dir = export_directory();
    fs::create_dir_all(&export_dir).map_err(|error| {
        format!(
            "Failed to create report export directory {}: {error}",
            export_dir.display()
        )
    })?;

    Ok(export_dir.join(format!(
        "{prefix}-{}-to-{}.{}",
        safe_file_part(&summary.week_start),
        safe_file_part(&summary.week_end),
        extension
    )))
}

fn export_directory() -> PathBuf {
    config::project_root().join("data").join("exports")
}

fn safe_file_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn escape_csv_cell(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn preview_text(content: &str) -> String {
    content.chars().take(600).collect()
}
