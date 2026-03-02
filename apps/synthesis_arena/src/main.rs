#![forbid(unsafe_code)]
#![deny(warnings)]

use askama::Template;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc, time::Instant};
use tracing::info;

#[derive(Clone)]
struct AppState {
    challenges: Arc<Vec<Challenge>>,
}

#[derive(Clone, Debug)]
struct Challenge {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    gates: [u32; 4],
    weights: [u32; 4],
    target_score: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ChallengeView {
    id: String,
    name: String,
    description: String,
    gates: [u32; 4],
    weights: [u32; 4],
    target_score: u32,
}

impl From<&Challenge> for ChallengeView {
    fn from(value: &Challenge) -> Self {
        Self {
            id: value.id.to_owned(),
            name: value.name.to_owned(),
            description: value.description.to_owned(),
            gates: value.gates,
            weights: value.weights,
            target_score: value.target_score,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct EvaluationRequest {
    challenge_id: String,
    throughput: u32,
    precision: u32,
    efficiency: u32,
    resilience: u32,
    seed: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct MetricResult {
    metric: String,
    value: u32,
    gate: u32,
    passed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct EvaluationResponse {
    challenge_id: String,
    composite_score: u32,
    target_score: u32,
    passed_all_gates: bool,
    meets_target: bool,
    metrics: Vec<MetricResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct BenchmarkRequest {
    request: EvaluationRequest,
    iterations: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct DurationStats {
    mean_micros: f64,
    min_micros: u64,
    max_micros: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct ScoreStats {
    mean: f64,
    min: u32,
    max: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct BenchmarkResponse {
    iterations: u32,
    deterministic: bool,
    duration: DurationStats,
    score: ScoreStats,
    sample: EvaluationResponse,
}

#[derive(Clone, Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    app_name: &'a str,
}

fn default_challenges() -> Vec<Challenge> {
    vec![
        Challenge {
            id: "forge",
            name: "Forge Sprint",
            description: "Fast synthesis with strict stage gates.",
            gates: [220, 280, 240, 260],
            weights: [3, 4, 2, 3],
            target_score: 285,
        },
        Challenge {
            id: "citadel",
            name: "Citadel Lock",
            description: "Resilience-heavy setup tuned for stable pipelines.",
            gates: [210, 245, 300, 235],
            weights: [2, 3, 5, 2],
            target_score: 295,
        },
        Challenge {
            id: "relay",
            name: "Relay Gauntlet",
            description: "Balanced profile with difficult delivery threshold.",
            gates: [230, 255, 250, 295],
            weights: [3, 3, 3, 4],
            target_score: 300,
        },
    ]
}

fn bounded_input(value: u32) -> u32 {
    value.min(100)
}

fn find_challenge<'a>(state: &'a AppState, id: &str) -> Result<&'a Challenge, ApiError> {
    state
        .challenges
        .iter()
        .find(|challenge| challenge.id == id)
        .ok_or_else(|| ApiError::BadRequest(format!("unknown challenge_id: {id}")))
}

fn evaluate_request(request: &EvaluationRequest, challenge: &Challenge) -> EvaluationResponse {
    let throughput = bounded_input(request.throughput);
    let precision = bounded_input(request.precision);
    let efficiency = bounded_input(request.efficiency);
    let resilience = bounded_input(request.resilience);
    let seed = (request.seed % 10_000) as u32;

    let ingest = 40 + ((throughput * 5) / 2) + (efficiency * 2) + (seed % 13);
    let synthesis = (35 + (precision * 3) + throughput + ((seed / 7) % 17))
        .saturating_sub((100 - efficiency) / 3);
    let stability = (30 + (resilience * 3) + (efficiency * 2) + ((seed / 11) % 19))
        .saturating_sub(throughput / 4);
    let delivery = 38
        + ((throughput + precision + resilience) / 2)
        + ((efficiency * 3) / 2)
        + ((seed / 13) % 23);

    let metric_names = ["ingest", "synthesis", "stability", "delivery"];
    let metric_values = [ingest, synthesis, stability, delivery];

    let mut metrics = Vec::with_capacity(4);
    for index in 0..4 {
        let value = metric_values[index];
        let gate = challenge.gates[index];
        metrics.push(MetricResult {
            metric: metric_names[index].to_owned(),
            value,
            gate,
            passed: value >= gate,
        });
    }

    let weight_sum: u32 = challenge.weights.iter().sum();
    let weighted_score: u32 = metrics
        .iter()
        .zip(challenge.weights.iter())
        .map(|(metric, weight)| metric.value * *weight)
        .sum();
    let composite_score = weighted_score / weight_sum;
    let passed_all_gates = metrics.iter().all(|metric| metric.passed);

    EvaluationResponse {
        challenge_id: challenge.id.to_owned(),
        composite_score,
        target_score: challenge.target_score,
        passed_all_gates,
        meets_target: composite_score >= challenge.target_score,
        metrics,
    }
}

fn build_router() -> Router {
    let state = AppState {
        challenges: Arc::new(default_challenges()),
    };

    Router::new()
        .route("/", get(index))
        .route("/assets/app.css", get(app_css))
        .route("/assets/app.js", get(app_js))
        .route("/healthz", get(healthz))
        .route("/api/challenges", get(list_challenges))
        .route("/api/evaluate", post(evaluate))
        .route("/api/benchmark", post(benchmark))
        .with_state(state)
}

async fn index() -> Response {
    let template = IndexTemplate {
        app_name: "Synthesis Arena",
    };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            let message = format!("template render failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
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

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn list_challenges(State(state): State<AppState>) -> Json<Vec<ChallengeView>> {
    let views = state
        .challenges
        .iter()
        .map(ChallengeView::from)
        .collect::<Vec<_>>();
    Json(views)
}

async fn evaluate(
    State(state): State<AppState>,
    Json(request): Json<EvaluationRequest>,
) -> Result<Json<EvaluationResponse>, ApiError> {
    let challenge = find_challenge(&state, &request.challenge_id)?;
    let result = evaluate_request(&request, challenge);
    Ok(Json(result))
}

async fn benchmark(
    State(state): State<AppState>,
    Json(request): Json<BenchmarkRequest>,
) -> Result<Json<BenchmarkResponse>, ApiError> {
    if request.iterations == 0 {
        return Err(ApiError::BadRequest(
            "iterations must be greater than zero".to_owned(),
        ));
    }
    if request.iterations > 50_000 {
        return Err(ApiError::BadRequest(
            "iterations must be less than or equal to 50000".to_owned(),
        ));
    }

    let challenge = find_challenge(&state, &request.request.challenge_id)?;
    let mut min_duration = u64::MAX;
    let mut max_duration = 0_u64;
    let mut sum_duration = 0_f64;

    let mut min_score = u32::MAX;
    let mut max_score = 0_u32;
    let mut sum_score = 0_f64;

    let mut sample: Option<EvaluationResponse> = None;
    let mut deterministic = true;

    for _ in 0..request.iterations {
        let started_at = Instant::now();
        let result = evaluate_request(&request.request, challenge);
        let elapsed_micros_u128 = started_at.elapsed().as_micros();
        let elapsed_micros = u64::try_from(elapsed_micros_u128).unwrap_or(u64::MAX);

        min_duration = min_duration.min(elapsed_micros);
        max_duration = max_duration.max(elapsed_micros);
        sum_duration += elapsed_micros as f64;

        min_score = min_score.min(result.composite_score);
        max_score = max_score.max(result.composite_score);
        sum_score += result.composite_score as f64;

        if let Some(first) = &sample {
            deterministic = deterministic && *first == result;
        } else {
            sample = Some(result);
        }
    }

    let sample = match sample {
        Some(value) => value,
        None => {
            return Err(ApiError::BadRequest(
                "benchmark execution produced no sample".to_owned(),
            ));
        }
    };

    let count = request.iterations as f64;
    let response = BenchmarkResponse {
        iterations: request.iterations,
        deterministic,
        duration: DurationStats {
            mean_micros: sum_duration / count,
            min_micros: min_duration,
            max_micros: max_duration,
        },
        score: ScoreStats {
            mean: sum_score / count,
            min: min_score,
            max: max_score,
        },
        sample,
    };

    Ok(Json(response))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let router = build_router();
    let host = env::var("SYNTHESIS_ARENA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("SYNTHESIS_ARENA_PORT").unwrap_or_else(|_| "8094".to_string());
    let bind_addr = format!("{host}:{port}");
    info!("synthesis_arena listening on http://{bind_addr}");

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, router)
        .await
        .expect("server execution failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[test]
    fn deterministic_evaluation_for_identical_input() {
        let state = AppState {
            challenges: Arc::new(default_challenges()),
        };
        let request = EvaluationRequest {
            challenge_id: "forge".to_owned(),
            throughput: 84,
            precision: 73,
            efficiency: 66,
            resilience: 91,
            seed: 1337,
        };

        let challenge = find_challenge(&state, &request.challenge_id)
            .expect("challenge should exist for deterministic test");
        let first = evaluate_request(&request, challenge);
        let second = evaluate_request(&request, challenge);

        assert_eq!(first, second, "evaluation should be deterministic");
        assert_eq!(first.metrics.len(), 4, "must expose four pipeline metrics");
    }

    #[tokio::test]
    async fn benchmark_api_returns_valid_shape_and_values() {
        let app = build_router();
        let payload = serde_json::json!({
            "request": {
                "challenge_id": "relay",
                "throughput": 77,
                "precision": 62,
                "efficiency": 59,
                "resilience": 88,
                "seed": 901
            },
            "iterations": 32
        });

        let request = Request::builder()
            .method("POST")
            .uri("/api/benchmark")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("failed to build benchmark request");

        let response = app
            .oneshot(request)
            .await
            .expect("router should return a response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let parsed: BenchmarkResponse =
            serde_json::from_slice(&body).expect("benchmark payload should parse");

        assert_eq!(parsed.iterations, 32);
        assert!(
            parsed.deterministic,
            "benchmark should remain deterministic"
        );
        assert!(parsed.duration.min_micros <= parsed.duration.max_micros);
        assert!(parsed.duration.mean_micros >= parsed.duration.min_micros as f64);
        assert!(parsed.duration.mean_micros <= parsed.duration.max_micros as f64);
        assert!(parsed.score.min <= parsed.score.max);
        assert!(parsed.score.mean >= parsed.score.min as f64);
        assert!(parsed.score.mean <= parsed.score.max as f64);
        assert_eq!(parsed.sample.metrics.len(), 4);
    }

    #[tokio::test]
    async fn benchmark_rejects_zero_iterations() {
        let app = build_router();
        let payload = serde_json::json!({
            "request": {
                "challenge_id": "forge",
                "throughput": 50,
                "precision": 50,
                "efficiency": 50,
                "resilience": 50,
                "seed": 9
            },
            "iterations": 0
        });

        let request = Request::builder()
            .method("POST")
            .uri("/api/benchmark")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("failed to build request");

        let response = app.oneshot(request).await.expect("router should respond");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
