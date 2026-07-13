use crate::models::entities::SentimentClassificationResult;
use crate::services::sentiment_classifier;

#[tauri::command]
pub fn classify_sentiments() -> Result<SentimentClassificationResult, String> {
    sentiment_classifier::classify_sentiments()
}
