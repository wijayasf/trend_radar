use crate::models::entities::{SentimentClassification, SentimentClassificationResult};
use crate::services::duckdb_service;

const PREVIEW_LIMIT: usize = 12;

pub fn classify_sentiments() -> Result<SentimentClassificationResult, String> {
    let mentions = duckdb_service::load_agent_mentions_for_sentiment()?;
    let classifications = mentions
        .iter()
        .map(|mention| {
            let mut classification = classify_text(&mention.source_snippet);
            classification.mention_id = mention.mention_id.clone();
            classification
        })
        .collect::<Vec<_>>();

    let updated_mentions_count = duckdb_service::save_sentiment_classifications(&classifications)?;
    let positive_count = count_sentiment(&classifications, "positive");
    let neutral_count = count_sentiment(&classifications, "neutral");
    let negative_count = count_sentiment(&classifications, "negative");
    let mixed_count = count_sentiment(&classifications, "mixed");
    let preview = duckdb_service::load_agent_mentions_preview(PREVIEW_LIMIT)?;

    Ok(SentimentClassificationResult {
        mentions_analyzed: classifications.len(),
        positive_count,
        neutral_count,
        negative_count,
        mixed_count,
        updated_mentions_count,
        message: format!(
            "Classified {} mentions and updated {} sentiment labels.",
            classifications.len(),
            updated_mentions_count
        ),
        preview,
    })
}

fn count_sentiment(classifications: &[SentimentClassification], sentiment: &str) -> usize {
    classifications
        .iter()
        .filter(|classification| classification.sentiment == sentiment)
        .count()
}

fn classify_text(text: &str) -> SentimentClassification {
    let normalized_text = normalize_text(text);

    if normalized_text.is_empty() {
        return classification("neutral", 0.5, "empty text has no sentiment signal");
    }

    let mixed_matches = matched_signals(&normalized_text, MIXED_INDICATORS);
    if !mixed_matches.is_empty() {
        return classification(
            "mixed",
            0.86,
            &format!(
                "matched mixed sentiment signals: {}",
                mixed_matches.join(", ")
            ),
        );
    }

    let positive_matches = matched_signals(&normalized_text, POSITIVE_INDICATORS);
    let negative_matches = matched_negative_signals(&normalized_text);

    match (positive_matches.is_empty(), negative_matches.is_empty()) {
        (false, false) => classification(
            "mixed",
            0.8,
            &format!(
                "matched positive and negative signals: positive={}, negative={}",
                positive_matches.join(", "),
                negative_matches.join(", ")
            ),
        ),
        (false, true) => classification(
            "positive",
            0.78,
            &format!("matched positive signals: {}", positive_matches.join(", ")),
        ),
        (true, false) => classification(
            "negative",
            0.78,
            &format!("matched negative signals: {}", negative_matches.join(", ")),
        ),
        (true, true) => classification("neutral", 0.62, "no clear opinion signal"),
    }
}

fn matched_signals(normalized_text: &str, signals: &[&'static str]) -> Vec<&'static str> {
    signals
        .iter()
        .copied()
        .filter(|signal| contains_phrase(normalized_text, signal))
        .collect()
}

fn matched_negative_signals(normalized_text: &str) -> Vec<&'static str> {
    NEGATIVE_INDICATORS
        .iter()
        .copied()
        .filter(|signal| {
            contains_phrase(normalized_text, signal)
                && (*signal != "expensive" || is_expensive_complaint(normalized_text))
        })
        .collect()
}

fn is_expensive_complaint(normalized_text: &str) -> bool {
    contains_phrase(normalized_text, "expensive")
        && !contains_phrase(normalized_text, "worth it")
        && (contains_phrase(normalized_text, "too expensive")
            || contains_phrase(normalized_text, "but expensive")
            || contains_phrase(normalized_text, "expensive")
            || contains_phrase(normalized_text, "not worth"))
}

fn classification(sentiment: &str, confidence: f64, reason: &str) -> SentimentClassification {
    SentimentClassification {
        mention_id: String::new(),
        sentiment: sentiment.to_string(),
        sentiment_confidence: confidence,
        sentiment_reason: reason.to_string(),
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

const POSITIVE_INDICATORS: &[&str] = &[
    "bagus", "enak", "helpful", "useful", "faster", "strong", "default", "cleaner", "worth it",
    "helps", "good",
];

const NEGATIVE_INDICATORS: &[&str] = &[
    "buruk",
    "jelek",
    "lambat",
    "error",
    "ngaco",
    "not good",
    "failed",
    "unreliable",
    "expensive",
];

const MIXED_INDICATORS: &[&str] = &[
    "bagus tapi",
    "helpful but",
    "good but",
    "useful but",
    "works but",
    "cepat tapi",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_positive_text() {
        let result = classify_text("Claude Code is useful and helpful for debugging.");

        assert_eq!(result.sentiment, "positive");
    }

    #[test]
    fn classifies_negative_text() {
        let result = classify_text("Cursor failed and feels unreliable.");

        assert_eq!(result.sentiment, "negative");
    }

    #[test]
    fn classifies_good_but_expensive_as_mixed() {
        let result = classify_text("This tool is good but expensive.");

        assert_eq!(result.sentiment, "mixed");
    }

    #[test]
    fn classifies_plain_mention_as_neutral() {
        let result = classify_text("OpenCode.");

        assert_eq!(result.sentiment, "neutral");
    }
}
