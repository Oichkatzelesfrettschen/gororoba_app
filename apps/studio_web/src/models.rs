use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDescriptor {
    pub id: String,
    pub title: String,
    pub hypothesis: String,
    pub primary_metric: String,
    pub quick_profile: String,
    pub full_profile: String,
    pub experiment_id: String,
    pub lineage_id: String,
    pub registry_binary: String,
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunProfile {
    #[default]
    Quick,
    Full,
}

impl RunProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResponse {
    pub api_version: String,
    pub run_id: u64,
    pub unix_seconds: u64,
    pub experiment_id: String,
    pub source_experiment_id: Option<String>,
    pub source_lineage_id: Option<String>,
    pub artifact_links: Vec<String>,
    pub profile: RunProfile,
    pub duration_ms: u128,
    pub thesis_id: usize,
    pub label: String,
    pub metric_value: f64,
    pub threshold: f64,
    pub passes_gate: bool,
    pub config_snapshot: Value,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunFailure {
    pub api_version: String,
    pub experiment_id: String,
    pub profile: RunProfile,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteResponse {
    pub api_version: String,
    pub profile: RunProfile,
    pub total_duration_ms: u128,
    pub pass_count: usize,
    pub fail_count: usize,
    pub success_rate: f64,
    pub results: Vec<RunResponse>,
    pub failures: Vec<RunFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResponse {
    pub api_version: String,
    pub experiment_id: String,
    pub profile: RunProfile,
    pub iterations_requested: usize,
    pub iterations_completed: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub mean_duration_ms: f64,
    pub median_duration_ms: f64,
    pub min_duration_ms: u128,
    pub max_duration_ms: u128,
    pub mean_metric_value: f64,
    pub metric_stddev: f64,
    pub runs: Vec<RunResponse>,
    pub failures: Vec<RunFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityResponse {
    pub api_version: String,
    pub experiment_id: String,
    pub profile: RunProfile,
    pub iterations_requested: usize,
    pub iterations_completed: usize,
    pub tolerance: f64,
    pub baseline_metric_value: f64,
    pub max_metric_delta: f64,
    pub gate_consistent: bool,
    pub stable: bool,
    pub runs: Vec<RunResponse>,
    pub failures: Vec<RunFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    pub api_version: String,
    pub service: String,
    pub package_version: String,
    pub catalog_source: String,
    pub pipeline_count: usize,
    pub catalog_warnings: Vec<String>,
    pub registry_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub api_version: String,
    pub error_code: String,
    pub message: String,
    pub known_ids: Vec<String>,
    pub details: Value,
}

#[derive(Debug, Deserialize)]
pub struct ProfileForm {
    pub profile: Option<RunProfile>,
}

#[derive(Debug, Deserialize)]
pub struct BenchmarkForm {
    pub profile: Option<RunProfile>,
    pub iterations: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ReproForm {
    pub profile: Option<RunProfile>,
    pub iterations: Option<usize>,
    pub tolerance: Option<f64>,
}
