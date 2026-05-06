import type { ReactNode } from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";
import { I18nProvider } from "../../../i18n";
import { KpiCards } from "../KpiCards";

function renderZh(ui: ReactNode) {
  localStorage.setItem("os-locale", "zh-CN");
  return render(<I18nProvider>{ui}</I18nProvider>);
}

describe("KpiCards", () => {
  test("shows input including cache read tokens", () => {
    renderZh(
      <KpiCards
        kpi={{
          error_rate: 0,
          total_input_tokens: 1200,
          total_output_tokens: 900,
          total_cache_read_tokens: 300,
          total_consumed_tokens: 2400,
        }}
      />
    );

    expect(screen.getByText("输入（含缓存）")).toBeInTheDocument();
    expect(screen.getByText("1.50K")).toBeInTheDocument();
  });
});
