use crate::models::entities::RegionClassificationResult;
use crate::services::region_classifier;

#[tauri::command]
pub fn classify_regions() -> Result<RegionClassificationResult, String> {
    region_classifier::classify_regions()
}
