use crate::client::{ClientError, StudioClient};
use crate::models::{
    BenchmarkForm, BenchmarkResponse, ProfileForm, ReproForm, ReproducibilityResponse, RunResponse,
    SuiteResponse,
};
use crate::views::{
    ErrorTemplate, IndexTemplate, PipelineLessonCard, PipelineTemplate, render_template,
};
use axum::extract::{Form, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use gororoba_shared_core::{LearningMode, lesson_for_pipeline};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub client: StudioClient,
    shell_cache: Arc<RwLock<Option<CachedShellData>>>,
    cache_ttl: Duration,
}

#[derive(Clone)]
struct ShellData {
    version: crate::models::VersionResponse,
    pipelines: Vec<crate::models::PipelineDescriptor>,
    history: Vec<RunResponse>,
}

#[derive(Clone)]
struct CachedShellData {
    fetched_at: Instant,
    data: ShellData,
}

impl AppState {
    pub fn new(client: StudioClient) -> Self {
        Self::with_cache_ttl(client, cache_ttl_from_env())
    }

    pub fn with_cache_ttl(client: StudioClient, cache_ttl: Duration) -> Self {
        Self {
            client,
            shell_cache: Arc::new(RwLock::new(None)),
            cache_ttl,
        }
    }

    async fn invalidate_shell_cache(&self) {
        let mut cache = self.shell_cache.write().await;
        *cache = None;
    }

    async fn load_shell_data_cached(&self) -> Result<ShellData, ClientError> {
        {
            let cache = self.shell_cache.read().await;
            if let Some(entry) = cache.as_ref()
                && entry.fetched_at.elapsed() <= self.cache_ttl
            {
                return Ok(entry.data.clone());
            }
        }

        let fresh_data = fetch_shell_data(&self.client).await?;
        let mut cache = self.shell_cache.write().await;
        *cache = Some(CachedShellData {
            fetched_at: Instant::now(),
            data: fresh_data.clone(),
        });
        Ok(fresh_data)
    }
}

fn cache_ttl_from_env() -> Duration {
    const DEFAULT_MS: u64 = 1500;
    let ttl_ms = std::env::var("GOROROBA_UI_CACHE_TTL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MS);
    Duration::from_millis(ttl_ms)
}

#[derive(Debug, Default, Deserialize)]
struct LearningModeQuery {
    mode: Option<String>,
}

impl LearningModeQuery {
    fn parsed_mode(&self) -> LearningMode {
        LearningMode::from_optional_query(self.mode.as_deref())
    }
}

fn mode_flags(mode: LearningMode) -> (bool, bool, bool) {
    (
        mode == LearningMode::Story,
        mode == LearningMode::Explorer,
        mode == LearningMode::Research,
    )
}

struct PipelineRenderInput {
    mode: LearningMode,
    run: Option<RunResponse>,
    benchmark: Option<BenchmarkResponse>,
    reproducibility: Option<ReproducibilityResponse>,
    action_error: Option<String>,
    success_status: StatusCode,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/run-suite", post(run_suite))
        .route("/healthz", get(healthz))
        .route("/assets/app.css", get(app_css))
        .route("/assets/app.js", get(app_js))
        .route("/pipeline/{experiment_id}", get(pipeline_page))
        .route("/pipeline/{experiment_id}/run", post(run_pipeline))
        .route(
            "/pipeline/{experiment_id}/benchmark",
            post(benchmark_pipeline),
        )
        .route(
            "/pipeline/{experiment_id}/reproducibility",
            post(repro_pipeline),
        )
        .with_state(state)
}

async fn app_css() -> impl IntoResponse {
    const APP_CSS: &str = include_str!("../assets/app.css");
    (
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        APP_CSS,
    )
}

async fn app_js() -> impl IntoResponse {
    const APP_JS: &str = include_str!("../assets/app.js");
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/javascript; charset=utf-8",
        )],
        APP_JS,
    )
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "frontend": "gororoba_studio_web",
    }))
}

async fn home(State(state): State<AppState>, Query(query): Query<LearningModeQuery>) -> Response {
    render_index_view(&state, query.parsed_mode(), None, None, StatusCode::OK).await
}

async fn run_suite(
    State(state): State<AppState>,
    Query(query): Query<LearningModeQuery>,
    Form(form): Form<ProfileForm>,
) -> Response {
    let mode = query.parsed_mode();
    let profile = form.profile.unwrap_or_default();
    match state.client.run_suite(profile).await {
        Ok(suite) => {
            state.invalidate_shell_cache().await;
            render_index_view(&state, mode, Some(suite), None, StatusCode::OK).await
        }
        Err(error) => {
            state.invalidate_shell_cache().await;
            render_index_view(
                &state,
                mode,
                None,
                Some(format!("Suite action failed: {error}")),
                StatusCode::BAD_GATEWAY,
            )
            .await
        }
    }
}

async fn pipeline_page(
    Path(experiment_id): Path<String>,
    State(state): State<AppState>,
    Query(query): Query<LearningModeQuery>,
) -> Response {
    render_pipeline_view(
        &state,
        &experiment_id,
        PipelineRenderInput {
            mode: query.parsed_mode(),
            run: None,
            benchmark: None,
            reproducibility: None,
            action_error: None,
            success_status: StatusCode::OK,
        },
    )
    .await
}

async fn run_pipeline(
    Path(experiment_id): Path<String>,
    State(state): State<AppState>,
    Query(query): Query<LearningModeQuery>,
    Form(form): Form<ProfileForm>,
) -> Response {
    let mode = query.parsed_mode();
    let profile = form.profile.unwrap_or_default();
    match state.client.run_experiment(&experiment_id, profile).await {
        Ok(run) => {
            state.invalidate_shell_cache().await;
            render_pipeline_view(
                &state,
                &experiment_id,
                PipelineRenderInput {
                    mode,
                    run: Some(run),
                    benchmark: None,
                    reproducibility: None,
                    action_error: None,
                    success_status: StatusCode::OK,
                },
            )
            .await
        }
        Err(err) => {
            state.invalidate_shell_cache().await;
            render_pipeline_view(
                &state,
                &experiment_id,
                PipelineRenderInput {
                    mode,
                    run: None,
                    benchmark: None,
                    reproducibility: None,
                    action_error: Some(format!("Run action failed: {err}")),
                    success_status: StatusCode::BAD_GATEWAY,
                },
            )
            .await
        }
    }
}

async fn benchmark_pipeline(
    Path(experiment_id): Path<String>,
    State(state): State<AppState>,
    Query(query): Query<LearningModeQuery>,
    Form(form): Form<BenchmarkForm>,
) -> Response {
    let mode = query.parsed_mode();
    let profile = form.profile.unwrap_or_default();
    match state
        .client
        .benchmark_experiment(&experiment_id, profile, form.iterations)
        .await
    {
        Ok(benchmark) => {
            state.invalidate_shell_cache().await;
            render_pipeline_view(
                &state,
                &experiment_id,
                PipelineRenderInput {
                    mode,
                    run: None,
                    benchmark: Some(benchmark),
                    reproducibility: None,
                    action_error: None,
                    success_status: StatusCode::OK,
                },
            )
            .await
        }
        Err(err) => {
            state.invalidate_shell_cache().await;
            render_pipeline_view(
                &state,
                &experiment_id,
                PipelineRenderInput {
                    mode,
                    run: None,
                    benchmark: None,
                    reproducibility: None,
                    action_error: Some(format!("Benchmark action failed: {err}")),
                    success_status: StatusCode::BAD_GATEWAY,
                },
            )
            .await
        }
    }
}

async fn repro_pipeline(
    Path(experiment_id): Path<String>,
    State(state): State<AppState>,
    Query(query): Query<LearningModeQuery>,
    Form(form): Form<ReproForm>,
) -> Response {
    let mode = query.parsed_mode();
    let profile = form.profile.unwrap_or_default();
    match state
        .client
        .reproducibility_experiment(&experiment_id, profile, form.iterations, form.tolerance)
        .await
    {
        Ok(reproducibility) => {
            state.invalidate_shell_cache().await;
            render_pipeline_view(
                &state,
                &experiment_id,
                PipelineRenderInput {
                    mode,
                    run: None,
                    benchmark: None,
                    reproducibility: Some(reproducibility),
                    action_error: None,
                    success_status: StatusCode::OK,
                },
            )
            .await
        }
        Err(err) => {
            state.invalidate_shell_cache().await;
            render_pipeline_view(
                &state,
                &experiment_id,
                PipelineRenderInput {
                    mode,
                    run: None,
                    benchmark: None,
                    reproducibility: None,
                    action_error: Some(format!("Reproducibility action failed: {err}")),
                    success_status: StatusCode::BAD_GATEWAY,
                },
            )
            .await
        }
    }
}

async fn render_index_view(
    state: &AppState,
    mode: LearningMode,
    suite: Option<SuiteResponse>,
    action_error: Option<String>,
    success_status: StatusCode,
) -> Response {
    match state.load_shell_data_cached().await {
        Ok(data) => {
            let has_action_error = action_error.is_some();
            let pipeline_cards = data
                .pipelines
                .into_iter()
                .map(|pipeline| PipelineLessonCard {
                    lesson: lesson_for_pipeline(&pipeline.id),
                    pipeline,
                })
                .collect();
            let (mode_story, mode_explorer, mode_research) = mode_flags(mode);
            let template = IndexTemplate {
                title: "Gororoba Studio | Experiment Lab".to_string(),
                backend_url: state.client.base_url().to_string(),
                version: data.version,
                pipeline_cards,
                history_preview: data.history.into_iter().take(12).collect(),
                suite,
                action_error,
                mode_label: mode.label(),
                mode_query_value: mode.as_query_value(),
                mode_story,
                mode_explorer,
                mode_research,
            };
            let status = if has_action_error {
                StatusCode::BAD_GATEWAY
            } else {
                success_status
            };
            render_html(status, &template)
        }
        Err(error) => render_error(
            StatusCode::BAD_GATEWAY,
            "Backend Unavailable",
            state.client.base_url(),
            "studio.v1",
            format!("Cannot load dashboard data: {error}"),
        ),
    }
}

async fn render_pipeline_view(
    state: &AppState,
    experiment_id: &str,
    input: PipelineRenderInput,
) -> Response {
    let data = match state.load_shell_data_cached().await {
        Ok(data) => data,
        Err(error) => {
            return render_error(
                StatusCode::BAD_GATEWAY,
                "Backend Unavailable",
                state.client.base_url(),
                "studio.v1",
                format!("Cannot load pipeline view: {error}"),
            );
        }
    };

    let Some(pipeline) = data
        .pipelines
        .iter()
        .find(|pipeline| pipeline.id == experiment_id)
        .cloned()
    else {
        return render_error(
            StatusCode::NOT_FOUND,
            "Unknown Pipeline",
            state.client.base_url(),
            &data.version.api_version,
            format!("No pipeline with id '{experiment_id}' is present in the backend catalog."),
        );
    };

    let has_action_error = input.action_error.is_some();
    let (mode_story, mode_explorer, mode_research) = mode_flags(input.mode);
    let template = PipelineTemplate {
        title: format!("Pipeline {} | Gororoba Studio", pipeline.id),
        backend_url: state.client.base_url().to_string(),
        version: data.version,
        lesson: lesson_for_pipeline(&pipeline.id),
        pipeline,
        history_preview: data.history.into_iter().take(20).collect(),
        run: input.run,
        benchmark: input.benchmark,
        reproducibility: input.reproducibility,
        action_error: input.action_error,
        mode_label: input.mode.label(),
        mode_query_value: input.mode.as_query_value(),
        mode_story,
        mode_explorer,
        mode_research,
    };
    let status = if has_action_error {
        StatusCode::BAD_GATEWAY
    } else {
        input.success_status
    };
    render_html(status, &template)
}

async fn fetch_shell_data(client: &StudioClient) -> Result<ShellData, ClientError> {
    let (version, pipelines, history) =
        tokio::join!(client.version(), client.pipelines(), client.history());
    Ok(ShellData {
        version: version?,
        pipelines: pipelines?,
        history: history?,
    })
}

fn render_html<T: askama::Template>(status: StatusCode, template: &T) -> Response {
    match render_template(template) {
        Ok(body) => (status, Html(body)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("template render failure: {err}"),
        )
            .into_response(),
    }
}

fn render_error(
    status: StatusCode,
    title: &str,
    backend_url: &str,
    api_version: &str,
    message: String,
) -> Response {
    let template = ErrorTemplate {
        title: title.to_string(),
        backend_url: backend_url.to_string(),
        api_version: api_version.to_string(),
        message,
    };
    render_html(status, &template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use axum::{Json, extract::Path};
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    #[tokio::test]
    async fn healthz_is_ok() {
        let app = build_router(AppState::new(StudioClient::new("http://127.0.0.1:9")));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("\"status\":\"ok\""));
    }

    #[tokio::test]
    async fn index_renders_with_mock_backend() {
        let backend_url = spawn_mock_backend().await;
        let app = build_router(AppState::new(StudioClient::new(backend_url)));
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("Gororoba Experiment Studio"));
        assert!(body.contains("thesis-1"));
    }

    #[tokio::test]
    async fn run_action_renders_latest_run() {
        let backend_url = spawn_mock_backend().await;
        let app = build_router(AppState::new(StudioClient::new(backend_url)));
        let request = Request::builder()
            .method("POST")
            .uri("/pipeline/thesis-1/run")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("profile=quick"))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("Latest Run Result"));
        assert!(body.contains("Signal Coupling"));
    }

    #[tokio::test]
    async fn suite_action_renders_summary() {
        let backend_url = spawn_mock_backend().await;
        let app = build_router(AppState::new(StudioClient::new(backend_url)));
        let request = Request::builder()
            .method("POST")
            .uri("/run-suite")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("profile=full"))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("Suite Summary"));
        assert!(body.contains("Pass/Fail"));
    }

    #[tokio::test]
    async fn benchmark_action_renders_summary() {
        let backend_url = spawn_mock_backend().await;
        let app = build_router(AppState::new(StudioClient::new(backend_url)));
        let request = Request::builder()
            .method("POST")
            .uri("/pipeline/thesis-1/benchmark")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("profile=quick&iterations=5"))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("Benchmark Summary"));
        assert!(body.contains("Mean Duration"));
    }

    #[tokio::test]
    async fn repro_action_renders_summary() {
        let backend_url = spawn_mock_backend().await;
        let app = build_router(AppState::new(StudioClient::new(backend_url)));
        let request = Request::builder()
            .method("POST")
            .uri("/pipeline/thesis-1/reproducibility")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("profile=quick&iterations=4&tolerance=0.002"))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("Reproducibility Summary"));
        assert!(body.contains("Max Metric Delta"));
    }

    #[tokio::test]
    async fn suite_action_error_renders_bad_gateway() {
        let backend_url = spawn_suite_error_backend().await;
        let app = build_router(AppState::new(StudioClient::new(backend_url)));
        let request = Request::builder()
            .method("POST")
            .uri("/run-suite")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("profile=quick"))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = response_text(response).await;
        assert!(body.contains("Action Error"));
        assert!(body.contains("Suite action failed"));
    }

    #[tokio::test]
    async fn cache_reuses_shell_data_and_invalidates_on_mutation() {
        let (backend_url, shell_hits) = spawn_counting_mock_backend().await;
        let app = build_router(AppState::with_cache_ttl(
            StudioClient::new(backend_url),
            Duration::from_secs(120),
        ));

        let first = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(shell_hits.load(Ordering::Relaxed), 3);

        let second = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(shell_hits.load(Ordering::Relaxed), 3);

        let run_request = Request::builder()
            .method("POST")
            .uri("/pipeline/thesis-1/run")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("profile=quick"))
            .unwrap();
        let run_response = app.clone().oneshot(run_request).await.unwrap();
        assert_eq!(run_response.status(), StatusCode::OK);
        assert_eq!(shell_hits.load(Ordering::Relaxed), 6);
    }

    #[tokio::test]
    async fn cache_ttl_expiry_forces_refetch() {
        let (backend_url, shell_hits) = spawn_counting_mock_backend().await;
        let app = build_router(AppState::with_cache_ttl(
            StudioClient::new(backend_url),
            Duration::from_millis(0),
        ));

        let first = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(shell_hits.load(Ordering::Relaxed), 3);

        let second = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(shell_hits.load(Ordering::Relaxed), 6);
    }

    #[tokio::test]
    async fn backend_unavailable_renders_bad_gateway_dashboard() {
        let app = build_router(AppState::new(StudioClient::new("http://127.0.0.1:9")));
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = response_text(response).await;
        assert!(body.contains("Backend Unavailable"));
    }

    #[tokio::test]
    async fn unknown_pipeline_returns_not_found() {
        let backend_url = spawn_mock_backend().await;
        let app = build_router(AppState::new(StudioClient::new(backend_url)));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/pipeline/not-real")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response_text(response).await;
        assert!(body.contains("Unknown Pipeline"));
    }

    async fn response_text(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn spawn_mock_backend() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let router = axum::Router::new()
            .route("/api/version", get(mock_version))
            .route("/api/pipelines", get(mock_pipelines))
            .route("/api/history", get(mock_history))
            .route("/api/run/{experiment_id}", post(mock_run))
            .route("/api/run-suite", post(mock_run_suite))
            .route("/api/benchmark/{experiment_id}", post(mock_benchmark))
            .route("/api/reproducibility/{experiment_id}", post(mock_repro));

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        format!("http://{addr}")
    }

    #[derive(Clone)]
    struct CountingState {
        shell_hits: Arc<AtomicUsize>,
    }

    async fn spawn_counting_mock_backend() -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let shell_hits = Arc::new(AtomicUsize::new(0));
        let state = CountingState {
            shell_hits: shell_hits.clone(),
        };

        let router = axum::Router::new()
            .route("/api/version", get(mock_version_counted))
            .route("/api/pipelines", get(mock_pipelines_counted))
            .route("/api/history", get(mock_history_counted))
            .route("/api/run/{experiment_id}", post(mock_run))
            .route("/api/run-suite", post(mock_run_suite))
            .route("/api/benchmark/{experiment_id}", post(mock_benchmark))
            .route("/api/reproducibility/{experiment_id}", post(mock_repro))
            .with_state(state);

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        (format!("http://{addr}"), shell_hits)
    }

    async fn spawn_suite_error_backend() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let router = axum::Router::new()
            .route("/api/version", get(mock_version))
            .route("/api/pipelines", get(mock_pipelines))
            .route("/api/history", get(mock_history))
            .route("/api/run/{experiment_id}", post(mock_run))
            .route("/api/run-suite", post(mock_run_suite_error))
            .route("/api/benchmark/{experiment_id}", post(mock_benchmark))
            .route("/api/reproducibility/{experiment_id}", post(mock_repro));

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        format!("http://{addr}")
    }

    fn increment_shell_hits(shell_hits: &Arc<AtomicUsize>) {
        shell_hits.fetch_add(1, Ordering::Relaxed);
    }

    async fn mock_version_counted(State(state): State<CountingState>) -> Json<serde_json::Value> {
        increment_shell_hits(&state.shell_hits);
        mock_version().await
    }

    async fn mock_pipelines_counted(State(state): State<CountingState>) -> Json<serde_json::Value> {
        increment_shell_hits(&state.shell_hits);
        mock_pipelines().await
    }

    async fn mock_history_counted(State(state): State<CountingState>) -> Json<serde_json::Value> {
        increment_shell_hits(&state.shell_hits);
        mock_history().await
    }

    async fn mock_version() -> Json<serde_json::Value> {
        Json(json!({
            "api_version": "studio.v1",
            "service": "gororoba-studio",
            "package_version": "0.1.0",
            "catalog_source": "registry",
            "pipeline_count": 2,
            "catalog_warnings": [],
            "registry_path": "registry/experiments.toml"
        }))
    }

    async fn mock_pipelines() -> Json<serde_json::Value> {
        Json(json!([
            {
                "id": "thesis-1",
                "title": "Signal Coupling",
                "hypothesis": "Correlation is stable under constrained viscosity.",
                "primary_metric": "Spearman correlation",
                "quick_profile": "8^3",
                "full_profile": "16^3",
                "experiment_id": "EXP-T1",
                "lineage_id": "LIN-T1",
                "registry_binary": "gororoba_thesis1",
                "artifact_paths": ["reports/t1.json"]
            },
            {
                "id": "thesis-2",
                "title": "Thickening Sweep",
                "hypothesis": "Viscosity ratio increases with strain rate.",
                "primary_metric": "Viscosity ratio",
                "quick_profile": "alpha=0.4",
                "full_profile": "alpha=0.5",
                "experiment_id": "EXP-T2",
                "lineage_id": "LIN-T2",
                "registry_binary": "gororoba_thesis2",
                "artifact_paths": ["reports/t2.json"]
            }
        ]))
    }

    async fn mock_history() -> Json<serde_json::Value> {
        Json(json!([
            {
                "api_version": "studio.v1",
                "run_id": 77,
                "unix_seconds": 1700000000,
                "experiment_id": "thesis-1",
                "source_experiment_id": "EXP-T1",
                "source_lineage_id": "LIN-T1",
                "artifact_links": ["reports/t1.json"],
                "profile": "quick",
                "duration_ms": 23,
                "thesis_id": 1,
                "label": "Signal Coupling",
                "metric_value": 0.91,
                "threshold": 0.85,
                "passes_gate": true,
                "config_snapshot": {"profile": "quick"},
                "messages": ["steady"]
            }
        ]))
    }

    async fn mock_run(Path(experiment_id): Path<String>) -> Json<serde_json::Value> {
        Json(json!({
            "api_version": "studio.v1",
            "run_id": 88,
            "unix_seconds": 1700000100,
            "experiment_id": experiment_id,
            "source_experiment_id": "EXP-T1",
            "source_lineage_id": "LIN-T1",
            "artifact_links": ["reports/t1.json"],
            "profile": "quick",
            "duration_ms": 19,
            "thesis_id": 1,
            "label": "Signal Coupling",
            "metric_value": 0.93,
            "threshold": 0.85,
            "passes_gate": true,
            "config_snapshot": {"profile": "quick"},
            "messages": ["gate_pass"]
        }))
    }

    async fn mock_run_suite() -> Json<serde_json::Value> {
        Json(json!({
            "api_version": "studio.v1",
            "profile": "full",
            "total_duration_ms": 99,
            "pass_count": 3,
            "fail_count": 1,
            "success_rate": 0.75,
            "results": [],
            "failures": []
        }))
    }

    async fn mock_run_suite_error() -> impl IntoResponse {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "api_version": "studio.v1",
                "error_code": "upstream_unavailable",
                "message": "mock suite failure",
                "known_ids": ["thesis-1", "thesis-2"],
                "details": {"endpoint": "/api/run-suite"}
            })),
        )
    }

    async fn mock_benchmark(Path(experiment_id): Path<String>) -> Json<serde_json::Value> {
        Json(json!({
            "api_version": "studio.v1",
            "experiment_id": experiment_id,
            "profile": "quick",
            "iterations_requested": 4,
            "iterations_completed": 4,
            "pass_count": 4,
            "fail_count": 0,
            "mean_duration_ms": 21.0,
            "median_duration_ms": 20.0,
            "min_duration_ms": 18,
            "max_duration_ms": 26,
            "mean_metric_value": 0.92,
            "metric_stddev": 0.01,
            "runs": [],
            "failures": []
        }))
    }

    async fn mock_repro(Path(experiment_id): Path<String>) -> Json<serde_json::Value> {
        Json(json!({
            "api_version": "studio.v1",
            "experiment_id": experiment_id,
            "profile": "quick",
            "iterations_requested": 5,
            "iterations_completed": 5,
            "tolerance": 0.001,
            "baseline_metric_value": 0.91,
            "max_metric_delta": 0.0004,
            "gate_consistent": true,
            "stable": true,
            "runs": [],
            "failures": []
        }))
    }
}
