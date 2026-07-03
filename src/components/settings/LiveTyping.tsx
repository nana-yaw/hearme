import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface LiveTypingProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const LiveTyping: React.FC<LiveTypingProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const enabled = getSetting("live_typing_enabled") ?? false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("live_typing_enabled", value)}
        isUpdating={isUpdating("live_typing_enabled")}
        label={t("settings.advanced.liveTyping.label")}
        description={t("settings.advanced.liveTyping.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);

LiveTyping.displayName = "LiveTyping";
