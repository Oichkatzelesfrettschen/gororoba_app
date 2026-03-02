use crate::models::{
    BenchmarkResponse, PipelineDescriptor, ReproducibilityResponse, RunResponse, SuiteResponse,
    VersionResponse,
};
use askama::Template;
use gororoba_shared_core::PipelineLesson;

#[derive(Debug, Clone)]
pub struct PipelineLessonCard {
    pub pipeline: PipelineDescriptor,
    pub lesson: PipelineLesson,
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub title: String,
    pub backend_url: String,
    pub version: VersionResponse,
    pub pipeline_cards: Vec<PipelineLessonCard>,
    pub history_preview: Vec<RunResponse>,
    pub suite: Option<SuiteResponse>,
    pub action_error: Option<String>,
    pub mode_label: &'static str,
    pub mode_query_value: &'static str,
    pub mode_story: bool,
    pub mode_explorer: bool,
    pub mode_research: bool,
}

#[derive(Debug, Template)]
#[template(path = "pipeline.html")]
pub struct PipelineTemplate {
    pub title: String,
    pub backend_url: String,
    pub version: VersionResponse,
    pub pipeline: PipelineDescriptor,
    pub history_preview: Vec<RunResponse>,
    pub run: Option<RunResponse>,
    pub benchmark: Option<BenchmarkResponse>,
    pub reproducibility: Option<ReproducibilityResponse>,
    pub action_error: Option<String>,
    pub lesson: PipelineLesson,
    pub mode_label: &'static str,
    pub mode_query_value: &'static str,
    pub mode_story: bool,
    pub mode_explorer: bool,
    pub mode_research: bool,
}

#[derive(Debug, Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub title: String,
    pub backend_url: String,
    pub api_version: String,
    pub message: String,
}

pub fn render_template<T: Template>(template: &T) -> Result<String, askama::Error> {
    template.render()
}
