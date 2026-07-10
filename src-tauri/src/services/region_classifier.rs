use crate::models::entities::{RegionClassification, RegionClassificationResult};
use crate::services::duckdb_service;

const PREVIEW_LIMIT: usize = 12;

pub fn classify_regions() -> Result<RegionClassificationResult, String> {
    let posts = duckdb_service::load_raw_posts_for_detection()?;
    let classifications = posts
        .iter()
        .map(|post| {
            let mut classification = classify_text(&post.text);
            classification.post_id = post.post_id.clone();
            classification
        })
        .collect::<Vec<_>>();

    let updated_mentions_count = duckdb_service::save_region_classifications(&classifications)?;
    let indonesia_count = classifications
        .iter()
        .filter(|classification| classification.region == "indonesia")
        .count();
    let global_count = classifications
        .iter()
        .filter(|classification| classification.region == "global")
        .count();
    let unknown_count = classifications
        .iter()
        .filter(|classification| classification.region == "unknown")
        .count();
    let preview = duckdb_service::load_agent_mentions_preview(PREVIEW_LIMIT)?;

    Ok(RegionClassificationResult {
        posts_analyzed: classifications.len(),
        indonesia_count,
        global_count,
        unknown_count,
        updated_mentions_count,
        message: format!(
            "Classified {} posts and updated {} agent mentions.",
            classifications.len(),
            updated_mentions_count
        ),
        preview,
    })
}

fn classify_text(text: &str) -> RegionClassification {
    let normalized_text = normalize_text(text);
    let token_count = normalized_text.split_whitespace().count();

    if normalized_text.is_empty() || token_count <= 2 {
        return classification(
            "unknown",
            0.35,
            "too little text for confident region classification",
        );
    }

    let indonesia_matches = matched_indonesia_signals(&normalized_text);
    if !indonesia_matches.is_empty() {
        let strong_signal_count = indonesia_matches
            .iter()
            .filter(|signal| is_strong_indonesia_signal(signal))
            .count();
        let confidence = if strong_signal_count > 0 {
            0.86
        } else if indonesia_matches.len() >= 2 {
            0.78
        } else {
            0.62
        };

        if confidence >= 0.7 {
            return classification(
                "indonesia",
                confidence,
                &format!(
                    "matched Indonesia signals: {}",
                    indonesia_matches.join(", ")
                ),
            );
        }
    }

    let english_score = english_context_score(&normalized_text);
    if english_score >= 2 || token_count >= 8 {
        return classification(
            "global",
            if english_score >= 2 { 0.74 } else { 0.64 },
            "no Indonesia signals and enough English/global context",
        );
    }

    classification(
        "unknown",
        0.45,
        "no strong Indonesia or global context signals",
    )
}

fn matched_indonesia_signals(normalized_text: &str) -> Vec<&'static str> {
    INDONESIA_SIGNALS
        .iter()
        .copied()
        .filter(|signal| contains_phrase(normalized_text, signal))
        .collect()
}

fn is_strong_indonesia_signal(signal: &str) -> bool {
    matches!(
        signal,
        "indonesia"
            | "indo"
            | "jakarta"
            | "bandung"
            | "surabaya"
            | "rupiah"
            | "mahal banget"
            | "anak dev indo"
            | "programmer indonesia"
            | "developer indonesia"
            | "kantor gue"
    )
}

fn english_context_score(normalized_text: &str) -> usize {
    ENGLISH_CONTEXT_SIGNALS
        .iter()
        .filter(|signal| contains_phrase(normalized_text, signal))
        .count()
}

fn classification(region: &str, confidence: f64, reason: &str) -> RegionClassification {
    RegionClassification {
        post_id: String::new(),
        region: region.to_string(),
        region_confidence: confidence,
        region_reason: reason.to_string(),
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

const INDONESIA_SIGNALS: &[&str] = &[
    "indonesia",
    "indo",
    "jakarta",
    "bandung",
    "surabaya",
    "rupiah",
    "mahal banget",
    "anak dev indo",
    "programmer indonesia",
    "developer indonesia",
    "gw",
    "gue",
    "lo",
    "kantor gue",
    "boros token",
    "ngebantu",
    "tapi",
    "pakai",
    "buat",
];

const ENGLISH_CONTEXT_SIGNALS: &[&str] = &[
    "is",
    "for",
    "when",
    "default",
    "coding",
    "agent",
    "workflow",
    "workflows",
    "global",
    "developer",
    "developers",
    "terminal",
    "repo",
    "support",
    "autocomplete",
    "discovery",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_indonesia_text() {
        let result = classify_text("Programmer Indonesia di Jakarta mulai pakai Claude Code.");

        assert_eq!(result.region, "indonesia");
        assert!(result.region_confidence >= 0.8);
    }

    #[test]
    fn classifies_english_text_as_global() {
        let result =
            classify_text("Claude Code is becoming my default coding agent for repo refactors.");

        assert_eq!(result.region, "global");
    }

    #[test]
    fn keeps_short_ambiguous_text_unknown() {
        let result = classify_text("OpenCode");

        assert_eq!(result.region, "unknown");
    }

    #[test]
    fn classifies_mixed_indo_english_as_indonesia() {
        let result = classify_text("Claude Code boros token tapi ngebantu debug.");

        assert_eq!(result.region, "indonesia");
    }
}
