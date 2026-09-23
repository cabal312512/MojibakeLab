use crate::{evidence::Metrics, ScoreBreakdown};

/// All ranking weights live together. Scores rank hypotheses; they are not
/// probabilities and deliberately give script composition little influence.
pub struct ScoringWeights {
    pub base: f64,
    pub reversible: f64,
    pub valid_unicode: f64,
    pub replacement: f64,
    pub replacement_cap: f64,
    pub control_ratio: f64,
    pub script_coherence: f64,
    pub suspicious: f64,
    pub suspicious_cap: f64,
    pub round_trip: f64,
    pub depth: f64,
    pub clean_preservation: f64,
    pub detector: f64,
    pub bom_or_utf16_pattern: f64,
    pub unsupported_utf16: f64,
}

pub const WEIGHTS: ScoringWeights = ScoringWeights {
    base: 70.0,
    reversible: 8.0,
    valid_unicode: 5.0,
    replacement: 12.0,
    replacement_cap: 60.0,
    control_ratio: 100.0,
    script_coherence: 2.0,
    suspicious: 8.0,
    suspicious_cap: 40.0,
    round_trip: 5.0,
    depth: 2.0,
    clean_preservation: 10.0,
    detector: 3.0,
    bom_or_utf16_pattern: 12.0,
    unsupported_utf16: 12.0,
};

pub(crate) fn score(
    values: &Metrics,
    reversible: bool,
    lossy: bool,
    depth: usize,
    preserve: bool,
    detector_match: bool,
    bom_or_pattern: bool,
) -> ScoreBreakdown {
    // ASCII, mixed Han/Latin, Japanese and source code remain valid. This tiny
    // preference only differentiates extremely fragmented script combinations.
    let meaningful_scripts = values
        .scripts
        .keys()
        .filter(|name| name.as_str() != "Common")
        .count();
    ScoreBreakdown {
        base: WEIGHTS.base,
        reversibility: if reversible {
            WEIGHTS.reversible
        } else {
            -WEIGHTS.reversible
        },
        unicode_validity: if !lossy { WEIGHTS.valid_unicode } else { 0.0 },
        replacement_penalty: -(values.replacements as f64 * WEIGHTS.replacement)
            .min(WEIGHTS.replacement_cap),
        control_penalty: -values.ratio(values.controls) * WEIGHTS.control_ratio,
        script_coherence: if meaningful_scripts <= 3 {
            WEIGHTS.script_coherence
        } else {
            0.0
        },
        mojibake_penalty: -(values.suspicious as f64 * WEIGHTS.suspicious)
            .min(WEIGHTS.suspicious_cap),
        round_trip: if reversible { WEIGHTS.round_trip } else { 0.0 },
        depth_penalty: -(depth as f64) * WEIGHTS.depth,
        preservation: if preserve && values.clean() {
            WEIGHTS.clean_preservation
        } else {
            0.0
        } + if detector_match {
            WEIGHTS.detector
        } else {
            0.0
        } + if bom_or_pattern {
            WEIGHTS.bom_or_utf16_pattern
        } else {
            0.0
        },
    }
}
