export type RequirementBlock = {
  title: string;
  rules: string[];
  steps: string[];
};

function cleanLine(value: string): string {
  return value.replace(/\r/g, "").trim();
}

export function parseRequirementBlocks(raw: string): RequirementBlock[] {
  const blocks: RequirementBlock[] = [];
  let current: RequirementBlock | null = null;

  const pushCurrent = () => {
    if (!current) return;
    const title = current.title.trim();
    if (!title) {
      current = null;
      return;
    }
    blocks.push({
      title,
      rules: [...current.rules],
      steps: [...current.steps]
    });
    current = null;
  };

  for (const line of raw.split(/\n/)) {
    const trimmed = cleanLine(line);
    if (!trimmed) continue;

    if (trimmed.startsWith("## ")) {
      pushCurrent();
      current = {
        title: trimmed.slice(3).trim(),
        rules: [],
        steps: []
      };
      continue;
    }
    if (!current) {
      continue;
    }
    if (trimmed.startsWith("- ")) {
      current.rules.push(trimmed.slice(2).trim());
      continue;
    }
    if (trimmed.startsWith("> ")) {
      current.steps.push(trimmed.slice(2).trim());
      continue;
    }
    if (/^\d+\.\s+/.test(trimmed)) {
      current.steps.push(trimmed.replace(/^\d+\.\s+/, "").trim());
      continue;
    }
  }

  pushCurrent();
  return blocks;
}
