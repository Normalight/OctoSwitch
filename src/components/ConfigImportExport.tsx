import { useRef, useState } from "react";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { CONFIG_IMPORTED } from "../lib/constants";

type Props = {
  onImported?: () => void;
};

export function ConfigImportExport({ onImported }: Props) {
  const { t } = useI18n();
  const fileRef = useRef<HTMLInputElement>(null);
  const [message, setMessage] = useState<string | null>(null);

  const exportJson = async () => {
    setMessage(null);
    try {
      const json = await tauriApi.exportConfig();
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "octoswitch-config.json";
      a.click();
      URL.revokeObjectURL(url);
      setMessage(t("configIo.exported"));
    } catch (e) {
      setMessage(String(e));
    }
  };

  const pickFile = () => fileRef.current?.click();

  const onFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    setMessage(null);
    try {
      const text = await file.text();
      await tauriApi.importConfig(text);
      setMessage(t("configIo.imported"));
      window.dispatchEvent(new CustomEvent(CONFIG_IMPORTED));
      onImported?.();
    } catch (err) {
      setMessage(String(err));
    }
  };

  return (
    <div className="config-io">
      <input
        ref={fileRef}
        type="file"
        accept="application/json,.json"
        className="sr-only"
        onChange={(e) => void onFile(e)}
      />
      <div className="config-io-actions" role="group" aria-label={t("configIo.aria")}>
        <button type="button" className="btn btn--ghost btn--sm config-io-btn" onClick={() => void exportJson()}>
          {t("configIo.export")}
        </button>
        <button type="button" className="btn btn--ghost btn--sm config-io-btn" onClick={pickFile}>
          {t("configIo.import")}
        </button>
      </div>
      {message ? <span className="config-io-msg">{message}</span> : null}
    </div>
  );
}
