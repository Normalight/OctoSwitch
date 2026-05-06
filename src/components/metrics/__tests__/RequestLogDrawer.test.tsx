import type { ReactNode } from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";
import { I18nProvider } from "../../../i18n";
import { RequestLogDrawer } from "../RequestLogDrawer";

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: () => ({
    getTotalSize: () => 38,
    getVirtualItems: () => [{ index: 0, start: 0 }],
  }),
}));

function renderZh(ui: ReactNode) {
  localStorage.setItem("os-locale", "zh-CN");
  return render(<I18nProvider>{ui}</I18nProvider>);
}

describe("RequestLogDrawer", () => {
  test("shows merged input header and merged input value", () => {
    renderZh(
      <RequestLogDrawer
        logs={[
          {
            id: "1",
            group_name: "Sonnet",
            model_name: "claude-3-7-sonnet",
            provider_name: "Anthropic",
            latency_ms: 500,
            input_tokens: 1200,
            output_tokens: 800,
            cache_read_tokens: 300,
            status_code: 200,
            created_at: "2026-05-06T10:00:00Z",
          },
        ]}
      />
    );

    act(() => {
      fireEvent.click(screen.getByText("区间内请求（最多 500 条）"));
    });

    expect(screen.getByText("输入（含缓存）")).toBeInTheDocument();
    expect(screen.getAllByText("1.50K").length).toBeGreaterThan(0);
    expect(screen.queryByText("1.20K + 300")).not.toBeInTheDocument();
  });
});
