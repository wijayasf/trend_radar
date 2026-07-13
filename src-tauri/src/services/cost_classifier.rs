use crate::models::entities::{CostClassification, CostClassificationResult};
use crate::services::duckdb_service;

const PREVIEW_LIMIT: usize = 12;

pub fn classify_cost_signals() -> Result<CostClassificationResult, String> {
    let mentions = duckdb_service::load_agent_mentions_for_cost()?;
    let classifications = mentions
        .iter()
        .map(|mention| {
            let mut classification = classify_text(&mention.source_snippet);
            classification.mention_id = mention.mention_id.clone();
            classification
        })
        .collect::<Vec<_>>();

    let updated_mentions_count = duckdb_service::save_cost_classifications(&classifications)?;
    let not_mentioned_count = count_cost_signal(&classifications, "not_mentioned");
    let cost_positive_count = count_cost_signal(&classifications, "cost_positive");
    let cost_negative_boros_count = count_cost_signal(&classifications, "cost_negative_boros");
    let cost_mixed_count = count_cost_signal(&classifications, "cost_mixed");
    let preview = duckdb_service::load_agent_mentions_preview(PREVIEW_LIMIT)?;

    Ok(CostClassificationResult {
        mentions_analyzed: classifications.len(),
        not_mentioned_count,
        cost_positive_count,
        cost_negative_boros_count,
        cost_mixed_count,
        updated_mentions_count,
        message: format!(
            "Classified {} mentions and updated {} cost signals.",
            classifications.len(),
            updated_mentions_count
        ),
        preview,
    })
}

fn count_cost_signal(classifications: &[CostClassification], cost_signal: &str) -> usize {
    classifications
        .iter()
        .filter(|classification| classification.cost_signal == cost_signal)
        .count()
}

fn classify_text(text: &str) -> CostClassification {
    let normalized_text = normalize_text(text);

    if normalized_text.is_empty() {
        return classification("not_mentioned", 0.5, "empty text has no cost signal");
    }

    let mixed_matches = matched_signals(&normalized_text, COST_MIXED_INDICATORS);
    if !mixed_matches.is_empty() {
        return classification(
            "cost_mixed",
            0.88,
            &format!("matched mixed cost signals: {}", mixed_matches.join(", ")),
        );
    }

    let positive_matches = matched_signals(&normalized_text, COST_POSITIVE_INDICATORS);
    let negative_matches = matched_signals(&normalized_text, COST_NEGATIVE_INDICATORS);

    match (positive_matches.is_empty(), negative_matches.is_empty()) {
        (false, false) => classification(
            "cost_mixed",
            0.82,
            &format!(
                "matched positive and negative cost signals: positive={}, negative={}",
                positive_matches.join(", "),
                negative_matches.join(", ")
            ),
        ),
        (false, true) => classification(
            "cost_positive",
            0.78,
            &format!(
                "matched positive cost signals: {}",
                positive_matches.join(", ")
            ),
        ),
        (true, false) => classification(
            "cost_negative_boros",
            0.78,
            &format!(
                "matched negative cost signals: {}",
                negative_matches.join(", ")
            ),
        ),
        (true, true) => classification("not_mentioned", 0.62, "no clear cost/token/quota signal"),
    }
}

fn matched_signals(normalized_text: &str, signals: &[&'static str]) -> Vec<&'static str> {
    signals
        .iter()
        .copied()
        .filter(|signal| contains_phrase(normalized_text, signal))
        .collect()
}

fn classification(cost_signal: &str, confidence: f64, reason: &str) -> CostClassification {
    CostClassification {
        mention_id: String::new(),
        cost_signal: cost_signal.to_string(),
        cost_confidence: confidence,
        cost_reason: reason.to_string(),
    }
}

fn contains_phrase(normalized_text: &str, normalized_phrase: &str) -> bool {
    let searchable_text = format!(" {normalized_text} ");
    let searchable_phrase = format!(" {normalized_phrase} ");
    searchable_text.contains(&searchable_phrase)
}

fn normalize_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut previous_was_space = true;

    for character in text.chars() {
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

    normalized.trim().to_string()
}

const COST_NEGATIVE_INDICATORS: &[&str] = &[
    "boros",
    "mahal",
    "mahal banget",
    "token habis",
    "token cepat habis",
    "quota habis",
    "kuota habis",
    "limit",
    "rate limit",
    "expensive",
    "pricey",
    "costly",
    "burns token",
    "burn tokens",
    "token hungry",
    "usage limit",
    "too expensive",
];

const COST_POSITIVE_INDICATORS: &[&str] = &[
    "murah",
    "hemat",
    "worth it",
    "cost effective",
    "cheap",
    "affordable",
    "good value",
    "worth the price",
];

const COST_MIXED_INDICATORS: &[&str] = &[
    "bagus tapi mahal",
    "helpful but expensive",
    "useful but costly",
    "worth it but pricey",
    "good but burns tokens",
    "enak tapi boros",
    "bagus tapi boros token",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_bagus_tapi_boros_token_as_mixed() {
        let result = classify_text("Claude Code bagus tapi boros token.");

        assert_eq!(result.cost_signal, "cost_mixed");
    }

    #[test]
    fn classifies_mahal_banget_as_negative_boros() {
        let result = classify_text("Cursor mahal banget.");

        assert_eq!(result.cost_signal, "cost_negative_boros");
    }

    #[test]
    fn classifies_worth_it_as_cost_positive() {
        let result = classify_text("Copilot worth it.");

        assert_eq!(result.cost_signal, "cost_positive");
    }

    #[test]
    fn classifies_plain_workflow_as_not_mentioned() {
        let result = classify_text("Cline is useful for approval workflow.");

        assert_eq!(result.cost_signal, "not_mentioned");
    }
}
