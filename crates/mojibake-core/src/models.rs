use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformationStep {
    pub operation: String,
    pub encoding: String,
    pub input_type: String,
    pub output_type: String,
    pub input_preview: String,
    pub output_preview: String,
    pub input_size: usize,
    pub output_size: usize,
    pub lossy: bool,
    pub replacement_count: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreBreakdown {
    pub base: f64,
    pub reversibility: f64,
    pub unicode_validity: f64,
    pub replacement_penalty: f64,
    pub control_penalty: f64,
    pub script_coherence: f64,
    pub mojibake_penalty: f64,
    pub round_trip: f64,
    pub depth_penalty: f64,
    pub preservation: f64,
}

impl ScoreBreakdown {
    pub fn total(&self) -> f64 {
        self.base
            + self.reversibility
            + self.unicode_validity
            + self.replacement_penalty
            + self.control_penalty
            + self.script_coherence
            + self.mojibake_penalty
            + self.round_trip
            + self.depth_penalty
            + self.preservation
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCandidate {
    pub id: String,
    pub preview: String,
    pub full_text: String,
    pub score: f64,
    pub score_breakdown: ScoreBreakdown,
    pub reversible: bool,
    pub lossy: bool,
    pub depth: usize,
    pub transformations: Vec<TransformationStep>,
    pub warnings: Vec<String>,
    pub reasons: Vec<String>,
    pub alternative_paths: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodingEvidence {
    pub bom: Option<String>,
    pub valid_utf8: Option<bool>,
    pub null_byte_pattern: String,
    pub replacement_count: usize,
    pub control_ratio: f64,
    pub ascii_ratio: f64,
    pub whitespace_ratio: f64,
    pub line_count: usize,
    pub script_distribution: BTreeMap<String, f64>,
    pub detector_suggestion: Option<String>,
    pub hex_prefix: String,
    pub sampled: bool,
    pub sample_size: usize,
    pub binary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseAnalysis {
    pub case_id: String,
    pub source_type: String,
    pub file_name: Option<String>,
    pub file_size: Option<u64>,
    pub original_text: String,
    pub evidence: EncodingEvidence,
    pub candidates: Vec<RecoveryCandidate>,
    pub warnings: Vec<String>,
    pub explored_states: usize,
    pub elapsed_ms: u64,
}
