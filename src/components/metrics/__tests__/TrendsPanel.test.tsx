import type { ReactNode } from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";
import { I18nProvider } from "../../../i18n";
import { ThemeProvider } from "../../../theme/ThemeContext";
import { TrendsPanel } from "../TrendsPanel";

vi.mock("recharts", async () => {
  const React = await import("react");
  return {
    ResponsiveContainer: ({ children }: { children: ReactNode }) => <div>{children}</div>,
    LineChart: ({ children }: { children: ReactNode }) => <div>{children}</div>,
    CartesianGrid: () => null,
    XAxis: () => null,
    YAxis: () => null,
    Line: () => null,
    Tooltip: ({ content }: { content: React.ReactElement }) =>
      React.cloneElement(content, {
        active: true,
        label: "2026-05-06T10:00:00Z",
        payload: [
          {
            payload: {
              consumed_tokens: 2300,
              _models: [
                {
                  group_name: "Sonnet",
                  provider_name: "Anthropic",
                  model_name: "claude-3-7-sonnet",
                  upstream_model_name: "claude-3-7-sonnet-20250219",
                  input_tokens: 1200,
                  output_tokens: 800,
                  cache_read_tokens: 300,
                },
              ],
            },
          },
        ],
      }),
  };
});

function renderZh(ui: ReactNode) {
  localStorage.setItem("os-locale", "zh-CN");
  return render(
    <I18nProvider>
      <ThemeProvider>{ui}</ThemeProvider>
    </I18nProvider>
  );
}

describe("TrendsPanel tooltip", () => {
  test("shows merged input label and merged value in tooltip", () => {
    renderZh(
      <TrendsPanel
        rangeLabel="1 小时"
        points={[
          {
            bucket_time: "2026-05-06T10:00:00Z",
            group_name: "Sonnet",
            provider_name: "Anthropic",
            model_name: "claude-3-7-sonnet",
            upstream_model_name: "claude-3-7-sonnet-20250219",
            input_tokens: 1200,
            output_tokens: 800,
            cache_read_tokens: 300,
            consumed_tokens: 2300,
          },
        ]}
      />
    );

    expect(screen.getByText("输入（含缓存）: 1.50K (20.00% 缓存)")).toBeInTheDocument();
  });
});
