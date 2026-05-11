import { type FC } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../ui/SettingContainer";
import { Dropdown } from "../ui/Dropdown";
import { useSettings } from "../../hooks/useSettings";

interface AccelerationSelectorProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const AccelerationSelector: FC<AccelerationSelectorProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const currentOrt = getSetting("ort_accelerator") ?? "auto";

  const ortOptions = [
    { value: "auto", label: "Auto" },
    { value: "cpu", label: "CPU" },
  ];

  return (
    <SettingContainer
      title={t("settings.advanced.acceleration.ort.title")}
      description={t("settings.advanced.acceleration.ort.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="horizontal"
    >
      <Dropdown
        options={ortOptions}
        selectedValue={currentOrt}
        onSelect={(value) => updateSetting("ort_accelerator", value as "auto" | "cpu")}
        disabled={isUpdating("ort_accelerator")}
      />
    </SettingContainer>
  );
};
