import { describe, it, expect } from "vitest";
import { ALL_LOCALE_KEYS } from "./keys";

describe("ALL_LOCALE_KEYS", () => {
  it("does not duplicate canonical language catalog entries", () => {
    expect(ALL_LOCALE_KEYS).not.toContain("LanguageEnglishOption");
    expect(ALL_LOCALE_KEYS).not.toContain("LanguageChineseOption");
    expect(ALL_LOCALE_KEYS).not.toContain("LanguageJapaneseOption");
    expect(ALL_LOCALE_KEYS).not.toContain("LanguageKoreanOption");
    expect(ALL_LOCALE_KEYS).not.toContain("LanguageSpanishOption");
  });
});
