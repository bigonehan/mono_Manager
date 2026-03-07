import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import YAML from "yaml";

export type ProjectRecord = {
  id: string;
  name: string;
  path: string;
  description: string;
  created_at: string;
  updated_at: string;
  selected: boolean;
  project_type: "story" | "movie" | "code" | "mono";
  state?: ProjectState;
};

type ProjectRegistry = {
  recentActivepane?: string;
  projects: ProjectRecord[];
};

type DraftsListDoc = {
  features?: string[];
  planned?: string[];
  worked?: string[];
  complete?: string[];
  failed?: string[];
  planned_items?: Array<{ name?: string; value?: string }>;
};

type DraftsDoc = {
  planned?: string[];
  worked?: string[];
  complete?: string[];
  failed?: string[];
};

export type ProjectState = "init" | "basic" | "work" | "wait";
export type BrowseEntry = { name: string; path: string; hasProjectMeta: boolean };
export type MonorepoPackage = {
  id: string;
  name: string;
  path: string;
  kind: "app" | "feature" | "template";
};

export type ProjectDetail = {
  id: string;
  name: string;
  description: string;
  path: string;
  memo: string;
  project_type: "story" | "movie" | "code" | "mono";
  spec: string;
  goal: string;
  rules: string[];
  constraints: string[];
  features: string[];
  domains: Array<{ name: string; description: string; features: string[] }>;
  planned: string[];
  plannedDisplay: string[];
  generated: string[];
  state: ProjectState;
  hasDraftsYaml: boolean;
};

export function repoRoot(): string {
  return process.env.ORC_ROOT ?? path.resolve(process.cwd(), "..", "..");
}

function browseRoot(): string {
  return process.env.ORC_BROWSE_ROOT ?? "/home/tree";
}

function monorepoRoot(): string {
  if (process.env.ORC_MONOREPO_ROOT) return process.env.ORC_MONOREPO_ROOT;
  const home = process.env.HOME ?? "/home/tree";
  return path.join(home, "home");
}

function registryPath(): string {
  return path.join(repoRoot(), "configs", "project.yaml");
}

function nowUnix(): string {
  return Math.floor(Date.now() / 1000).toString();
}

function randomId(length = 4): string {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789";
  let out = "";
  for (let i = 0; i < length; i += 1) {
    out += chars[Math.floor(Math.random() * chars.length)];
  }
  return out;
}

function normalizeProjectType(raw: unknown): ProjectRecord["project_type"] {
  if (raw === "story" || raw === "movie" || raw === "mono") {
    return raw;
  }
  return "code";
}

export function loadRegistry(): ProjectRegistry {
  const file = registryPath();
  if (!fs.existsSync(file)) {
    return { recentActivepane: "", projects: [] };
  }
  const raw = fs.readFileSync(file, "utf8");
  const parsed = YAML.parse(raw) ?? {};
  const projects = Array.isArray(parsed.projects) ? parsed.projects : [];
  return {
    recentActivepane: parsed.recentActivepane ?? "",
    projects: projects.map((project: Record<string, unknown>) => ({
      id: String(project.id ?? ""),
      name: String(project.name ?? ""),
      path: String(project.path ?? ""),
      description: String(project.description ?? ""),
      created_at: String(project.created_at ?? nowUnix()),
      updated_at: String(project.updated_at ?? nowUnix()),
      selected: Boolean(project.selected),
      project_type: normalizeProjectType(project.project_type)
    }))
  };
}

export function saveRegistry(registry: ProjectRegistry): void {
  const file = registryPath();
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, YAML.stringify(registry), "utf8");
}

function projectMetaDir(projectPath: string): string {
  return path.join(projectPath, ".project");
}

function projectMdPath(projectPath: string): string {
  return path.join(projectMetaDir(projectPath), "project.md");
}

function draftsListPath(projectPath: string): string {
  return path.join(projectMetaDir(projectPath), "drafts_list.yaml");
}

function draftsYamlPath(projectPath: string): string {
  return path.join(projectMetaDir(projectPath), "drafts.yaml");
}

function memoPath(projectPath: string): string {
  return path.join(projectMetaDir(projectPath), "memo.md");
}

function ensureProjectFiles(project: ProjectRecord): void {
  fs.mkdirSync(project.path, { recursive: true });
  fs.mkdirSync(projectMetaDir(project.path), { recursive: true });

  const pmd = projectMdPath(project.path);
  if (!fs.existsSync(pmd)) {
    fs.writeFileSync(
      pmd,
      `# info\nname: ${project.name}\ndescription: ${project.description}\nspec: auto\ngoal: init\n\n# rules\n- \n\n# constraints\n- \n\n# features\n- \n`,
      "utf8"
    );
  }

  const dlist = draftsListPath(project.path);
  if (!fs.existsSync(dlist)) {
    fs.writeFileSync(
      dlist,
      YAML.stringify({ features: [], planned: [], planned_items: [], worked: [], complete: [], failed: [] }),
      "utf8"
    );
  }

  const memo = memoPath(project.path);
  if (!fs.existsSync(memo)) {
    fs.writeFileSync(memo, "", "utf8");
  }
}

function listImmediateDirs(basePath: string): string[] {
  if (!fs.existsSync(basePath) || !fs.statSync(basePath).isDirectory()) return [];
  return fs
    .readdirSync(basePath, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && !entry.name.startsWith(".") && entry.name !== "node_modules")
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b));
}

function inferFrameworkLabel(projectPath: string): string {
  if (fs.existsSync(path.join(projectPath, "next.config.js")) || fs.existsSync(path.join(projectPath, "next.config.ts"))) {
    return "next";
  }
  if (fs.existsSync(path.join(projectPath, "astro.config.mjs")) || fs.existsSync(path.join(projectPath, "astro.config.ts"))) {
    return "astro";
  }
  if (fs.existsSync(path.join(projectPath, "app.json"))) {
    return "expo";
  }
  return "app";
}

function collectMonorepoPackages(root: string): Array<{ kind: MonorepoPackage["kind"]; path: string; name: string }> {
  const roots: Array<{ kind: MonorepoPackage["kind"]; dirs: string[]; depth: "single" | "double" }> = [
    { kind: "app", dirs: ["apps", "app"], depth: "double" },
    { kind: "feature", dirs: ["packages/features", "features", "feature"], depth: "single" },
    { kind: "template", dirs: ["template", "templates"], depth: "double" }
  ];
  const seen = new Set<string>();
  const out: Array<{ kind: MonorepoPackage["kind"]; path: string; name: string }> = [];
  for (const bucket of roots) {
    for (const rel of bucket.dirs) {
      const parent = path.join(root, rel);
      for (const child of listImmediateDirs(parent)) {
        const firstPath = path.join(parent, child);
        if (bucket.depth === "double") {
          const nested = listImmediateDirs(firstPath);
          let addedNested = false;
          for (const grandchild of nested) {
            const secondPath = path.join(firstPath, grandchild);
            if (seen.has(secondPath)) continue;
            seen.add(secondPath);
            out.push({
              kind: bucket.kind,
              path: secondPath,
              name: `${child}/${grandchild}`
            });
            addedNested = true;
          }
          if (!addedNested) {
            const fallbackLabel = `${child}/${inferFrameworkLabel(firstPath)}`;
            if (seen.has(firstPath)) continue;
            seen.add(firstPath);
            out.push({
              kind: bucket.kind,
              path: firstPath,
              name: fallbackLabel
            });
          }
          continue;
        }
        if (seen.has(firstPath)) continue;
        seen.add(firstPath);
        out.push({
          kind: bucket.kind,
          path: firstPath,
          name: child
        });
      }
    }
  }
  out.sort((a, b) => a.path.localeCompare(b.path));
  return out;
}

function collectMonorepoDomains(root: string): string[] {
  return listImmediateDirs(path.join(root, "packages", "domains"));
}

function isInside(parent: string, child: string): boolean {
  const normalizedParent = path.resolve(parent);
  const normalizedChild = path.resolve(child);
  return normalizedChild === normalizedParent || normalizedChild.startsWith(`${normalizedParent}${path.sep}`);
}

function isMonorepoManagedPath(projectPath: string, root: string): boolean {
  const monitored = [
    path.join(root, "apps"),
    path.join(root, "app"),
    path.join(root, "packages", "features"),
    path.join(root, "features"),
    path.join(root, "feature"),
    path.join(root, "template"),
    path.join(root, "templates")
  ];
  return monitored.some((base) => isInside(base, projectPath));
}

function monorepoDomainDetails(root: string): Array<{ name: string; description: string; features: string[] }> {
  return collectMonorepoDomains(root).map((name) => ({ name, description: "", features: [] }));
}

export function syncMonorepoProjects(): {
  root: string;
  domains: string[];
  packages: MonorepoPackage[];
  created: number;
  updated: number;
} {
  const root = monorepoRoot();
  const domains = collectMonorepoDomains(root);
  const packageRows = collectMonorepoPackages(root);
  const registry = loadRegistry();
  const now = nowUnix();
  registry.projects = registry.projects.filter(
    (project) => !(project.project_type === "code" && isMonorepoManagedPath(project.path, root))
  );
  let created = 0;
  let updated = 0;
  for (const row of packageRows) {
    const existing = registry.projects.find((p) => p.path === row.path);
    if (existing) {
      const nextDescription = `monorepo ${row.kind} package`;
      if (
        existing.name !== row.name ||
        existing.description !== nextDescription ||
        existing.project_type !== "mono"
      ) {
        existing.name = row.name;
        existing.description = nextDescription;
        existing.project_type = "mono";
        existing.updated_at = now;
        updated += 1;
      }
      ensureProjectFiles(existing);
      continue;
    }
    const record: ProjectRecord = {
      id: randomId(),
      name: row.name,
      path: row.path,
      description: `monorepo ${row.kind} package`,
      created_at: now,
      updated_at: now,
      selected: false,
      project_type: "mono"
    };
    registry.projects.push(record);
    ensureProjectFiles(record);
    created += 1;
  }
  saveRegistry(registry);
  const projects = listProjects();
  const packages: MonorepoPackage[] = packageRows
    .map((row) => {
      const project = projects.find((p) => p.path === row.path);
      if (!project) return null;
      return {
        id: project.id,
        name: project.name,
        path: project.path,
        kind: row.kind
      };
    })
    .filter((v): v is MonorepoPackage => Boolean(v));
  return { root, domains, packages, created, updated };
}

export function createProject(input: {
  name: string;
  description: string;
  projectPath: string;
  spec?: string;
  projectType?: ProjectRecord["project_type"];
}): ProjectRecord {
  const registry = loadRegistry();
  const now = nowUnix();
  const normalizedPath = input.projectPath.trim();
  const existingByPath = registry.projects.find((p) => p.path === normalizedPath);
  if (existingByPath) {
    return updateProject(existingByPath.id, {
      name: input.name,
      description: input.description,
      projectPath: normalizedPath,
      selected: true
    });
  }
  const existing = registry.projects.find((p) => p.name === input.name);
  if (existing) {
    throw new Error(`project already exists: ${input.name}`);
  }
  const id = randomId();
  const record: ProjectRecord = {
    id,
    name: input.name,
    path: normalizedPath,
    description: input.description,
    created_at: now,
    updated_at: now,
    selected: true,
    project_type: normalizeProjectType(input.projectType)
  };
  registry.projects = registry.projects.map((p) => ({ ...p, selected: false }));
  registry.projects.push(record);
  registry.recentActivepane = id;
  saveRegistry(registry);
  ensureProjectFiles(record);

  if (input.spec && input.spec.trim().length > 0) {
    const detail = loadProjectDetail(id);
    saveProjectInfo(id, {
      name: detail.name,
      description: detail.description,
      spec: input.spec,
      goal: detail.goal
    });
  }

  return record;
}

export function loadProjectFromPath(input: {
  projectPath: string;
  createIfMissing?: boolean;
  projectType?: ProjectRecord["project_type"];
}): { project: ProjectRecord; createdProjectMeta: boolean } {
  const projectPath = input.projectPath.trim();
  if (projectPath.length === 0) {
    throw new Error("project path is required");
  }
  if (!fs.existsSync(projectPath)) {
    throw new Error(`path not found: ${projectPath}`);
  }
  if (!fs.statSync(projectPath).isDirectory()) {
    throw new Error(`path is not directory: ${projectPath}`);
  }

  const meta = projectMetaDir(projectPath);
  const hasMeta = fs.existsSync(meta);
  if (!hasMeta && !input.createIfMissing) {
    throw new Error("PROJECT_META_MISSING");
  }

  const baseName = path.basename(projectPath) || "project";
  let parsedName = baseName;
  let parsedDescription = "loaded project";
  if (hasMeta && fs.existsSync(projectMdPath(projectPath))) {
    const parsed = readProjectMdAttributes(fs.readFileSync(projectMdPath(projectPath), "utf8"));
    parsedName = parsed.name || baseName;
    parsedDescription = parsed.description || parsedDescription;
  }

  const registry = loadRegistry();
  const now = nowUnix();
  let record = registry.projects.find((p) => p.path === projectPath);
  if (record) {
    record = {
      ...record,
      name: parsedName,
      description: parsedDescription,
      selected: true,
      updated_at: now
    };
    registry.projects = registry.projects.map((p) =>
      p.id === record?.id ? record! : { ...p, selected: false }
    );
  } else {
    const id = randomId();
    record = {
      id,
      name: parsedName,
      path: projectPath,
      description: parsedDescription,
      created_at: now,
      updated_at: now,
      selected: true,
      project_type: normalizeProjectType(input.projectType)
    };
    registry.projects = registry.projects.map((p) => ({ ...p, selected: false }));
    registry.projects.push(record);
  }
  registry.recentActivepane = record.id;
  saveRegistry(registry);
  ensureProjectFiles(record);
  return { project: record, createdProjectMeta: !hasMeta };
}

export function updateProject(
  id: string,
  input: Partial<{ name: string; description: string; projectPath: string; selected: boolean }>
): ProjectRecord {
  const registry = loadRegistry();
  const idx = registry.projects.findIndex((p) => p.id === id);
  if (idx < 0) {
    throw new Error(`project not found: ${id}`);
  }
  const target = registry.projects[idx];
  const updated: ProjectRecord = {
    ...target,
    name: input.name?.trim() || target.name,
    description: input.description?.trim() || target.description,
    path: input.projectPath?.trim() || target.path,
    selected: input.selected ?? target.selected,
    updated_at: nowUnix()
  };

  if (updated.selected) {
    registry.projects = registry.projects.map((p) => ({ ...p, selected: p.id === id }));
    registry.recentActivepane = id;
  }
  registry.projects[idx] = updated;
  saveRegistry(registry);
  ensureProjectFiles(updated);
  return updated;
}

export function deleteProject(id: string): void {
  const registry = loadRegistry();
  const target = registry.projects.find((p) => p.id === id);
  if (!target) {
    throw new Error(`project not found: ${id}`);
  }
  registry.projects = registry.projects.filter((p) => p.id !== id);
  if (registry.recentActivepane === id) {
    registry.recentActivepane = registry.projects[0]?.id ?? "";
  }
  if (registry.projects.length > 0 && !registry.projects.some((p) => p.selected)) {
    registry.projects[0].selected = true;
  }
  saveRegistry(registry);

  const meta = projectMetaDir(target.path);
  if (fs.existsSync(meta)) {
    fs.rmSync(meta, { recursive: true, force: true });
  }
}

// Parses project.md once and returns all key sections used by web detail panes.
function readProjectMdAttributes(raw: string): {
  name: string;
  description: string;
  spec: string;
  goal: string;
  rules: string[];
  constraints: string[];
  features: string[];
  domains: Array<{ name: string; description: string; features: string[] }>;
} {
  type DomainRow = { name: string; description: string; features: string[] };
  const out = {
    name: "",
    description: "",
    spec: "",
    goal: "",
    rules: [] as string[],
    constraints: [] as string[],
    features: [] as string[],
    domains: [] as DomainRow[]
  };

  let section: "rules" | "constraints" | "features" | "none" = "none";
  let inDomains = false;
  let activeDomain: DomainRow | null = null;
  let domainSubsection = "";
  for (const line of raw.split(/\r?\n/)) {
    const t = line.trim();
    if (t.toLowerCase() === "# rules") {
      section = "rules";
      continue;
    }
    if (t.toLowerCase() === "# constraints") {
      section = "constraints";
      continue;
    }
    if (t.toLowerCase() === "# features") {
      section = "features";
      continue;
    }
    if (t.toLowerCase() === "# domains") {
      section = "none";
      inDomains = true;
      continue;
    }
    if (t.startsWith("#")) {
      section = "none";
      if (inDomains && /^#\s+/i.test(t) && t.toLowerCase() !== "# domains") {
        inDomains = false;
        activeDomain = null;
        domainSubsection = "";
      }
    }

    if (inDomains && /^##\s+/i.test(t)) {
      const heading = t.replace(/^##\s+/i, "").trim().replace(/`/g, "");
      if (heading.length > 0) {
        const [namePart, descPart = ""] = heading.split(/\s*[|:]\s*/, 2);
        const name = namePart.trim();
        if (name.length > 0 && name.toLowerCase() !== "name") {
          activeDomain = {
            name,
            description: descPart.trim(),
            features: []
          };
          out.domains.push(activeDomain);
          domainSubsection = "";
        }
      }
      continue;
    }

    if (inDomains) {
      if (/^###\s+/i.test(t)) {
        domainSubsection = t.replace(/^###\s+/i, "").trim().toLowerCase();
        continue;
      }
      if (activeDomain && t.startsWith("- ")) {
        const item = t.slice(2).trim();
        if (item.length === 0) continue;
        if (domainSubsection === "action" || domainSubsection === "feature" || domainSubsection === "features") {
          if (!activeDomain.features.includes(item)) {
            activeDomain.features.push(item);
          }
        } else if ((domainSubsection === "rules" || domainSubsection === "description") && !activeDomain.description) {
          activeDomain.description = item;
        }
      }
      continue;
    }

    if (section === "rules" && t.startsWith("- ")) {
      out.rules.push(t.slice(2).trim());
      continue;
    }
    if (section === "constraints" && t.startsWith("- ")) {
      out.constraints.push(t.slice(2).trim());
      continue;
    }
    if (section === "features" && t.startsWith("- ")) {
      out.features.push(t.slice(2).trim());
      continue;
    }

    const pair = t.split(":");
    if (pair.length < 2) {
      continue;
    }
    const key = pair[0].trim().toLowerCase();
    const value = pair.slice(1).join(":").trim();
    if (key === "name") out.name = value;
    if (key === "description") out.description = value;
    if (key === "spec") out.spec = value;
    if (key === "goal") out.goal = value;
  }

  return out;
}

function writeProjectMd(projectPath: string, doc: {
  name: string;
  description: string;
  spec: string;
  goal: string;
  rules: string[];
  constraints: string[];
  features: string[];
}): void {
  const content = [
    "# info",
    `name: ${doc.name}`,
    `description: ${doc.description}`,
    `spec: ${doc.spec}`,
    `goal: ${doc.goal}`,
    "",
    "# rules",
    ...(doc.rules.length > 0 ? doc.rules : [""]).map((v) => `- ${v}`),
    "",
    "# constraints",
    ...(doc.constraints.length > 0 ? doc.constraints : [""]).map((v) => `- ${v}`),
    "",
    "# features",
    ...(doc.features.length > 0 ? doc.features : [""]).map((v) => `- ${v}`),
    ""
  ].join("\n");

  fs.writeFileSync(projectMdPath(projectPath), content, "utf8");
}

function loadDraftsList(projectPath: string): DraftsListDoc {
  const file = draftsListPath(projectPath);
  if (!fs.existsSync(file)) {
    return { features: [], planned: [], planned_items: [] };
  }
  return (YAML.parse(fs.readFileSync(file, "utf8")) as DraftsListDoc) ?? {};
}

function saveDraftsList(projectPath: string, doc: DraftsListDoc): void {
  fs.writeFileSync(draftsListPath(projectPath), YAML.stringify(doc), "utf8");
}

function loadDraftsDoc(projectPath: string): DraftsDoc {
  const file = draftsYamlPath(projectPath);
  if (!fs.existsSync(file)) {
    return {};
  }
  return (YAML.parse(fs.readFileSync(file, "utf8")) as DraftsDoc) ?? {};
}

function listCount(values: unknown): number {
  return Array.isArray(values) ? values.filter(Boolean).length : 0;
}

function isBootstrapCompleted(projectPath: string): boolean {
  if (!fs.existsSync(projectPath)) {
    return false;
  }
  const entries = fs.readdirSync(projectPath, { withFileTypes: true });
  return entries.some((entry) => ![".project", ".git", ".jj"].includes(entry.name));
}

function resolveProjectState(project: ProjectRecord): ProjectState {
  const pmdPath = projectMdPath(project.path);
  if (!fs.existsSync(pmdPath)) {
    return "init";
  }

  const drafts = loadDraftsDoc(project.path);
  const plannedCount = listCount(drafts.planned);
  const workedCount = listCount(drafts.worked);
  const completeCount = listCount(drafts.complete);
  if (plannedCount > 0 || workedCount > 0) {
    return "work";
  }
  if (completeCount > 0 && plannedCount === 0 && workedCount === 0) {
    return "wait";
  }
  if (!isBootstrapCompleted(project.path)) {
    return "init";
  }
  return "basic";
}

export function listProjects(): ProjectRecord[] {
  const registry = loadRegistry();
  return registry.projects.map((project) => ({
    ...project,
    state: resolveProjectState(project)
  }));
}

function collectGenerated(projectPath: string): string[] {
  const featureRoot = path.join(projectPath, ".project", "feature");
  if (!fs.existsSync(featureRoot)) {
    return [];
  }
  const out: string[] = [];
  for (const dirent of fs.readdirSync(featureRoot, { withFileTypes: true })) {
    if (!dirent.isDirectory()) {
      continue;
    }
    const dir = path.join(featureRoot, dirent.name);
    const hasDraft = fs.existsSync(path.join(dir, "drafts.yaml")) || fs.existsSync(path.join(dir, "tasks.yaml"));
    if (hasDraft) {
      out.push(dirent.name);
    }
  }
  return out.sort();
}

export function loadProjectDetail(id: string): ProjectDetail {
  const registry = loadRegistry();
  const project = registry.projects.find((p) => p.id === id);
  if (!project) {
    throw new Error(`project not found: ${id}`);
  }
  ensureProjectFiles(project);
  const parsed = readProjectMdAttributes(fs.readFileSync(projectMdPath(project.path), "utf8"));
  const drafts = loadDraftsList(project.path);
  const hasDraftsYaml = fs.existsSync(draftsYamlPath(project.path));
  const planned = Array.isArray(drafts.planned) ? drafts.planned : [];
  const plannedItems = Array.isArray(drafts.planned_items) ? drafts.planned_items : [];
  const memo = fs.existsSync(memoPath(project.path)) ? fs.readFileSync(memoPath(project.path), "utf8") : "";

  const root = monorepoRoot();
  const domains = isMonorepoManagedPath(project.path, root) ? monorepoDomainDetails(root) : parsed.domains;
  return {
    id: project.id,
    name: parsed.name || project.name,
    description: parsed.description || project.description,
    path: project.path,
    memo,
    project_type: project.project_type,
    spec: parsed.spec,
    goal: parsed.goal,
    rules: parsed.rules.filter((v) => v.length > 0),
    constraints: parsed.constraints.filter((v) => v.length > 0),
    features: parsed.features.filter((v) => v.length > 0),
    domains,
    planned,
    plannedDisplay: planned.map((key) => {
      const row = plannedItems.find((item) => item.name === key);
      return row?.value?.trim() || key;
    }),
    generated: collectGenerated(project.path),
    state: resolveProjectState(project),
    hasDraftsYaml
  };
}

export function saveProjectMemo(id: string, memo: string): ProjectDetail {
  const detail = loadProjectDetail(id);
  fs.writeFileSync(memoPath(detail.path), memo, "utf8");
  return loadProjectDetail(id);
}

export function saveProjectInfo(id: string, input: {
  name: string;
  description: string;
  spec: string;
  goal: string;
}): ProjectDetail {
  const updated = updateProject(id, {
    name: input.name,
    description: input.description
  });
  const current = loadProjectDetail(id);
  writeProjectMd(updated.path, {
    name: input.name,
    description: input.description,
    spec: input.spec,
    goal: input.goal,
    rules: current.rules,
    constraints: current.constraints,
    features: current.features
  });
  return loadProjectDetail(id);
}

export function saveLists(id: string, input: {
  rules: string[];
  constraints: string[];
  features: string[];
}): ProjectDetail {
  const current = loadProjectDetail(id);
  writeProjectMd(current.path, {
    name: current.name,
    description: current.description,
    spec: current.spec,
    goal: current.goal,
    rules: input.rules,
    constraints: input.constraints,
    features: input.features
  });

  const drafts = loadDraftsList(current.path);
  drafts.features = input.features;
  saveDraftsList(current.path, drafts);
  return loadProjectDetail(id);
}

export function runOrcAction(id: string, action: string, payload?: string): string {
  const detail = loadProjectDetail(id);
  const argsMap: Record<string, string[]> = {
    create_draft: ["create_code_draft"],
    add_draft: payload?.trim().length ? ["add_code_draft", "-m", payload] : ["add_code_draft", "-a"],
    impl_draft: ["impl_code_draft"],
    check_code: ["check_code_draft", "-a"],
    check_draft: ["check_draft"]
  };
  const args = argsMap[action];
  if (!args) {
    throw new Error(`unsupported action: ${action}`);
  }

  const orcBin = process.env.ORC_BIN ?? "orc";
  const result = spawnSync(orcBin, args, {
    cwd: repoRoot(),
    encoding: "utf8"
  });

  if (result.status !== 0) {
    const stderr = (result.stderr || "").trim();
    throw new Error(stderr.length > 0 ? stderr : `command failed: ${orcBin} ${args.join(" ")}`);
  }

  return `action=${action} project=${detail.name} output=${(result.stdout || "").trim()}`;
}

export function browseProjectDirs(inputPath: string): {
  currentPath: string;
  parentPath: string | null;
  entries: BrowseEntry[];
} {
  const root = path.resolve(browseRoot());
  const requested = inputPath.trim().length > 0 ? path.resolve(inputPath.trim()) : root;
  const currentPath = requested.startsWith(root) ? requested : root;
  if (!fs.existsSync(currentPath) || !fs.statSync(currentPath).isDirectory()) {
    throw new Error(`directory not found: ${currentPath}`);
  }

  const parentPath =
    currentPath !== root && path.dirname(currentPath).startsWith(root)
      ? path.dirname(currentPath)
      : null;

  const entries = fs
    .readdirSync(currentPath, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => {
      const nextPath = path.join(currentPath, entry.name);
      return {
        name: entry.name,
        path: nextPath,
        hasProjectMeta: fs.existsSync(path.join(nextPath, ".project"))
      };
    })
    .sort((a, b) => a.name.localeCompare(b.name, "en"));

  return { currentPath, parentPath, entries };
}
