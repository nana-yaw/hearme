import type { BenchmarkModelResult } from "@/bindings";

export interface ModelStateEvent {
  event_type: string;
  model_id?: string;
  model_name?: string;
  error?: string;
}

export interface RecordingErrorEvent {
  error_type: string;
  detail?: string;
}

export interface RecordingWarningEvent {
  warning_type: string;
  level_dbfs: number;
}

export interface BenchmarkProgressEvent {
  model_id: string;
  model_index: number;
  total_models: number;
  phrase_index: number;
  total_phrases: number;
  stage: "loading" | "transcribing";
}

export interface BenchmarkCompleteEvent {
  results: BenchmarkModelResult[];
  recommended_model_id: string | null;
  cancelled: boolean;
}
