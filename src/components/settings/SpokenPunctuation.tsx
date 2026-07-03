import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface SpokenPunctuationProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const SpokenPunctuation: React.FC<SpokenPunctuationProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("spoken_punctuation_enabled") ?? true;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("spoken_punctuation_enabled", value)}
        isUpdating={isUpdating("spoken_punctuation_enabled")}
        label={t("settings.general.spokenPunctuation.label")}
        description={t("settings.general.spokenPunctuation.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);

SpokenPunctuation.displayName = "SpokenPunctuation";
