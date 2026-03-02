#![forbid(unsafe_code)]
#![deny(warnings)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LearningMode {
    Story,
    #[default]
    Explorer,
    Research,
}

impl LearningMode {
    pub fn from_query_value(value: &str) -> Option<Self> {
        let trimmed = value.trim();
        if trimmed.eq_ignore_ascii_case("story") {
            Some(Self::Story)
        } else if trimmed.eq_ignore_ascii_case("explorer") {
            Some(Self::Explorer)
        } else if trimmed.eq_ignore_ascii_case("research") {
            Some(Self::Research)
        } else {
            None
        }
    }

    pub fn from_optional_query(value: Option<&str>) -> Self {
        value
            .and_then(Self::from_query_value)
            .unwrap_or(Self::Explorer)
    }

    pub const fn as_query_value(self) -> &'static str {
        match self {
            Self::Story => "story",
            Self::Explorer => "explorer",
            Self::Research => "research",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Story => "Story",
            Self::Explorer => "Explorer",
            Self::Research => "Research",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LessonLink {
    pub label: String,
    pub uri: String,
    pub link_type: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineLesson {
    pub pipeline_id: String,
    pub summary: String,
    pub story_summary: String,
    pub analogy: String,
    pub story_activity: String,
    pub equation: String,
    pub equation_walkthrough: Vec<String>,
    pub derivation_steps: Vec<String>,
    pub assumptions: Vec<String>,
    pub research_questions: Vec<String>,
    pub proof_sketch: String,
    pub visual_guidance: String,
    pub artifact_guidance: String,
    pub artifact_links: Vec<LessonLink>,
    pub reference_links: Vec<LessonLink>,
}

pub fn lesson_for_pipeline(pipeline_id: &str) -> PipelineLesson {
    match pipeline_id {
        "thesis-1" => thesis_1_lesson(),
        "thesis-2" => thesis_2_lesson(),
        "thesis-3" => thesis_3_lesson(),
        "thesis-4" => thesis_4_lesson(),
        other => fallback_lesson(other),
    }
}

fn artifact(label: &str, uri: &str) -> LessonLink {
    LessonLink {
        label: label.to_string(),
        uri: uri.to_string(),
        link_type: "artifact".to_string(),
    }
}

fn reference(label: &str, uri: &str) -> LessonLink {
    LessonLink {
        label: label.to_string(),
        uri: uri.to_string(),
        link_type: "reference".to_string(),
    }
}

pub fn thesis_1_lesson() -> PipelineLesson {
    PipelineLesson {
        pipeline_id: "thesis-1".to_string(),
        summary: "Signal coupling tracks whether rank relationships remain stable as viscosity constraints tighten.".to_string(),
        story_summary: "Imagine two dancers keeping the same rhythm even when the floor gets sticky; this pipeline checks if the rhythm still lines up.".to_string(),
        analogy: "Treat each run as a duet where both dancers should keep rank order even as friction changes.".to_string(),
        story_activity: "Sort 8 cards by height with a classmate, then repeat after swapping two cards; count how many positions changed.".to_string(),
        equation: "rho = 1 - (6 * sum(d_i^2)) / (n * (n^2 - 1))".to_string(),
        equation_walkthrough: vec![
            "rho is Spearman rank correlation: 1 means identical order, 0 means weak rank relation, -1 means opposite order.".to_string(),
            "d_i is the rank gap for pair i, so large gaps penalize rho quadratically.".to_string(),
            "The denominator n * (n^2 - 1) normalizes scale so different sample sizes remain comparable.".to_string(),
        ],
        derivation_steps: vec![
            "Rank both signals over the same observation window.".to_string(),
            "Compute rank gaps d_i between paired observations.".to_string(),
            "Square and sum d_i values, then normalize by n * (n^2 - 1).".to_string(),
            "Subtract the normalized term from 1 to obtain Spearman rho.".to_string(),
        ],
        assumptions: vec![
            "Samples are paired and represent the same timeline.".to_string(),
            "Rank ties are either absent or consistently tie-corrected.".to_string(),
            "Noise does not dominate ordering across the full window.".to_string(),
        ],
        research_questions: vec![
            "How sensitive is rho to tie-correction strategy under identical raw samples?".to_string(),
            "Does rho remain stable when the observation window is downsampled by 2x and 4x?".to_string(),
        ],
        proof_sketch: "If monotonic ordering is preserved under constrained viscosity, the rank displacement sum remains small, so rho stays near 1 and passes the gate.".to_string(),
        visual_guidance: "Overlay ranked trajectories and shade segments where rank inversions occur.".to_string(),
        artifact_guidance: "Compare run reports and keep the rank-delta table beside the final gate decision.".to_string(),
        artifact_links: vec![
            artifact(
                "Pipeline registry entry",
                "open_gororoba/registry/pipelines.toml",
            ),
            artifact(
                "Claims-evidence matrix",
                "open_gororoba/CLAIMS_EVIDENCE_MATRIX.md",
            ),
        ],
        reference_links: vec![
            reference(
                "Spearman rank correlation (NIST handbook)",
                "https://www.itl.nist.gov/div898/software/dataplot/refman2/auxillar/spearman.htm",
            ),
            reference(
                "Rank correlation overview",
                "https://en.wikipedia.org/wiki/Rank_correlation",
            ),
        ],
    }
}

pub fn thesis_2_lesson() -> PipelineLesson {
    PipelineLesson {
        pipeline_id: "thesis-2".to_string(),
        summary: "Thickening sweep estimates how response viscosity scales with strain-rate changes across profiles.".to_string(),
        story_summary: "Like stirring syrup faster and feeling it resist more, this pipeline measures how much harder the fluid pushes back.".to_string(),
        analogy: "Think of a gearbox that stiffens as you spin the handle faster.".to_string(),
        story_activity: "Mix cornstarch and water in a cup, then tap slowly versus quickly and compare the resistance you feel.".to_string(),
        equation: "eta_ratio = eta(strain_high) / eta(strain_low)".to_string(),
        equation_walkthrough: vec![
            "eta is effective viscosity estimated from measured stress and strain rate.".to_string(),
            "A ratio greater than 1 indicates shear thickening behavior.".to_string(),
            "A ratio near 1 indicates near-Newtonian behavior for the tested band.".to_string(),
        ],
        derivation_steps: vec![
            "Estimate viscosity at a lower reference strain rate.".to_string(),
            "Estimate viscosity at a higher strain rate under the same setup.".to_string(),
            "Form the ratio eta_high over eta_low to quantify thickening.".to_string(),
            "Aggregate ratios across iterations and compare against threshold.".to_string(),
        ],
        assumptions: vec![
            "Temperature and composition remain fixed per sweep.".to_string(),
            "The estimator for eta is calibrated across both strain rates.".to_string(),
            "Boundary effects are negligible for selected sample windows.".to_string(),
        ],
        research_questions: vec![
            "Is eta_ratio robust under bootstrap resampling of stress-strain samples?".to_string(),
            "At what strain-rate interval does eta_ratio transition from neutral to clearly thickening?".to_string(),
        ],
        proof_sketch: "For shear-thickening behavior, eta increases with strain rate, forcing eta_ratio above 1; stable runs keep this ratio consistently above the configured gate.".to_string(),
        visual_guidance: "Plot eta against strain rate and annotate slope changes near the transition region.".to_string(),
        artifact_guidance: "Store per-iteration eta pairs and include a ratio histogram in the benchmark artifact.".to_string(),
        artifact_links: vec![
            artifact(
                "Experiment manifests",
                "open_gororoba/registry/experiments.toml",
            ),
            artifact(
                "Benchmark outputs lane",
                "open_gororoba/artifacts/",
            ),
        ],
        reference_links: vec![
            reference(
                "Shear thickening review (arXiv)",
                "https://arxiv.org/abs/1307.0269",
            ),
            reference(
                "Viscosity and rheology handbook reference (NIST)",
                "https://www.itl.nist.gov/div898/handbook/prc/section2/prc252.htm",
            ),
        ],
    }
}

pub fn thesis_3_lesson() -> PipelineLesson {
    PipelineLesson {
        pipeline_id: "thesis-3".to_string(),
        summary: "Boundary persistence tracks whether flow structures survive perturbations near edge conditions.".to_string(),
        story_summary: "Picture chalk lines near the edge of a table; this checks if the lines stay visible after repeated bumps.".to_string(),
        analogy: "Boundary features are footprints in wet sand that should remain readable after light waves.".to_string(),
        story_activity: "Draw six chalk marks near a desk edge and gently brush the edge five times, then count how many marks remain visible.".to_string(),
        equation: "persistence = retained_features / initial_features".to_string(),
        equation_walkthrough: vec![
            "initial_features counts structures at baseline before perturbation.".to_string(),
            "retained_features counts structures that remain detectable after perturbation cycles.".to_string(),
            "The ratio gives a normalized survivability score in [0, 1].".to_string(),
        ],
        derivation_steps: vec![
            "Count detectable features at baseline.".to_string(),
            "Apply controlled perturbation cycles.".to_string(),
            "Recount features that remain above detection threshold.".to_string(),
            "Compute retained over initial as persistence score.".to_string(),
        ],
        assumptions: vec![
            "Feature detector threshold is stable across runs.".to_string(),
            "Perturbation intensity is reproducible per profile.".to_string(),
            "Feature loss is primarily due to boundary dynamics, not sensor drift.".to_string(),
        ],
        research_questions: vec![
            "How does persistence change under threshold sweeps for the feature detector?".to_string(),
            "Does persistence remain invariant under equivalent perturbation energy applied in fewer versus more cycles?".to_string(),
        ],
        proof_sketch: "If boundary dynamics are resilient, perturbations remove only a small share of features, so retained over initial remains high and the gate stays consistent.".to_string(),
        visual_guidance: "Use before-and-after edge maps with highlighted dropped features.".to_string(),
        artifact_guidance: "Archive detector masks and a compact table of retained feature counts per cycle.".to_string(),
        artifact_links: vec![
            artifact(
                "Visualization lane inputs",
                "open_gororoba/docs/visualization/",
            ),
            artifact(
                "Boundary experiment notes",
                "open_gororoba/docs/research/",
            ),
        ],
        reference_links: vec![
            reference(
                "Boundary layer fundamentals (NASA)",
                "https://www.grc.nasa.gov/www/k-12/airplane/boundlay.html",
            ),
            reference(
                "Feature detection robustness (OpenCV docs)",
                "https://docs.opencv.org/4.x/d5/d51/group__features2d__main.html",
            ),
        ],
    }
}

pub fn thesis_4_lesson() -> PipelineLesson {
    PipelineLesson {
        pipeline_id: "thesis-4".to_string(),
        summary: "Convergence envelope quantifies how quickly iterative estimates settle within a target tolerance.".to_string(),
        story_summary: "Imagine tuning a radio until the static fades; this pipeline asks how many turns it takes before the sound is clean enough.".to_string(),
        analogy: "Each iteration is another lens adjustment that should shrink blur toward a sharp focus.".to_string(),
        story_activity: "Estimate the same quantity five times in a row and track how quickly your answers bunch inside a small acceptable band.".to_string(),
        equation: "error_k = |estimate_k - target|, converge when error_k <= epsilon".to_string(),
        equation_walkthrough: vec![
            "error_k is the absolute distance between the current estimate and the target.".to_string(),
            "epsilon sets the acceptance band based on domain tolerance.".to_string(),
            "Convergence requires both entering and staying inside the epsilon band.".to_string(),
        ],
        derivation_steps: vec![
            "Compute absolute error for each iteration against target.".to_string(),
            "Track the first iteration where error falls below epsilon.".to_string(),
            "Measure whether later iterations stay within the envelope.".to_string(),
            "Report convergence speed and stability over the run set.".to_string(),
        ],
        assumptions: vec![
            "Target value is trustworthy for the current scenario.".to_string(),
            "Iteration update rule is deterministic for fixed seeds.".to_string(),
            "Tolerance epsilon reflects practical acceptance criteria.".to_string(),
        ],
        research_questions: vec![
            "Is the observed convergence rate linear, sublinear, or geometric across profiles?".to_string(),
            "How does convergence break when epsilon is tightened by one order of magnitude?".to_string(),
        ],
        proof_sketch: "When the update map is contractive near the target, error_k decreases geometrically; once below epsilon, subsequent errors stay bounded and demonstrate convergence.".to_string(),
        visual_guidance: "Render error-versus-iteration curves with the epsilon band shaded.".to_string(),
        artifact_guidance: "Persist per-iteration error traces and include the first-hit index for epsilon.".to_string(),
        artifact_links: vec![
            artifact(
                "Convergence benchmark outputs",
                "open_gororoba/artifacts/benchmarks/",
            ),
            artifact(
                "Pipeline registry and thresholds",
                "open_gororoba/registry/pipelines.toml",
            ),
        ],
        reference_links: vec![
            reference(
                "Contraction mapping theorem notes",
                "https://mathworld.wolfram.com/BanachFixedPointTheorem.html",
            ),
            reference(
                "Numerical convergence basics (Stanford EE364A notes)",
                "https://web.stanford.edu/class/ee364a/",
            ),
        ],
    }
}

pub fn fallback_lesson(pipeline_id: &str) -> PipelineLesson {
    PipelineLesson {
        pipeline_id: pipeline_id.to_string(),
        summary: "This pipeline has not been mapped to a thesis-specific lesson yet; use the general validation checklist.".to_string(),
        story_summary: "Start with the plain-language checklist: what changed, what stayed stable, and what evidence was saved.".to_string(),
        analogy: "Treat this like a lab notebook entry that needs clear observations before theory.".to_string(),
        story_activity: "Describe the run in three sentences: what you changed, what you observed, and what evidence you saved.".to_string(),
        equation: "score = observed_signal - baseline_signal".to_string(),
        equation_walkthrough: vec![
            "Compute a baseline reference using the same instrumentation.".to_string(),
            "Measure observed signal after applying the pipeline action.".to_string(),
            "Subtract baseline from observed to get a signed effect estimate.".to_string(),
        ],
        derivation_steps: vec![
            "Capture baseline measurements.".to_string(),
            "Run the pipeline under selected profile.".to_string(),
            "Compute observed-minus-baseline deltas.".to_string(),
            "Compare deltas to the declared acceptance threshold.".to_string(),
        ],
        assumptions: vec![
            "Baseline and observed runs are comparable.".to_string(),
            "Artifacts are versioned and traceable to the run id.".to_string(),
            "Threshold values are documented for the selected profile.".to_string(),
        ],
        research_questions: vec![
            "Which confounders could explain a positive delta besides the intended intervention?".to_string(),
            "How does the delta distribution evolve over repeated runs and random seeds?".to_string(),
        ],
        proof_sketch: "A stable positive delta with reproducible artifacts provides preliminary support, while inconsistent deltas indicate the need for deeper diagnostics.".to_string(),
        visual_guidance: "Use side-by-side baseline and observed panels with delta annotations.".to_string(),
        artifact_guidance: "Bundle run metadata, delta summaries, and reproducibility checks in one folder.".to_string(),
        artifact_links: vec![
            artifact("General source registry", "open_gororoba/registry/"),
            artifact("Experiment evidence matrix", "open_gororoba/CLAIMS_EVIDENCE_MATRIX.md"),
        ],
        reference_links: vec![
            reference(
                "Effect size overview",
                "https://en.wikipedia.org/wiki/Effect_size",
            ),
            reference(
                "Reproducibility principles (NASEM overview)",
                "https://nap.nationalacademies.org/read/25303/chapter/1",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_learning_modes_from_query_values() {
        assert_eq!(
            LearningMode::from_query_value("story"),
            Some(LearningMode::Story)
        );
        assert_eq!(
            LearningMode::from_query_value("explorer"),
            Some(LearningMode::Explorer)
        );
        assert_eq!(
            LearningMode::from_query_value("research"),
            Some(LearningMode::Research)
        );
        assert_eq!(
            LearningMode::from_query_value(" Story "),
            Some(LearningMode::Story)
        );
    }

    #[test]
    fn falls_back_to_explorer_for_invalid_or_missing_mode() {
        assert_eq!(
            LearningMode::from_optional_query(Some("unknown")),
            LearningMode::Explorer
        );
        assert_eq!(
            LearningMode::from_optional_query(None),
            LearningMode::Explorer
        );
    }

    #[test]
    fn thesis_and_fallback_lessons_are_complete() {
        let lessons = [
            thesis_1_lesson(),
            thesis_2_lesson(),
            thesis_3_lesson(),
            thesis_4_lesson(),
            fallback_lesson("thesis-unknown"),
        ];

        for lesson in lessons {
            assert_lesson_complete(&lesson);
        }
    }

    fn assert_lesson_complete(lesson: &PipelineLesson) {
        assert!(!lesson.pipeline_id.trim().is_empty());
        assert!(!lesson.summary.trim().is_empty());
        assert!(!lesson.story_summary.trim().is_empty());
        assert!(!lesson.analogy.trim().is_empty());
        assert!(!lesson.story_activity.trim().is_empty());
        assert!(!lesson.equation.trim().is_empty());
        assert!(!lesson.proof_sketch.trim().is_empty());
        assert!(!lesson.visual_guidance.trim().is_empty());
        assert!(!lesson.artifact_guidance.trim().is_empty());
        assert!(!lesson.equation_walkthrough.is_empty());
        assert!(!lesson.derivation_steps.is_empty());
        assert!(!lesson.assumptions.is_empty());
        assert!(!lesson.research_questions.is_empty());
        assert!(!lesson.artifact_links.is_empty());
        assert!(!lesson.reference_links.is_empty());

        for step in &lesson.equation_walkthrough {
            assert!(!step.trim().is_empty());
        }

        for step in &lesson.derivation_steps {
            assert!(!step.trim().is_empty());
        }
        for assumption in &lesson.assumptions {
            assert!(!assumption.trim().is_empty());
        }

        for question in &lesson.research_questions {
            assert!(!question.trim().is_empty());
        }

        for link in &lesson.artifact_links {
            assert!(!link.label.trim().is_empty());
            assert!(!link.uri.trim().is_empty());
            assert_eq!(link.link_type, "artifact");
        }

        for link in &lesson.reference_links {
            assert!(!link.label.trim().is_empty());
            assert!(!link.uri.trim().is_empty());
            assert_eq!(link.link_type, "reference");
        }
    }
}
