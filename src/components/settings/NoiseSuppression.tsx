import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface NoiseSuppressionProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const NoiseSuppression: React.FC<NoiseSuppressionProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("noise_suppression_enabled") ?? false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("noise_suppression_enabled", value)}
        isUpdating={isUpdating("noise_suppression_enabled")}
        label={t("settings.advanced.noiseSuppression.label")}
        description={t("settings.advanced.noiseSuppression.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);

NoiseSuppression.displayName = "NoiseSuppression";
