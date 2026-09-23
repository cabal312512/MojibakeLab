use crate::codec::{self, Codec, CODECS};
use crate::evidence::{hex_preview, metrics, text_preview};
use crate::{scoring, RecoveryCandidate, TransformationStep};
use std::collections::{HashMap, HashSet};

pub const MAX_DEPTH: usize = 3;
pub const BEAM_WIDTH: usize = 12;
pub const MAX_CANDIDATES: usize = 12;
pub const MAX_STATES: usize = 1024;
pub const MAX_EXPLORED: usize = 4096;
const MAX_SEARCH_WORK_BYTES: usize = 8 * 1024 * 1024;
const MAX_TRANSFORM_BYTES: usize = 128 * 1024;

/// Encoding and decoding consume different states. This is also useful to
/// consumers replaying an inspected path without confusing text and bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataState {
    Bytes(Vec<u8>),
    Text(String),
}

pub fn transform(
    state: DataState,
    codec: Codec,
) -> Result<(DataState, TransformationStep), String> {
    match state {
        DataState::Text(text) => {
            let bytes = codec::encode(&text, codec)?;
            let round_trip = codec::decode(&bytes, codec).text == text;
            let step = encode_step(&text, &bytes, codec, !round_trip);
            Ok((DataState::Bytes(bytes), step))
        }
        DataState::Bytes(bytes) => {
            let decoded = codec::decode(&bytes, codec);
            let step = decode_step(&bytes, &decoded, codec);
            Ok((DataState::Text(decoded.text), step))
        }
    }
}

fn encode_step(text: &str, bytes: &[u8], codec: Codec, lossy: bool) -> TransformationStep {
    TransformationStep {
        operation: "encode".into(),
        encoding: codec.name().into(),
        input_type: "text".into(),
        output_type: "bytes".into(),
        input_preview: text_preview(text, 160),
        output_preview: hex_preview(bytes, 48),
        input_size: text.len(),
        output_size: bytes.len(),
        lossy,
        replacement_count: 0,
    }
}

fn decode_step(bytes: &[u8], decoded: &codec::Decoded, codec: Codec) -> TransformationStep {
    TransformationStep {
        operation: "decode".into(),
        encoding: codec.name().into(),
        input_type: "bytes".into(),
        output_type: "text".into(),
        input_preview: hex_preview(bytes, 48),
        output_preview: text_preview(&decoded.text, 160),
        input_size: bytes.len(),
        output_size: decoded.text.len(),
        lossy: decoded.had_errors || !decoded.round_trip,
        replacement_count: decoded.replacement_count,
    }
}

#[derive(Clone)]
struct State {
    text: String,
    steps: Vec<TransformationStep>,
    depth: usize,
    reversible: bool,
    lossy: bool,
    inherited_loss: usize,
    preserve: bool,
    detector_match: bool,
    bom_or_pattern: bool,
    unsupported_utf16: bool,
    validated_unicode: bool,
    alternatives: usize,
    score: f64,
    score_components: crate::ScoreBreakdown,
}

impl State {
    fn score(&mut self) {
        self.score_components = self.breakdown();
        self.score = self.score_components.total();
    }
    fn breakdown(&self) -> crate::ScoreBreakdown {
        let mut values = metrics(&self.text);
        // Decoding the UTF-8 spelling of U+FFFD as a legacy encoding must not
        // launder an already observed loss into an apparently intact candidate.
        values.replacements = values.replacements.max(self.inherited_loss);
        let mut breakdown = scoring::score(
            &values,
            self.reversible,
            self.lossy,
            self.depth,
            self.preserve,
            self.detector_match,
            self.bom_or_pattern,
        );
        // Any even byte stream can look like printable UTF-16 ideographs. Keep
        // that hypothesis visible without outranking a supported repair solely
        // because random code units happened to form Han characters.
        if self.unsupported_utf16 {
            breakdown.preservation -= scoring::WEIGHTS.unsupported_utf16;
        }
        // Every Rust String is Unicode; that alone is not encoding evidence.
        // Award this component only when actual UTF-8 byte structure (or
        // explicit UTF-16 structure) validates, not any legacy decode output.
        if !self.validated_unicode {
            breakdown.unicode_validity = 0.0;
        }
        breakdown
    }
    fn candidate(self, original: &str) -> RecoveryCandidate {
        let values = metrics(&self.text);
        let original_values = metrics(original);
        let mut warnings = vec!["heuristic_not_probability".into()];
        if self.lossy {
            warnings.push("information_loss".into());
        }
        if !self.reversible {
            warnings.push("non_round_trip".into());
        }
        let mut reasons = Vec::new();
        if self.reversible {
            reasons.push("reversible_path".into());
        }
        if values.replacements == 0 && self.inherited_loss == 0 {
            reasons.push("no_replacement_characters".into());
        }
        if values.controls < original_values.controls {
            reasons.push("fewer_controls".into());
        }
        if values.suspicious < original_values.suspicious {
            reasons.push("fewer_mojibake_patterns".into());
        }
        if self.depth > 0
            && self
                .steps
                .last()
                .is_some_and(|step| step.encoding == "UTF-8")
        {
            reasons.push("utf8_recovered".into());
        }
        if self.preserve && values.clean() {
            reasons.push("preserved_original".into());
        }
        if self.detector_match {
            reasons.push("detector_match".into());
        }
        if self.bom_or_pattern {
            reasons.push("encoding_structure_match".into());
        }
        RecoveryCandidate {
            id: format!("candidate-{:016x}", stable_hash(self.text.as_bytes())),
            preview: text_preview(&self.text, 4096),
            score: self.score,
            score_breakdown: self.score_components,
            reversible: self.reversible,
            lossy: self.lossy,
            depth: self.depth,
            transformations: self.steps,
            warnings,
            reasons,
            alternative_paths: self.alternatives,
            full_text: self.text,
        }
    }
}

// FNV is used only as an index. Hash matches are followed by exact equality,
// so collisions cannot merge evidence or candidate states.
pub(crate) fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |value, byte| {
        (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn insert(
    pool: &mut Vec<State>,
    index: &mut HashMap<u64, Vec<usize>>,
    mut state: State,
) -> Option<usize> {
    state.score();
    let digest = stable_hash(state.text.as_bytes());
    if let Some(indices) = index.get(&digest) {
        if let Some(&existing) = indices.iter().find(|&&id| pool[id].text == state.text) {
            let alternative_count = pool[existing].alternatives + 1;
            if state.score > pool[existing].score {
                state.alternatives = alternative_count;
                pool[existing] = state;
            } else {
                pool[existing].alternatives = alternative_count;
            }
            return None;
        }
    }
    if pool.len() >= MAX_STATES {
        return None;
    }
    let position = pool.len();
    pool.push(state);
    index.entry(digest).or_default().push(position);
    Some(position)
}

fn rank(ids: &mut [usize], pool: &[State]) {
    ids.sort_by(|&left, &right| {
        pool[right]
            .score
            .total_cmp(&pool[left].score)
            .then(pool[left].depth.cmp(&pool[right].depth))
            .then(pool[left].steps.len().cmp(&pool[right].steps.len()))
            .then(left.cmp(&right))
    });
}

fn rank_frontier(ids: &mut [usize], pool: &[State]) {
    // A strict legacy encode → UTF-8 decode that contracts the text is a useful
    // intermediate in double mojibake, even while its remaining markers lower
    // the final-display score. Use bounded lookahead only for beam retention;
    // it does not manufacture certainty or inflate candidate scores.
    let priority = |state: &State| {
        let contraction = state.steps.len() >= 2
            && state
                .steps
                .last()
                .is_some_and(|step| step.encoding == "UTF-8")
            && state.steps[state.steps.len() - 2].input_size
                > state.steps[state.steps.len() - 2].output_size;
        state.score
            + if contraction {
                -state.score_components.mojibake_penalty
            } else {
                0.0
            }
    };
    ids.sort_by(|&left, &right| {
        priority(&pool[right])
            .total_cmp(&priority(&pool[left]))
            .then(pool[left].depth.cmp(&pool[right].depth))
            .then(left.cmp(&right))
    });
}

pub(crate) fn search(
    original: &str,
    raw: Option<&[u8]>,
    bom: Option<Codec>,
    detector: Option<Codec>,
    utf16_pattern: Option<Codec>,
) -> (Vec<RecoveryCandidate>, usize) {
    let mut pool = Vec::<State>::new();
    let mut index = HashMap::new();
    let mut frontier = Vec::new();
    let mut explored = 0;
    // Keep worst-case work proportional to a fixed byte budget as evidence
    // grows. This limits expensive legacy encode/decode attempts, not the
    // preserved input or exportable candidate text.
    let max_explored = MAX_EXPLORED.min((MAX_SEARCH_WORK_BYTES / original.len().max(1)).max(192));
    let original_values = metrics(original);
    // Clean, valid Unicode has a strong unchanged baseline. Inspect one repair
    // layer for alternatives, but do not spend three layers inventing further
    // histories from already legible text or from its intentionally wrong raw
    // decodes. Visible corruption still receives the complete depth budget.
    let clean_unicode_source = original_values.clean()
        && (bom.is_some()
            || utf16_pattern.is_some()
            || raw.is_none_or(|bytes| std::str::from_utf8(bytes).is_ok()));
    let inherited_loss = if raw.is_none()
        || raw.is_some_and(|bytes| std::str::from_utf8(bytes).is_ok())
        || bom.is_some()
    {
        original_values.replacements
    } else {
        0
    };
    if let Some(bytes) = raw {
        for codec in CODECS {
            if bom.is_some_and(|expected| codec != expected) {
                continue;
            }
            let decoded = codec::decode(bytes, codec);
            explored += 1;
            let state = State {
                steps: vec![decode_step(bytes, &decoded, codec)],
                text: decoded.text,
                depth: 0,
                reversible: decoded.round_trip && inherited_loss == 0,
                lossy: decoded.had_errors || !decoded.round_trip || inherited_loss > 0,
                inherited_loss,
                preserve: codec == Codec::Utf8 && original_values.clean(),
                // chardetng names the shared GBK/GB18030 decoder family GBK.
                // Its clue also supports the GB18030 encoder when four-byte
                // characters prevent a GBK byte-for-byte round trip.
                detector_match: detector == Some(codec)
                    || (detector == Some(Codec::Gbk) && codec == Codec::Gb18030),
                bom_or_pattern: bom == Some(codec) || utf16_pattern == Some(codec),
                unsupported_utf16: matches!(codec, Codec::Utf16Le | Codec::Utf16Be)
                    && bom != Some(codec)
                    && utf16_pattern != Some(codec),
                validated_unicode: codec == Codec::Utf8
                    || bom == Some(codec)
                    || utf16_pattern == Some(codec),
                alternatives: 0,
                score: 0.0,
                score_components: Default::default(),
            };
            if let Some(id) = insert(&mut pool, &mut index, state) {
                frontier.push(id);
            }
        }
    } else {
        let state = State {
            text: original.into(),
            steps: Vec::new(),
            depth: 0,
            reversible: inherited_loss == 0,
            lossy: inherited_loss > 0,
            inherited_loss,
            preserve: true,
            detector_match: false,
            bom_or_pattern: false,
            unsupported_utf16: false,
            validated_unicode: true,
            alternatives: 0,
            score: 0.0,
            score_components: Default::default(),
        };
        if let Some(id) = insert(&mut pool, &mut index, state) {
            frontier.push(id);
        }
        explored = 1;
    }
    rank(&mut frontier, &pool);
    if clean_unicode_source {
        frontier.retain(|&id| pool[id].text == original);
    }
    frontier.truncate(BEAM_WIDTH);
    let mut expanded = HashSet::new();
    let depth_limit = if clean_unicode_source { 1 } else { MAX_DEPTH };
    'search: for depth in 1..=depth_limit {
        let mut next = Vec::new();
        for id in frontier {
            if !expanded.insert(id) {
                continue;
            }
            let parent = pool[id].clone();
            let parent_values = metrics(&parent.text);
            if !parent.reversible
                || parent.text.is_empty()
                || parent_values.ratio(parent_values.controls) > 0.10
            {
                continue;
            }
            for wrong_codec in CODECS {
                // UTF-16 encoded from ordinary Unicode creates vast numbers of
                // arbitrary but printable legacy interpretations. Only attempt
                // this direction if the text already has an actual NUL clue.
                if matches!(wrong_codec, Codec::Utf16Le | Codec::Utf16Be)
                    && !parent.text.contains('\0')
                {
                    continue;
                }
                let Ok(bytes) = codec::encode(&parent.text, wrong_codec) else {
                    continue;
                };
                if bytes.len() > MAX_TRANSFORM_BYTES {
                    continue;
                }
                let inverse = codec::decode(&bytes, wrong_codec);
                if inverse.had_errors || inverse.text != parent.text {
                    continue;
                }
                for target_codec in CODECS {
                    if target_codec == wrong_codec {
                        continue;
                    }
                    if matches!(target_codec, Codec::Utf16Le | Codec::Utf16Be)
                        && crate::evidence::null_pattern(&bytes) == "none"
                    {
                        continue;
                    }
                    if explored >= max_explored || pool.len() >= MAX_STATES {
                        break 'search;
                    }
                    explored += 1;
                    let decoded = codec::decode(&bytes, target_codec);
                    if decoded.text == parent.text
                        || decoded.text.is_empty()
                        || !decoded.round_trip
                        || decoded.had_errors
                        || decoded.replacement_count > 0
                        || decoded.text.len() > MAX_TRANSFORM_BYTES
                    {
                        continue;
                    }
                    let values = metrics(&decoded.text);
                    if values.ratio(values.controls) > 0.10 {
                        continue;
                    }
                    let mut steps = parent.steps.clone();
                    steps.push(encode_step(&parent.text, &bytes, wrong_codec, false));
                    steps.push(decode_step(&bytes, &decoded, target_codec));
                    let state = State {
                        text: decoded.text,
                        steps,
                        depth,
                        reversible: parent.reversible,
                        lossy: parent.lossy,
                        inherited_loss: parent.inherited_loss,
                        preserve: false,
                        detector_match: false,
                        bom_or_pattern: false,
                        unsupported_utf16: parent.unsupported_utf16,
                        validated_unicode: matches!(
                            target_codec,
                            Codec::Utf8 | Codec::Utf16Le | Codec::Utf16Be
                        ),
                        alternatives: 0,
                        score: 0.0,
                        score_components: Default::default(),
                    };
                    if let Some(new_id) = insert(&mut pool, &mut index, state) {
                        next.push(new_id);
                    }
                }
            }
        }
        rank_frontier(&mut next, &pool);
        next.truncate(BEAM_WIDTH);
        frontier = next;
        if frontier.is_empty() || explored >= max_explored || pool.len() >= MAX_STATES {
            break;
        }
    }
    let mut selected: Vec<usize> = (0..pool.len()).collect();
    rank(&mut selected, &pool);
    selected.truncate(MAX_CANDIDATES);
    // The unchanged interpretation stays inspectable even if badly damaged.
    if let Some(baseline) = pool.iter().position(|state| state.text == original) {
        if !selected.contains(&baseline) {
            if selected.len() == MAX_CANDIDATES {
                selected.pop();
            }
            selected.push(baseline);
        }
    }
    let candidates = selected
        .into_iter()
        .map(|id| pool[id].clone().candidate(original))
        .collect();
    (candidates, explored)
}
