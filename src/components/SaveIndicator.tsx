import { useEffect, useState } from "react";

/**
 * Animated save indicator: checkmark appears with color flash then fades.
 * Phases: idle → check-appear (scale + green) → done (handled by caller removing the component)
 */
export function SaveIndicator({
  show,
  onDone,
  durationMs = 1400
}: {
  show: boolean;
  onDone?: () => void;
  durationMs?: number;
}) {
  const [phase, setPhase] = useState<"idle" | "active">("idle");

  useEffect(() => {
    if (!show) {
      setPhase("idle");
      return;
    }
    setPhase("active");
    const timer = setTimeout(() => {
      setPhase("idle");
      onDone?.();
    }, durationMs);
    return () => clearTimeout(timer);
  }, [show, durationMs, onDone]);

  if (phase !== "active") return null;

  return (
    <span
      className="save-indicator"
      aria-label="Saved"
      style={{
        display: "inline-flex",
        alignItems: "center",
        animation: `save-check-bounce 0.35s cubic-bezier(0.34, 1.56, 0.64, 1) forwards,
                    save-check-color 0.6s ease-in-out forwards,
                    save-check-fade 0.5s ${(durationMs - 500) / 1000}s ease-out forwards`,
      }}
    >
      <svg
        viewBox="0 0 24 24"
        width={18}
        height={18}
        fill="none"
        stroke="currentColor"
        strokeWidth={2.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <polyline points="20 6 9 17 4 12" />
      </svg>
    </span>
  );
}
