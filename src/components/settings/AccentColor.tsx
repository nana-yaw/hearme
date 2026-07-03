import React from "react";
import { useTranslation } from "react-i18next";
import { RotateCcw } from "lucide-react";
import { useSettings } from "../../hooks/useSettings";

// Curated presets around the brand pink; the native color input covers the rest.
const PRESETS = [
  "#faa2ca", // HearMe pink (default)
  "#7dd3fc", // sky
  "#86efac", // mint
  "#fcd34d", // amber
  "#c4b5fd", // violet
  "#fda4af", // coral
];

interface AccentColorProps {
  grouped?: boolean;
}

export const AccentColor: React.FC<AccentColorProps> = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();

  const accent = getSetting("accent_color") || "";
  const effective = accent || PRESETS[0];

  return (
    <div className="flex items-center justify-between px-4 py-3">
      <div className="flex flex-col">
        <span className="text-sm text-text">
          {t("settings.advanced.accentColor.label")}
        </span>
        <span className="text-xs text-text/50">
          {t("settings.advanced.accentColor.description")}
        </span>
      </div>
      <div className="flex items-center gap-1.5">
        {PRESETS.map((preset) => (
          <button
            key={preset}
            onClick={() =>
              updateSetting("accent_color", preset === PRESETS[0] ? "" : preset)
            }
            title={preset}
            className={`w-5 h-5 rounded-full border-2 transition-transform hover:scale-110 ${
              effective.toLowerCase() === preset
                ? "border-text/70"
                : "border-transparent"
            }`}
            style={{ backgroundColor: preset }}
          />
        ))}
        <input
          type="color"
          value={effective}
          onChange={(e) => updateSetting("accent_color", e.target.value)}
          title={t("settings.advanced.accentColor.custom")}
          className="w-6 h-6 rounded cursor-pointer border border-mid-gray/40 bg-transparent p-0"
        />
        {accent && (
          <button
            onClick={() => updateSetting("accent_color", "")}
            title={t("settings.advanced.accentColor.reset")}
            className="p-1 text-text/50 hover:text-logo-primary"
          >
            <RotateCcw className="w-3.5 h-3.5" />
          </button>
        )}
      </div>
    </div>
  );
});

AccentColor.displayName = "AccentColor";
