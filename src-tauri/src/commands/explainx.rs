use crate::models::explainx::ExplainXImportResult;
use crate::services::explainx_importer;

#[tauri::command]
pub fn import_explainx_records(file_path: String) -> Result<ExplainXImportResult, String> {
    explainx_importer::import_explainx_records(file_path)
}
