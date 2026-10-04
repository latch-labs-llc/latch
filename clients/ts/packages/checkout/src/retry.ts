/** Retry with backoff for transient transport failures only — deterministic
 * program rejections (Anchor error codes) are rethrown immediately. */
export async function withRetry<T>(
  label: string,
  fn: () => Promise<T>,
  tries = 3
): Promise<T> {
  let last: any;
  for (let i = 0; i < tries; i++) {
    try {
      return await fn();
    } catch (e: any) {
      last = e;
      const msg = String(e?.message ?? e);
      if (msg.includes("Error Code") || msg.includes("custom program error")) throw e;
      if (i < tries - 1) await new Promise((r) => setTimeout(r, 1500 * (i + 1)));
    }
  }
  throw new Error(`${label} failed after ${tries} attempts: ${last?.message ?? last}`);
}
