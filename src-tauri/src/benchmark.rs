//! Voice-setup benchmark scoring: the user records a few reference phrases
//! once, every downloaded model transcribes those same recordings, and each
//! model is scored on accuracy (word error rate against the phrase) and speed
//! (real-time factor). Pure logic lives here so it is unit-testable without
//! audio hardware or loaded models; the recording/orchestration side is in
//! `commands::benchmark`.

use serde::Serialize;
use specta::Type;

/// One model's aggregate over all recorded phrases. `error` marks a model that
/// failed to load or transcribe — it stays in the results list (the UI shows
/// what happened) but is excluded from the recommendation.
#[derive(Serialize, Clone, Debug, Type)]
pub struct BenchmarkModelResult {
    pub model_id: String,
    pub model_name: String,
    /// Mean of (1 − WER) over phrases, 0..1.
    pub accuracy: f64,
    /// Mean real-time factor (audio seconds per compute second); higher is faster.
    pub speed_factor: f64,
    /// Wall-clock time of the initial model load.
    pub load_ms: u64,
    /// What the model heard, phrase by phrase, so the numbers can be sanity-read.
    pub transcripts: Vec<String>,
    pub error: Option<String>,
}

/// Lowercased words with punctuation stripped. Apostrophes stay word-internal
/// ("don't" is one word); every other non-alphanumeric splits, so hyphenation
/// differences ("part-time" vs "part time") normalize identically on both
/// sides. Numerals are avoided in the built-in phrases instead of normalized —
/// "nine" vs "9" cannot be reconciled fairly here.
fn normalize_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|word| word.trim_matches('\''))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// Word-level Levenshtein distance (substitutions, insertions, deletions all
/// cost 1), single-row DP.
fn word_edit_distance(reference: &[String], hypothesis: &[String]) -> usize {
    let mut previous_row: Vec<usize> = (0..=hypothesis.len()).collect();
    for (i, ref_word) in reference.iter().enumerate() {
        let mut current_row = vec![i + 1];
        for (j, hyp_word) in hypothesis.iter().enumerate() {
            let substitution = previous_row[j] + usize::from(ref_word != hyp_word);
            let insertion = current_row[j] + 1;
            let deletion = previous_row[j + 1] + 1;
            current_row.push(substitution.min(insertion).min(deletion));
        }
        previous_row = current_row;
    }
    previous_row[hypothesis.len()]
}

/// Word error rate of `hypothesis` against `reference`, case- and
/// punctuation-insensitive, capped at 1.0 so a degenerate hypothesis (long
/// hallucination) cannot push accuracy below zero.
pub fn word_error_rate(reference: &str, hypothesis: &str) -> f64 {
    let reference_words = normalize_words(reference);
    let hypothesis_words = normalize_words(hypothesis);
    if reference_words.is_empty() {
        return if hypothesis_words.is_empty() {
            0.0
        } else {
            1.0
        };
    }
    let distance = word_edit_distance(&reference_words, &hypothesis_words);
    (distance as f64 / reference_words.len() as f64).min(1.0)
}

/// How close to the top accuracy a model must be for speed to break the tie.
/// 0.02 ≈ one substituted word in a fifty-word session — inside phrase-reading
/// noise, so preferring the faster model there matches how the live A/B was
/// judged (Turbo won on accuracy AND speed; a near-tie should not pick the
/// slower model over noise).
const ACCURACY_TIE_MARGIN: f64 = 0.02;

/// Pick the model to recommend: highest accuracy, with speed breaking
/// near-ties (within [`ACCURACY_TIE_MARGIN`]). Errored models never win.
/// `None` when every model errored.
pub fn recommend(results: &[BenchmarkModelResult]) -> Option<String> {
    let scored = || results.iter().filter(|result| result.error.is_none());
    let best_accuracy = scored()
        .map(|result| result.accuracy)
        .max_by(f64::total_cmp)?;
    scored()
        .filter(|result| best_accuracy - result.accuracy <= ACCURACY_TIE_MARGIN)
        .max_by(|a, b| a.speed_factor.total_cmp(&b.speed_factor))
        .map(|result| result.model_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(model_id: &str, accuracy: f64, speed_factor: f64) -> BenchmarkModelResult {
        BenchmarkModelResult {
            model_id: model_id.to_string(),
            model_name: model_id.to_string(),
            accuracy,
            speed_factor,
            load_ms: 0,
            transcripts: vec![],
            error: None,
        }
    }

    #[test]
    fn wer_zero_for_identical_text() {
        assert_eq!(
            word_error_rate("the quick brown fox", "the quick brown fox"),
            0.0
        );
    }

    #[test]
    fn wer_ignores_case_and_punctuation() {
        assert_eq!(
            word_error_rate(
                "Please send the invoice, then copy the manager.",
                "please send the invoice then copy the manager"
            ),
            0.0
        );
    }

    #[test]
    fn wer_normalizes_hyphenation_both_ways() {
        assert_eq!(word_error_rate("a part-time role", "a part time role"), 0.0);
    }

    #[test]
    fn wer_keeps_apostrophes_word_internal() {
        assert_eq!(word_error_rate("don't stop now", "don't stop now"), 0.0);
        // "dont" vs "don't" differs after normalization — that's a real miss.
        assert!(word_error_rate("don't stop now", "dont stop now") > 0.0);
    }

    #[test]
    fn wer_counts_deletions() {
        // Parakeet-style word dropping: 3 of 6 reference words missing.
        let wer = word_error_rate(
            "the children watch from the veranda",
            "the children veranda",
        );
        assert!((wer - 0.5).abs() < 1e-9);
    }

    #[test]
    fn wer_caps_at_one_for_hallucination() {
        let wer = word_error_rate(
            "short phrase",
            "a very long hallucinated transcript with many extra words",
        );
        assert_eq!(wer, 1.0);
    }

    #[test]
    fn wer_empty_hypothesis_is_total_error() {
        assert_eq!(word_error_rate("anything at all", ""), 1.0);
    }

    #[test]
    fn wer_empty_reference_edge_cases() {
        assert_eq!(word_error_rate("", ""), 0.0);
        assert_eq!(word_error_rate("", "noise"), 1.0);
    }

    #[test]
    fn recommend_picks_highest_accuracy() {
        let results = vec![
            result("fast-but-wrong", 0.60, 30.0),
            result("turbo", 0.95, 5.0),
        ];
        assert_eq!(recommend(&results).as_deref(), Some("turbo"));
    }

    #[test]
    fn recommend_breaks_near_tie_by_speed() {
        let results = vec![
            result("accurate-slow", 0.96, 2.0),
            result("accurate-fast", 0.95, 8.0),
        ];
        assert_eq!(recommend(&results).as_deref(), Some("accurate-fast"));
    }

    #[test]
    fn recommend_ignores_speed_outside_margin() {
        let results = vec![result("accurate", 0.95, 2.0), result("fast", 0.90, 20.0)];
        assert_eq!(recommend(&results).as_deref(), Some("accurate"));
    }

    #[test]
    fn recommend_excludes_errored_models() {
        let mut errored = result("broken", 1.0, 100.0);
        errored.error = Some("failed to load".to_string());
        let results = vec![errored, result("working", 0.80, 3.0)];
        assert_eq!(recommend(&results).as_deref(), Some("working"));
    }

    #[test]
    fn recommend_none_when_all_errored() {
        let mut errored = result("broken", 0.9, 1.0);
        errored.error = Some("failed".to_string());
        assert_eq!(recommend(&[errored]), None);
    }
}
