export type JobLockKey = "input" | "output" | "keep" | "add" | "forbid";

export type JobLockSection = {
  key: JobLockKey;
  title: string;
  items: string[];
};

export type JobCheckboxItem = {
  text: string;
  checked: boolean;
};

export type JobProcessSnapshot = {
  locks: JobLockSection[];
  problems: JobCheckboxItem[];
  verify: JobCheckboxItem[];
  evidence: JobCheckboxItem[];
};

function collectSectionBody(raw: string, heading: string): string[] {
  const lines = raw.split(/\n/);
  const target = heading.trim().toLowerCase();
  const collected: string[] = [];
  let active = false;
  let depth = 0;

  for (const rawLine of lines) {
    const line = rawLine.replace(/\r/g, "");
    const trimmed = line.trim();
    const headingMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
    if (headingMatch) {
      const nextDepth = headingMatch[1].length;
      const title = headingMatch[2].trim().toLowerCase();
      if (!active) {
        if (title === target) {
          active = true;
          depth = nextDepth;
        }
        continue;
      }
      if (nextDepth <= depth) {
        break;
      }
      continue;
    }
    if (active) {
      collected.push(trimmed);
    }
  }

  return collected;
}

function parseBulletItems(lines: string[]): string[] {
  return lines
    .filter((line) => /^- /.test(line) && !/^- \[[xX ]\]/.test(line))
    .map((line) => line.slice(2).trim())
    .filter(Boolean);
}

function parseCheckboxItems(lines: string[]): JobCheckboxItem[] {
  return lines
    .map((line) => {
      const match = line.match(/^- \[( |x|X)\]\s+(.*)$/);
      if (!match) return null;
      return {
        checked: match[1].toLowerCase() === "x",
        text: match[2].trim()
      };
    })
    .filter((item): item is JobCheckboxItem => item !== null && item.text.length > 0);
}

export function parseJobProcessSnapshot(raw: string): JobProcessSnapshot {
  return {
    locks: [
      { key: "input", title: "input", items: parseBulletItems(collectSectionBody(raw, "input")) },
      { key: "output", title: "output", items: parseBulletItems(collectSectionBody(raw, "output")) },
      { key: "keep", title: "keep", items: parseBulletItems(collectSectionBody(raw, "keep")) },
      { key: "add", title: "add", items: parseBulletItems(collectSectionBody(raw, "add")) },
      { key: "forbid", title: "forbid", items: parseBulletItems(collectSectionBody(raw, "forbid")) }
    ],
    problems: parseCheckboxItems(collectSectionBody(raw, "problems")),
    verify: parseCheckboxItems(collectSectionBody(raw, "verify")),
    evidence: parseCheckboxItems(collectSectionBody(raw, "check evidence"))
  };
}
