/**
 * A random identifier that works on plain-HTTP pages too.
 *
 * `crypto.randomUUID()` exists only in secure contexts (HTTPS or localhost);
 * the web shell is commonly reached over plain HTTP on a VPN address, where
 * it is undefined. `getRandomValues` is available everywhere; the last
 * fallback covers exotic embeds without Web Crypto at all.
 */
export function randomId(): string {
  const c = globalThis.crypto as Crypto | undefined;
  if (c && typeof c.randomUUID === "function") {
    try {
      return c.randomUUID();
    } catch {
      /* fall through */
    }
  }
  if (c && typeof c.getRandomValues === "function") {
    const bytes = new Uint8Array(16);
    c.getRandomValues(bytes);
    return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  }
  return `${Date.now().toString(16)}-${Math.random().toString(16).slice(2)}-${Math.random()
    .toString(16)
    .slice(2)}`;
}
