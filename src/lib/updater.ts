const REPO = "Neksand/TrueHalo";

export interface UpdateInfo {
  available: boolean;
  latest: string;
  url: string;
}

// ponytail: naive semver compare, switch to semver lib if prereleases matter
export function isNewer(latest: string, current: string): boolean {
  const norm = (v: string) => v.trim().replace(/^v/i, "").split(".").map((p) => parseInt(p, 10) || 0);
  const a = norm(latest);
  const b = norm(current);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if ((a[i] ?? 0) > (b[i] ?? 0)) return true;
    if ((a[i] ?? 0) < (b[i] ?? 0)) return false;
  }
  return false;
}

const REQUEST_TIMEOUT_MS = 8000;

export async function checkForUpdate(current: string): Promise<UpdateInfo | null> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
      signal: controller.signal,
    });
    if (!res.ok) throw new Error(`GitHub responded with ${res.status}`);
    const j = await res.json();
    const tag = String(j.tag_name ?? j.name ?? "").replace(/^v/i, "");
    if (!tag) return null;
    return {
      available: isNewer(tag, current),
      latest: tag,
      url: String(j.html_url ?? `https://github.com/${REPO}/releases/latest`),
    };
  } finally {
    clearTimeout(timer);
  }
}
