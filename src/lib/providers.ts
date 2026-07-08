export const WIDGET_PROVIDERS = ["OpenAI", "AWS Bedrock", "Azure OpenAI"] as const;

export type WidgetProvider = (typeof WIDGET_PROVIDERS)[number];

export function providerFromWindowLabel(label: string): string | null {
  if (!label.startsWith("widget-")) return null;
  const slug = label.slice("widget-".length);
  if (slug === "openai") return "OpenAI";
  if (slug === "aws-bedrock") return "AWS Bedrock";
  if (slug === "azure-openai") return "Azure OpenAI";
  return null;
}

export function providerSlug(provider: string): string {
  if (provider === "OpenAI") return "openai";
  if (provider === "AWS Bedrock") return "aws-bedrock";
  if (provider === "Azure OpenAI") return "azure-openai";
  return provider.toLowerCase().replace(/\s+/g, "-");
}
