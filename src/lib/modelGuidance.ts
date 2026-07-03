import type { ModelInfo } from "@/bindings";

// Fork-owned editorial guidance shown on model cards: what a model is actually
// good and bad at, beyond the accuracy/speed bars. Kept out of the Rust catalog
// (which upstream edits weekly) so upstream merges stay clean. Entries informed
// by published benchmarks AND live A/B tests on this machine (July 2026,
// West African accented English) — see UAT.md.
export interface ModelGuidance {
  bestFor: string[];
  notFor?: string[];
  note?: string;
}

const WHISPER_TURBO: ModelGuidance = {
  bestFor: [
    "Accented English (verified: near-verbatim on Ghanaian English)",
    "Multilingual dictation",
  ],
  note: "Winner of the live accent A/B on this machine — the recommended daily driver.",
};

const WHISPER_LARGE_V3: ModelGuidance = {
  bestFor: ["Maximum accuracy", "Difficult or noisy audio"],
  notFor: ["Snappy short dictation — slowest Whisper"],
  note: "Marginally more accurate than Turbo at ~3x the latency.",
};

const WHISPER_MEDIUM: ModelGuidance = {
  bestFor: ["Lower RAM footprint than large models"],
  notFor: ["Heavily accented speech — use Whisper Turbo"],
};

const WHISPER_SMALL: ModelGuidance = {
  bestFor: ["Quick notes", "Low RAM / battery"],
  notFor: ["Accented or noisy speech", "Proper nouns"],
};

const PARAKEET: ModelGuidance = {
  bestFor: ["Near-instant US/European English"],
  notFor: [
    "West African accented English — dropped 60-80% of words in live testing here",
  ],
  note: "English-benchmark leader, but verified weak on Ghanaian English on this machine.",
};

// Exact catalog ids (legacy blob models + ONNX models).
const BY_ID: Record<string, ModelGuidance> = {
  turbo: WHISPER_TURBO,
  large: WHISPER_LARGE_V3,
  medium: WHISPER_MEDIUM,
  small: WHISPER_SMALL,
  "parakeet-tdt-0.6b-v3": {
    ...PARAKEET,
    bestFor: [...PARAKEET.bestFor, "25 European languages"],
  },
  "parakeet-tdt-0.6b-v2": PARAKEET,
  "canary-180m-flash": {
    bestFor: ["Minimal RAM", "Quick smoke tests"],
    notFor: ["Accuracy-critical dictation", "Accented speech"],
  },
  "canary-1b-v2": {
    bestFor: ["European languages with good speed"],
    notFor: ["West African accents (US/EU training data)"],
  },
  "moonshine-base": {
    bestFor: ["Short English commands", "Very low latency"],
    notFor: ["Long-form or accented dictation"],
  },
  "moonshine-tiny-streaming-en": {
    bestFor: ["Live streaming preview", "Minimal footprint"],
    notFor: ["Accuracy-critical or accented dictation"],
  },
  "moonshine-small-streaming-en": {
    bestFor: ["Live streaming preview in English"],
    notFor: ["Accented dictation"],
  },
  "moonshine-medium-streaming-en": {
    bestFor: ["Best streaming-capable English quality"],
    notFor: ["Accented dictation"],
  },
  "sense-voice-int8": {
    bestFor: ["Chinese, Cantonese, Japanese, Korean"],
    notFor: ["English-first use — Whisper family is stronger"],
  },
  "gigaam-v3-e2e-ctc": {
    bestFor: ["Russian speech"],
    notFor: ["Everything else — Russian-specialized"],
  },
  "breeze-asr": {
    bestFor: ["Taiwanese Mandarin, Mandarin-English code-switching"],
  },
  "cohere-int8": {
    bestFor: ["Multilingual coverage in a small model"],
    notFor: ["Accented English — untested here, prefer Whisper Turbo"],
  },
};

// GGUF catalog ids look like "handy-computer/whisper-large-v3-turbo-gguf/…" —
// match by filename substring, most specific first.
const BY_SUBSTRING: Array<[string, ModelGuidance]> = [
  ["whisper-large-v3-turbo", WHISPER_TURBO],
  ["whisper-large-v3", WHISPER_LARGE_V3],
  [
    "whisper-large-v2",
    {
      ...WHISPER_LARGE_V3,
      note: "Older generation — prefer Large v3 or Turbo.",
    },
  ],
  ["whisper-large", WHISPER_LARGE_V3],
  ["whisper-medium", WHISPER_MEDIUM],
  ["whisper-small", WHISPER_SMALL],
];

export function getModelGuidance(model: ModelInfo): ModelGuidance | undefined {
  const exact = BY_ID[model.id];
  if (exact) return exact;
  const haystack = `${model.id} ${model.filename}`.toLowerCase();
  const rule = BY_SUBSTRING.find(([pattern]) => haystack.includes(pattern));
  return rule?.[1];
}

// Lets the models-page search find models by guidance text (e.g. "accent").
export function getGuidanceSearchText(model: ModelInfo): string {
  const guidance = getModelGuidance(model);
  if (!guidance) return "";
  return [
    ...guidance.bestFor,
    ...(guidance.notFor ?? []),
    guidance.note ?? "",
  ].join(" ");
}
