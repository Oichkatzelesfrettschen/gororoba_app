#![forbid(unsafe_code)]
#![deny(warnings)]

use askama::Template;
use axum::extract::Json;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Router, response::Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Deserialize)]
struct SimulateRequest {
    steps: usize,
    dt: f64,
    spring_k: f64,
    damping: f64,
    seed: u64,
}

impl Default for SimulateRequest {
    fn default() -> Self {
        Self {
            steps: 480,
            dt: 0.01,
            spring_k: 3.2,
            damping: 0.08,
            seed: 7,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SamplePoint {
    step: usize,
    x: f64,
    energy: f64,
}

#[derive(Debug, Clone, Serialize)]
struct SimulateResponse {
    steps: usize,
    dt: f64,
    spring_k: f64,
    damping: f64,
    final_x: f64,
    final_v: f64,
    max_abs_x: f64,
    mean_energy: f64,
    energy_drift: f64,
    stable: bool,
    samples: Vec<SamplePoint>,
}

#[derive(Debug, Clone, Deserialize)]
struct BenchmarkRequest {
    iterations: usize,
    steps: usize,
    dt: f64,
    spring_k: f64,
    damping: f64,
    seed: u64,
}

impl Default for BenchmarkRequest {
    fn default() -> Self {
        Self {
            iterations: 12,
            steps: 480,
            dt: 0.01,
            spring_k: 3.2,
            damping: 0.08,
            seed: 7,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkResponse {
    iterations: usize,
    steps: usize,
    mean_duration_ms: f64,
    min_duration_ms: f64,
    max_duration_ms: f64,
    mean_final_x: f64,
    final_x_stddev: f64,
    mean_energy_drift: f64,
    stable_ratio: f64,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "physics_sandbox=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = env::var("PHYSICS_SANDBOX_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PHYSICS_SANDBOX_PORT").unwrap_or_else(|_| "8093".to_string());
    let bind_addr = format!("{host}:{port}");

    let listener = TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind physics_sandbox on {bind_addr}: {err}"));

    tracing::info!(bind_addr, "starting physics_sandbox");

    axum::serve(listener, build_router())
        .await
        .unwrap_or_else(|err| panic!("physics_sandbox server failed: {err}"));
}

fn build_router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/assets/app.css", get(app_css))
        .route("/assets/app.js", get(app_js))
        .route("/api/simulate", post(simulate_api))
        .route("/api/benchmark", post(benchmark_api))
}

async fn index() -> Response {
    let template = IndexTemplate {
        title: "Gororoba Physics Sandbox".to_string(),
    };
    render_html(StatusCode::OK, &template)
}

async fn healthz() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "service": "physics_sandbox"}))
}

async fn app_css() -> impl IntoResponse {
    const CSS: &str = include_str!("../assets/app.css");
    (
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        CSS,
    )
}

async fn app_js() -> impl IntoResponse {
    const JS: &str = include_str!("../assets/app.js");
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/javascript; charset=utf-8",
        )],
        JS,
    )
}

async fn simulate_api(Json(request): Json<SimulateRequest>) -> Json<SimulateResponse> {
    Json(run_simulation(request))
}

async fn benchmark_api(Json(request): Json<BenchmarkRequest>) -> Json<BenchmarkResponse> {
    Json(run_benchmark(request))
}

fn run_simulation(request: SimulateRequest) -> SimulateResponse {
    let steps = request.steps.clamp(20, 50_000);
    let dt = request.dt.clamp(0.0001, 0.1);
    let spring_k = request.spring_k.clamp(0.05, 20.0);
    let damping = request.damping.clamp(0.0, 2.0);

    let mut x = 1.0 + (request.seed % 17) as f64 * 0.01;
    let mut v = 0.0;
    let mut max_abs_x = x.abs();
    let mut total_energy = 0.0;
    let mut samples = Vec::new();
    let stride = (steps / 90).max(1);

    let initial_energy = 0.5 * spring_k * x * x + 0.5 * v * v;

    for step in 0..steps {
        let a = -spring_k * x - damping * v;
        v += a * dt;
        x += v * dt;

        let energy = 0.5 * spring_k * x * x + 0.5 * v * v;
        total_energy += energy;
        if x.abs() > max_abs_x {
            max_abs_x = x.abs();
        }
        if step % stride == 0 || step + 1 == steps {
            samples.push(SamplePoint { step, x, energy });
        }
    }

    let final_energy = 0.5 * spring_k * x * x + 0.5 * v * v;
    let mean_energy = total_energy / steps as f64;
    let energy_drift = final_energy - initial_energy;

    SimulateResponse {
        steps,
        dt,
        spring_k,
        damping,
        final_x: x,
        final_v: v,
        max_abs_x,
        mean_energy,
        energy_drift,
        stable: max_abs_x < 8.0 && energy_drift.abs() < 2.5,
        samples,
    }
}

fn run_benchmark(request: BenchmarkRequest) -> BenchmarkResponse {
    let iterations = request.iterations.clamp(1, 300);
    let mut durations = Vec::with_capacity(iterations);
    let mut final_x_values = Vec::with_capacity(iterations);
    let mut energy_drifts = Vec::with_capacity(iterations);
    let mut stable_count = 0usize;

    for i in 0..iterations {
        let start = Instant::now();
        let response = run_simulation(SimulateRequest {
            steps: request.steps,
            dt: request.dt,
            spring_k: request.spring_k,
            damping: request.damping,
            seed: request.seed + i as u64,
        });
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        durations.push(duration);
        final_x_values.push(response.final_x);
        energy_drifts.push(response.energy_drift);
        if response.stable {
            stable_count += 1;
        }
    }

    let mean_duration_ms = mean(&durations);
    let min_duration_ms = durations
        .iter()
        .copied()
        .fold(f64::INFINITY, |acc, value| acc.min(value));
    let max_duration_ms = durations
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));

    BenchmarkResponse {
        iterations,
        steps: request.steps,
        mean_duration_ms,
        min_duration_ms,
        max_duration_ms,
        mean_final_x: mean(&final_x_values),
        final_x_stddev: stddev(&final_x_values),
        mean_energy_drift: mean(&energy_drifts),
        stable_ratio: stable_count as f64 / iterations as f64,
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn stddev(values: &[f64]) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - m;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn render_html<T: Template>(status: StatusCode, template: &T) -> Response {
    match template.render() {
        Ok(body) => (status, Html(body)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("template render failure: {err}"),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    #[test]
    fn simulation_is_deterministic_for_same_input() {
        let request = SimulateRequest::default();
        let a = run_simulation(request.clone());
        let b = run_simulation(request);
        assert_eq!(a.steps, b.steps);
        assert!((a.final_x - b.final_x).abs() < 1e-12);
        assert!((a.energy_drift - b.energy_drift).abs() < 1e-12);
    }

    #[tokio::test]
    async fn benchmark_endpoint_returns_metrics() {
        let app = build_router();
        let request = Request::builder()
            .method("POST")
            .uri("/api/benchmark")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "iterations": 5,
                    "steps": 360,
                    "dt": 0.01,
                    "spring_k": 3.2,
                    "damping": 0.07,
                    "seed": 9
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["iterations"], 5);
        assert!(parsed["mean_duration_ms"].as_f64().unwrap() >= 0.0);
        assert!(parsed["stable_ratio"].as_f64().unwrap() >= 0.0);
    }
}
