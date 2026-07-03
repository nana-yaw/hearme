import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Crosshair, Loader2, Trash2 } from "lucide-react";
import { commands, type AppProfile } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Button } from "../../ui/Button";

// Per-app prompt overrides: while dictating with post-processing, the prompt
// can follow the app the text will land in (email tone in Mail, terse in
// Slack). Capture uses a 3 s countdown so the user can bring the target app
// to the front before the frontmost-app snapshot is taken.
export const AppProfiles: React.FC = () => {
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettings();
  const [capturing, setCapturing] = useState(false);

  const profiles = settings?.app_profiles ?? [];
  const prompts = settings?.post_process_prompts ?? [];

  const setProfiles = (next: AppProfile[]) =>
    updateSetting("app_profiles", next);

  const addCurrentApp = () => {
    if (capturing || prompts.length === 0) return;
    setCapturing(true);
    setTimeout(async () => {
      try {
        const bundleId = await commands.getFrontmostAppBundleId();
        if (bundleId && !profiles.some((p) => p.bundle_id === bundleId)) {
          setProfiles([
            ...profiles,
            { bundle_id: bundleId, prompt_id: prompts[0].id },
          ]);
        }
      } finally {
        setCapturing(false);
      }
    }, 3000);
  };

  const setPromptFor = (index: number, promptId: string) =>
    setProfiles(
      profiles.map((p, i) => (i === index ? { ...p, prompt_id: promptId } : p)),
    );

  const remove = (index: number) =>
    setProfiles(profiles.filter((_, i) => i !== index));

  return (
    <div className="flex flex-col gap-2 px-4 py-3">
      <p className="text-xs text-text/50">
        {t("settings.postProcessing.appProfiles.description")}
      </p>
      {profiles.map((profile, index) => (
        <div key={profile.bundle_id} className="flex items-center gap-2">
          <span
            className="flex-1 text-sm text-text/80 truncate"
            title={profile.bundle_id}
          >
            {profile.bundle_id}
          </span>
          <select
            value={profile.prompt_id}
            onChange={(e) => setPromptFor(index, e.target.value)}
            className="text-sm bg-mid-gray/10 border border-mid-gray/40 rounded-md px-2 py-1"
          >
            {prompts.map((prompt) => (
              <option key={prompt.id} value={prompt.id}>
                {prompt.name}
              </option>
            ))}
          </select>
          <button
            onClick={() => remove(index)}
            title={t("common.delete")}
            className="p-1 text-text/50 hover:text-logo-primary"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      ))}
      <div>
        <Button
          onClick={addCurrentApp}
          variant="secondary"
          size="sm"
          disabled={capturing || prompts.length === 0}
          className="flex items-center gap-2"
        >
          {capturing ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Crosshair className="w-3.5 h-3.5" />
          )}
          <span>
            {capturing
              ? t("settings.postProcessing.appProfiles.capturing")
              : t("settings.postProcessing.appProfiles.addCurrent")}
          </span>
        </Button>
      </div>
    </div>
  );
};
