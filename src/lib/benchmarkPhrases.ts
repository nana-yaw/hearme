// English benchmark stimuli for the voice-setup wizard. These stay in code
// (not locale files): the WER reference must be byte-identical to what the
// user is shown to read, and the benchmark measures English transcription.
// Numerals are avoided on purpose — models write "9:30" for "nine thirty" and
// word-level scoring would count honest transcriptions wrong.

export interface BenchmarkPhrase {
  /** Shown to the user to read, and used verbatim as the WER reference. */
  text: string;
  /** Custom-words phrase: rendered as a word list, read with short pauses. */
  isWordList: boolean;
}

const GENERAL_PHRASES = [
  "The quick brown fox jumps over the lazy dog while the children watch from the veranda.",
  "Please send the updated invoice to the finance team before the end of the week and copy the operations manager.",
] as const;

const FALLBACK_PHRASE =
  "Everyone agreed the proposal was ambitious, but the budget review still raised several difficult questions.";

const MAX_CUSTOM_WORDS = 6;

/**
 * Two general phrases plus a third built from the user's custom words (the
 * proper nouns models most often miss), falling back to a general phrase when
 * fewer than two custom words exist.
 */
export const buildBenchmarkPhrases = (
  customWords: string[],
): BenchmarkPhrase[] => {
  const words = customWords
    .map((word) => word.trim())
    .filter(Boolean)
    .slice(0, MAX_CUSTOM_WORDS);
  const thirdPhrase: BenchmarkPhrase =
    words.length >= 2
      ? { text: words.join(", "), isWordList: true }
      : { text: FALLBACK_PHRASE, isWordList: false };
  return [
    { text: GENERAL_PHRASES[0], isWordList: false },
    { text: GENERAL_PHRASES[1], isWordList: false },
    thirdPhrase,
  ];
};
