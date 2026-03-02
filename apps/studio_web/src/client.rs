use crate::models::{
    ApiErrorResponse, BenchmarkResponse, PipelineDescriptor, ReproducibilityResponse, RunProfile,
    RunResponse, SuiteResponse, VersionResponse,
};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct StudioClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug)]
pub enum ClientError {
    Transport(reqwest::Error),
    Decode {
        context: &'static str,
        body: String,
        source: serde_json::Error,
    },
    Backend {
        status: StatusCode,
        envelope: Option<ApiErrorResponse>,
        body: String,
    },
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(err) => write!(f, "backend transport error: {err}"),
            Self::Decode {
                context,
                body,
                source,
            } => {
                write!(
                    f,
                    "backend decode error ({context}): {source}; response body: {body}"
                )
            }
            Self::Backend {
                status,
                envelope,
                body,
            } => {
                if let Some(err) = envelope {
                    write!(
                        f,
                        "backend returned {} {}: {} ({})",
                        status.as_u16(),
                        err.error_code,
                        err.message,
                        err.api_version
                    )
                } else {
                    write!(
                        f,
                        "backend returned {} with non-standard error body: {}",
                        status.as_u16(),
                        body
                    )
                }
            }
        }
    }
}

impl std::error::Error for ClientError {}

impl StudioClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        Self {
            base_url,
            http: reqwest::Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn version(&self) -> Result<VersionResponse, ClientError> {
        self.request_json(Method::GET, "/api/version", None).await
    }

    pub async fn pipelines(&self) -> Result<Vec<PipelineDescriptor>, ClientError> {
        self.request_json(Method::GET, "/api/pipelines", None).await
    }

    pub async fn history(&self) -> Result<Vec<RunResponse>, ClientError> {
        self.request_json(Method::GET, "/api/history", None).await
    }

    pub async fn run_experiment(
        &self,
        experiment_id: &str,
        profile: RunProfile,
    ) -> Result<RunResponse, ClientError> {
        let path = format!("/api/run/{}", urlencoding::encode(experiment_id));
        self.request_json(Method::POST, &path, Some(json!({"profile": profile})))
            .await
    }

    pub async fn run_suite(&self, profile: RunProfile) -> Result<SuiteResponse, ClientError> {
        self.request_json(
            Method::POST,
            "/api/run-suite",
            Some(json!({"profile": profile})),
        )
        .await
    }

    pub async fn benchmark_experiment(
        &self,
        experiment_id: &str,
        profile: RunProfile,
        iterations: Option<usize>,
    ) -> Result<BenchmarkResponse, ClientError> {
        let path = format!("/api/benchmark/{}", urlencoding::encode(experiment_id));
        self.request_json(
            Method::POST,
            &path,
            Some(json!({
                "profile": profile,
                "iterations": iterations,
            })),
        )
        .await
    }

    pub async fn reproducibility_experiment(
        &self,
        experiment_id: &str,
        profile: RunProfile,
        iterations: Option<usize>,
        tolerance: Option<f64>,
    ) -> Result<ReproducibilityResponse, ClientError> {
        let path = format!(
            "/api/reproducibility/{}",
            urlencoding::encode(experiment_id)
        );
        self.request_json(
            Method::POST,
            &path,
            Some(json!({
                "profile": profile,
                "iterations": iterations,
                "tolerance": tolerance,
            })),
        )
        .await
    }

    async fn request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        payload: Option<Value>,
    ) -> Result<T, ClientError> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.http.request(method, &url);
        if let Some(payload) = payload {
            request = request.json(&payload);
        }

        let response = request.send().await.map_err(ClientError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(ClientError::Transport)?;

        if status.is_success() {
            serde_json::from_slice::<T>(&bytes).map_err(|source| ClientError::Decode {
                context: "success payload",
                body: String::from_utf8_lossy(&bytes).into_owned(),
                source,
            })
        } else {
            let envelope = serde_json::from_slice::<ApiErrorResponse>(&bytes).ok();
            Err(ClientError::Backend {
                status,
                envelope,
                body: String::from_utf8_lossy(&bytes).into_owned(),
            })
        }
    }
}
