const PROVIDER_CHART_DATA_IDS = new Set(["claude", "codex", "openai"]);

export function providerSupportsChartData(providerId: string): boolean {
  return PROVIDER_CHART_DATA_IDS.has(providerId.toLowerCase());
}
