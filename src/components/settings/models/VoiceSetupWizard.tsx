import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { Award, Loader2, Mic, RotateCcw, Square } from "lucide-react";
import {
  commands,
  type BenchmarkModelResult,
  type ModelInfo,
} from "@/bindings";
import type {
  BenchmarkCompleteEvent,
  BenchmarkProgressEvent,
} from "@/lib/types/events";
import { buildBenchmarkPhrases } from "@/lib/benchmarkPhrases";
import { useModelStore } from "@/stores/modelStore";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";

// Voice-setup wizard: record the same few phrases once, run every downloaded
// model over them, and pick the best model for THIS user's voice with data.
// Recorded AUDIO lives only in Rust memory (never history/disk) and is
// discarded when the wizard closes; transcripts flow through the standard
// pipeline, which logs locally like any dictation.

type WizardStep = "intro" | "record" | "running" | "results";

interface VoiceSetupWizardProps {
  open: boolean;
  onClose: () => void;
}

export const VoiceSetupWizard: React.FC<VoiceSetupWizardProps> = ({
  open,
  onClose,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();
  const { models, selectModel } = useModelStore();

  const downloadedModels = useMemo(
    () => models.filter((model: ModelInfo) => model.is_downloaded),
    [models],
  );
  const phrases = useMemo(
    () => buildBenchmarkPhrases(settings?.custom_words ?? []),
    [settings?.custom_words],
  );

  const [step, setStep] = useState<WizardStep>("intro");
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [phraseIndex, setPhraseIndex] = useState(0);
  const [recordedSecs, setRecordedSecs] = useState<(number | null)[]>([]);
  const [quietPhrases, setQuietPhrases] = useState<Set<number>>(new Set());
  const [isRecording, setIsRecording] = useState(false);
  const [micLevel, setMicLevel] = useState(0);
  const [progress, setProgress] = useState<BenchmarkProgressEvent | null>(null);
  const [outcome, setOutcome] = useState<BenchmarkCompleteEvent | null>(null);
  const [applyingId, setApplyingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const isRecordingRef = useRef(false);
  isRecordingRef.current = isRecording;
  const stepRef = useRef(step);
  stepRef.current = step;

  // Fresh state each time the wizard opens; default to testing every model.
  useEffect(() => {
    if (!open) return;
    setStep("intro");
    setSelectedIds(new Set(downloadedModels.map((model) => model.id)));
    setPhraseIndex(0);
    setRecordedSecs(new Array(phrases.length).fill(null));
    setQuietPhrases(new Set());
    setIsRecording(false);
    setProgress(null);
    setOutcome(null);
    setApplyingId(null);
    setError(null);
    // The model list and phrases are deliberately captured at open time.
  }, [open]);

  // Benchmark progress + completion events.
  useEffect(() => {
    if (!open) return;
    const unlistenProgress = listen<BenchmarkProgressEvent>(
      "benchmark-progress",
      (event) => setProgress(event.payload),
    );
    const unlistenComplete = listen<BenchmarkCompleteEvent>(
      "benchmark-complete",
      (event) => {
        // A cancelled run's thread can outlive the wizard that started it;
        // ignore its completion unless THIS wizard instance is mid-run.
        if (stepRef.current !== "running") return;
        setProgress(null);
        if (event.payload.cancelled) {
          setStep("record");
          return;
        }
        setOutcome(event.payload);
        setStep("results");
      },
    );
    return () => {
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, [open]);

  // Live mic level while recording (same event the recording overlay uses).
  useEffect(() => {
    if (!open || !isRecording) return;
    const unlisten = listen<number[]>("mic-level", (event) => {
      const peak = Math.max(0, ...event.payload);
      setMicLevel(Math.min(1, peak));
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [open, isRecording]);

  // Leaving the wizard mid-anything: stop recording, cancel the run, drop audio.
  const cleanupAndClose = () => {
    if (isRecordingRef.current) {
      commands.benchmarkCancelRecording();
    }
    commands.cancelModelBenchmark();
    commands.benchmarkDiscardSamples();
    onClose();
  };
  const cleanupRef = useRef(cleanupAndClose);
  cleanupRef.current = cleanupAndClose;
  useEffect(() => {
    if (!open) return;
    return () => cleanupRef.current();
    // Cleanup must run exactly once per open/close cycle.
  }, [open]);

  const toggleModel = (modelId: string) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(modelId)) {
        next.delete(modelId);
      } else {
        next.add(modelId);
      }
      return next;
    });
  };

  const startRecording = async () => {
    setError(null);
    const result = await commands.benchmarkStartRecording();
    if (result.status === "error") {
      setError(result.error);
      return;
    }
    setMicLevel(0);
    setIsRecording(true);
  };

  const stopRecording = async () => {
    const result = await commands.benchmarkStopRecording(phraseIndex);
    setIsRecording(false);
    if (result.status === "error") {
      setError(result.error);
      return;
    }
    setRecordedSecs((current) => {
      const next = [...current];
      next[phraseIndex] = result.data.duration_secs;
      return next;
    });
    setQuietPhrases((current) => {
      const next = new Set(current);
      if (result.data.too_quiet) {
        next.add(phraseIndex);
      } else {
        next.delete(phraseIndex);
      }
      return next;
    });
  };

  const allRecorded = recordedSecs.every((secs) => secs !== null);

  const runBenchmark = async () => {
    setError(null);
    const result = await commands.runModelBenchmark(
      [...selectedIds],
      phrases.map((phrase) => phrase.text),
    );
    if (result.status === "error") {
      setError(result.error);
      return;
    }
    setProgress(null);
    setStep("running");
  };

  const applyModel = async (modelId: string) => {
    setApplyingId(modelId);
    try {
      // selectModel reports failure via its boolean (it never throws) —
      // closing on false would claim success while the old model stays active.
      const applied = await selectModel(modelId);
      if (applied) {
        cleanupAndClose();
      } else {
        setError(
          useModelStore.getState().error ??
            t("settings.models.voiceSetup.applyFailed"),
        );
      }
    } finally {
      setApplyingId(null);
    }
  };

  const sortedResults = useMemo(() => {
    if (!outcome) return [];
    return [...outcome.results].sort((a, b) => {
      if (!!a.error !== !!b.error) return a.error ? 1 : -1;
      return b.accuracy - a.accuracy;
    });
  }, [outcome]);

  const phrase = phrases[phraseIndex];
  const currentModelName =
    progress &&
    (downloadedModels.find((model) => model.id === progress.model_id)?.name ??
      progress.model_id);

  const renderResultRow = (result: BenchmarkModelResult) => {
    const isRecommended = result.model_id === outcome?.recommended_model_id;
    return (
      <div
        key={result.model_id}
        className={`rounded-lg border p-3 space-y-1.5 ${
          isRecommended
            ? "border-logo-primary bg-logo-primary/10"
            : "border-mid-gray/30"
        }`}
      >
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-1.5 min-w-0">
            {isRecommended && (
              <Award className="w-4 h-4 shrink-0 text-logo-primary" />
            )}
            <span className="text-sm font-medium truncate">
              {result.model_name}
            </span>
            {isRecommended && (
              <span className="text-xs text-logo-primary shrink-0">
                {t("settings.models.voiceSetup.results.recommended")}
              </span>
            )}
          </div>
          {!result.error && (
            <Button
              size="sm"
              variant={isRecommended ? "primary-soft" : "secondary"}
              disabled={applyingId !== null}
              onClick={() => applyModel(result.model_id)}
            >
              {applyingId === result.model_id ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                t("settings.models.voiceSetup.results.use")
              )}
            </Button>
          )}
        </div>
        {result.error ? (
          <p className="text-xs text-red-400">{result.error}</p>
        ) : (
          <>
            <div className="flex gap-4 text-xs text-text/70">
              <span>
                {t("settings.models.voiceSetup.results.accuracy")}{" "}
                {`${Math.round(result.accuracy * 100)}%`}
              </span>
              <span>
                {t("settings.models.voiceSetup.results.speed")}{" "}
                {`${result.speed_factor.toFixed(1)}x`}
              </span>
              <span>
                {t("settings.models.voiceSetup.results.loadTime")}{" "}
                {`${(result.load_ms / 1000).toFixed(1)}s`}
              </span>
            </div>
            <details className="text-xs text-text/60">
              <summary className="cursor-pointer select-none">
                {t("settings.models.voiceSetup.results.heard")}
              </summary>
              <ol className="mt-1 ms-4 list-decimal space-y-0.5">
                {result.transcripts.map((transcript, index) => (
                  <li key={index}>{transcript}</li>
                ))}
              </ol>
            </details>
          </>
        )}
      </div>
    );
  };

  return (
    <Dialog
      open={open}
      title={t("settings.models.voiceSetup.title")}
      onOpenChange={(next) => {
        if (!next) cleanupAndClose();
      }}
      closeLabel={t("settings.models.voiceSetup.close")}
      closeOnBackdrop={false}
      contentFades={false}
      className="max-w-xl"
    >
      <div className="space-y-4">
        {error && (
          <p className="text-xs text-red-400 border border-red-400/30 rounded-md px-3 py-2">
            {error}
          </p>
        )}

        {step === "intro" && (
          <>
            <p className="text-sm text-text/70">
              {t("settings.models.voiceSetup.intro")}
            </p>
            <p className="text-xs text-text/50">
              {t("settings.models.voiceSetup.englishNote")}
            </p>
            <div className="space-y-2">
              <h3 className="text-sm font-medium">
                {t("settings.models.voiceSetup.modelsLabel")}
              </h3>
              {downloadedModels.map((model) => (
                <label
                  key={model.id}
                  className="flex items-center gap-2 text-sm cursor-pointer"
                >
                  <input
                    type="checkbox"
                    checked={selectedIds.has(model.id)}
                    onChange={() => toggleModel(model.id)}
                    className="accent-logo-primary"
                  />
                  <span>{model.name}</span>
                </label>
              ))}
              {downloadedModels.length === 0 && (
                <p className="text-xs text-text/50">
                  {t("settings.models.voiceSetup.needModels")}
                </p>
              )}
            </div>
            <div className="flex justify-end">
              <Button
                onClick={() => setStep("record")}
                disabled={selectedIds.size === 0}
              >
                {t("settings.models.voiceSetup.begin")}
              </Button>
            </div>
          </>
        )}

        {step === "record" && (
          <>
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">
                {t("settings.models.voiceSetup.phraseProgress", {
                  current: phraseIndex + 1,
                  total: phrases.length,
                })}
              </span>
              <span className="text-xs text-text/50">
                {t("settings.models.voiceSetup.readAloud")}
              </span>
            </div>
            {phrase.isWordList && (
              <p className="text-xs text-text/50">
                {t("settings.models.voiceSetup.customWordsHint")}
              </p>
            )}
            <blockquote className="rounded-lg border border-mid-gray/30 bg-mid-gray/5 px-4 py-3 text-base leading-relaxed">
              {phrase.text}
            </blockquote>

            <div className="flex items-center gap-3">
              {isRecording ? (
                <Button variant="danger" onClick={stopRecording}>
                  <span className="flex items-center gap-2">
                    <Square className="w-3.5 h-3.5" />
                    <span>{t("settings.models.voiceSetup.stop")}</span>
                  </span>
                </Button>
              ) : (
                <Button onClick={startRecording}>
                  <span className="flex items-center gap-2">
                    <Mic className="w-3.5 h-3.5" />
                    <span>
                      {recordedSecs[phraseIndex] !== null
                        ? t("settings.models.voiceSetup.reRecord")
                        : t("settings.models.voiceSetup.record")}
                    </span>
                  </span>
                </Button>
              )}
              {isRecording && (
                <div className="flex-1 h-2 rounded-full bg-mid-gray/20 overflow-hidden">
                  <div
                    className="h-full bg-logo-primary transition-[width] duration-100"
                    style={{ width: `${Math.round(micLevel * 100)}%` }}
                  />
                </div>
              )}
              {!isRecording && recordedSecs[phraseIndex] !== null && (
                <span className="text-xs text-text/60">
                  {t("settings.models.voiceSetup.recorded", {
                    seconds: (recordedSecs[phraseIndex] ?? 0).toFixed(1),
                  })}
                </span>
              )}
            </div>
            {quietPhrases.has(phraseIndex) && (
              <p className="text-xs text-amber-400">
                {t("settings.models.voiceSetup.tooQuiet")}
              </p>
            )}

            <div className="flex justify-between">
              <Button
                variant="ghost"
                disabled={isRecording}
                onClick={() =>
                  phraseIndex === 0
                    ? setStep("intro")
                    : setPhraseIndex(phraseIndex - 1)
                }
              >
                {t("settings.models.voiceSetup.back")}
              </Button>
              {phraseIndex < phrases.length - 1 ? (
                <Button
                  disabled={isRecording || recordedSecs[phraseIndex] === null}
                  onClick={() => setPhraseIndex(phraseIndex + 1)}
                >
                  {t("settings.models.voiceSetup.next")}
                </Button>
              ) : (
                <Button
                  disabled={isRecording || !allRecorded}
                  onClick={runBenchmark}
                >
                  {t("settings.models.voiceSetup.start")}
                </Button>
              )}
            </div>
          </>
        )}

        {step === "running" && (
          <div className="flex flex-col items-center gap-3 py-6">
            <Loader2 className="w-6 h-6 animate-spin text-logo-primary" />
            <p className="text-sm font-medium">
              {t("settings.models.voiceSetup.running.title")}
            </p>
            {progress && currentModelName && (
              <p className="text-xs text-text/60">
                {progress.stage === "loading"
                  ? t("settings.models.voiceSetup.running.loading", {
                      model: currentModelName,
                    })
                  : t("settings.models.voiceSetup.running.transcribing", {
                      model: currentModelName,
                      current: progress.phrase_index + 1,
                      total: progress.total_phrases,
                    })}
              </p>
            )}
            {progress && (
              <div className="w-full h-1.5 rounded-full bg-mid-gray/20 overflow-hidden">
                <div
                  className="h-full bg-logo-primary transition-[width] duration-300"
                  style={{
                    width: `${Math.round(
                      ((progress.model_index * progress.total_phrases +
                        progress.phrase_index) /
                        (progress.total_models * progress.total_phrases)) *
                        100,
                    )}%`,
                  }}
                />
              </div>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => commands.cancelModelBenchmark()}
            >
              {t("settings.models.voiceSetup.cancel")}
            </Button>
          </div>
        )}

        {step === "results" && outcome && (
          <>
            <p className="text-sm text-text/70">
              {outcome.recommended_model_id
                ? t("settings.models.voiceSetup.results.summary")
                : t("settings.models.voiceSetup.results.allFailed")}
            </p>
            <div className="space-y-2">
              {sortedResults.map(renderResultRow)}
            </div>
            <div className="flex justify-between">
              <Button
                variant="ghost"
                onClick={() => {
                  setOutcome(null);
                  setStep("record");
                }}
              >
                <span className="flex items-center gap-1.5">
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>{t("settings.models.voiceSetup.retest")}</span>
                </span>
              </Button>
              <Button variant="secondary" onClick={cleanupAndClose}>
                {t("settings.models.voiceSetup.close")}
              </Button>
            </div>
          </>
        )}
      </div>
    </Dialog>
  );
};
