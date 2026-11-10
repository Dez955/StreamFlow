/** Trim pasted text; returns null unless it looks like an http(s) link. */
export function normalizeUrl(input: string): string | null {
  const text = input.trim();
  return /^https?:\/\/\S+$/i.test(text) ? text : null;
}
