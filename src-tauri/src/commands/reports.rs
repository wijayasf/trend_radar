use crate::models::trend::ReportExportResult;
use crate::services::report_exporter;

#[tauri::command]
pub fn export_weekly_report_markdown() -> Result<ReportExportResult, String> {
    report_exporter::export_weekly_report_markdown()
}

#[tauri::command]
pub fn export_weekly_metrics_csv() -> Result<ReportExportResult, String> {
    report_exporter::export_weekly_metrics_csv()
}
