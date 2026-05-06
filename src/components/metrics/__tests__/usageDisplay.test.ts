import { describe, expect, test } from "vitest";
import { getTotalInputWithCache } from "../usageDisplay";

describe("getTotalInputWithCache", () => {
  test("returns uncached input plus cache read input", () => {
    expect(getTotalInputWithCache(1200, 300)).toBe(1500);
  });

  test("treats missing values as zero", () => {
    expect(getTotalInputWithCache(undefined, undefined)).toBe(0);
  });
});
