import fs from "node:fs";
import path from "node:path";
import net from "node:net";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
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
  current_job?: string;
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
  draft?: Array<Record<string, unknown>>;
  planned?: string[];
  worked?: string[];
  complete?: string[];
  failed?: string[];
};

type PlanDoc = {
  drafts?: {
    planned?: string[];
    worked?: string[];
    complete?: string[];
    failed?: string[];
  };
};

export type ProjectState = "init" | "basic" | "work" | "wait" | "review" | "run" | "build";
export type ProfileType = "code" | "mono" | "write" | "video";

const runtimeLogsByProject = new Map<string, string[]>();
const runProcessesByProject = new Map<string, ChildProcess>();
const runPortsByProject = new Map<string, number>();
const runUrlsByProject = new Map<string, string>();
const buildProcessesByProject = new Map<string, ChildProcess>();
const buildCurrentJobByProject = new Map<string, string>();
const buildCompletionByProject = new Map<string, string>();
const DEV_PORT_MIN = 4300;
const DEV_PORT_MAX = 4999;
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
  current_job?: string;
  hasDraftsYaml: boolean;
  dev_server_url?: string;
  draftsYamlRaw?: string;
  inputMdRaw?: string;
  inputTitles: string[];
  inputItems: Array<{ title: string; rule: string; step: string }>;
  draftItems: Array<Record<string, unknown>>;
  draftsYamlItems: Array<{
    name: string;
    status: "work" | "wait" | "complete";
    draft: Record<string, unknown>;
  }>;
};

export function repoRoot(): string {
  return process.env.ORC_ROOT ?? path.resolve(process.cwd(), "..", "..");
}

function browseRoot(): string {
  return process.env.ORC_BROWSE_ROOT ?? "/home/tree";
}

function resolveOrcCommandArgs(args: string[]): { bin: string; args: string[] } {
  const envBin = (process.env.ORC_BIN ?? "").trim();
  if (envBin.length > 0) {
    return { bin: envBin, args };
  }
  const root = repoRoot();
  const legacyAssets = path.join(root, "assets", "code");
  const presetsAssets = path.join(root, "assets", "presets", "code");
  if (!fs.existsSync(legacyAssets) && fs.existsSync(presetsAssets)) {
    return {
      bin: "cargo",
      args: ["run", "--quiet", "--manifest-path", path.join(root, "Cargo.toml"), "--", ...args]
    };
  }
  return { bin: "orc", args };
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

function normalizeProfileType(raw: unknown): ProfileType {
  if (raw === "mono" || raw === "write" || raw === "video") {
    return raw;
  }
  return "code";
}

function profileTypeFromProjectType(projectType: ProjectRecord["project_type"]): ProfileType {
  if (projectType === "mono") return "mono";
  if (projectType === "story") return "write";
  if (projectType === "movie") return "video";
  return "code";
}

function profileAssetsDir(profile: ProfileType): string {
  return path.join(repoRoot(), "assets", "presets", profile);
}

function safeReadFile(filePath: string): string {
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) return "";
  return fs.readFileSync(filePath, "utf8");
}

function listFilesWithContent(dir: string): Array<{ name: string; path: string; content: string }> {
  if (!fs.existsSync(dir) || !fs.statSync(dir).isDirectory()) {
    return [];
  }
  const files = fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b));
  return files.map((name) => {
    const full = path.join(dir, name);
    return {
      name,
      path: full,
      content: safeReadFile(full)
    };
  });
}

function safeAssetFileName(name: string): string {
  const trimmed = name.trim();
  if (!trimmed || trimmed.includes("/") || trimmed.includes("\\")) {
    throw new Error(`invalid file name: ${name}`);
  }
  return trimmed;
}

function resolveProfileAssetFile(profile: ProfileType, section: "prompts" | "templates", name: string): string {
  const fileName = safeAssetFileName(name);
  return path.join(profileAssetsDir(profile), section, fileName);
}

export function loadProfileAssets(rawType: unknown): {
  profile: ProfileType;
  prompts: Array<{ name: string; path: string; content: string }>;
  templates: Array<{ name: string; path: string; content: string }>;
} {
  const profile = normalizeProfileType(rawType);
  const root = profileAssetsDir(profile);
  return {
    profile,
    prompts: listFilesWithContent(path.join(root, "prompts")),
    templates: listFilesWithContent(path.join(root, "templates"))
  };
}

export function loadDraftFormTemplate(rawType: unknown): {
  profile: ProfileType;
  modalName: string;
  raw: string;
  fields: Array<{ key: string; value: string }>;
} {
  const profile = normalizeProfileType(rawType);
  const draftTemplatePath = path.join(profileAssetsDir(profile), "templates", "draft.yaml");
  const raw = safeReadFile(draftTemplatePath);
  if (!raw.trim()) {
    return { profile, modalName: `edit_${profile}_drafts`, raw: "", fields: [] };
  }
  const parsed = YAML.parse(raw);
  const row = Array.isArray(parsed) && parsed.length > 0 ? parsed[0] : parsed;
  const fields: Array<{ key: string; value: string }> = [];
  if (row && typeof row === "object" && !Array.isArray(row)) {
    for (const [key, value] of Object.entries(row as Record<string, unknown>)) {
      let normalized = "";
      if (typeof value === "string") {
        normalized = value;
      } else if (Array.isArray(value)) {
        normalized = value.join(", ");
      } else if (value && typeof value === "object") {
        normalized = JSON.stringify(value);
      } else if (value != null) {
        normalized = String(value);
      }
      fields.push({ key, value: normalized });
    }
  }
  return { profile, modalName: `edit_${profile}_drafts`, raw, fields };
}

export function updateProfileAssetFile(input: {
  type: unknown;
  section: unknown;
  name: unknown;
  content: unknown;
}): { profile: ProfileType; output: string } {
  const profile = normalizeProfileType(input.type);
  const section = input.section === "prompts" ? "prompts" : "templates";
  const name = String(input.name ?? "");
  const content = String(input.content ?? "");
  const filePath = resolveProfileAssetFile(profile, section, name);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");

  const prompt = `${filePath}을 수정했으니 소스코드를 보고 관련된 모든 항목을 갱신해달라`;
  const llmBin = process.env.WEB_LLM_BIN ?? "codex";
  const result = spawnSync(llmBin, ["exec", prompt], {
    cwd: repoRoot(),
    encoding: "utf8"
  });
  if (result.status !== 0) {
    const stderr = (result.stderr || "").trim();
    throw new Error(stderr || `${llmBin} exec failed`);
  }
  const output = (result.stdout || "").trim();
  return { profile, output: output || `updated ${section}/${name}` };
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

function planYamlPath(projectPath: string): string {
  return path.join(projectMetaDir(projectPath), "plan.yaml");
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

export function reorderProjects(orderedIds: string[]): ProjectRecord[] {
  const registry = loadRegistry();
  if (!Array.isArray(orderedIds) || orderedIds.length === 0) {
    return listProjects();
  }
  const byId = new Map(registry.projects.map((project) => [project.id, project]));
  const used = new Set<string>();
  const reordered: ProjectRecord[] = [];
  for (const id of orderedIds) {
    if (used.has(id)) continue;
    const project = byId.get(id);
    if (!project) continue;
    reordered.push(project);
    used.add(id);
  }
  for (const project of registry.projects) {
    if (!used.has(project.id)) {
      reordered.push(project);
    }
  }
  registry.projects = reordered;
  saveRegistry(registry);
  return listProjects();
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

function normalizeFeatureName(value: string): string {
  return String(value ?? "")
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "_")
    .replace(/^_+|_+$/g, "");
}

function dedupNormalized(values: string[]): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const value of values) {
    const normalized = normalizeFeatureName(value);
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    out.push(normalized);
  }
  return out;
}

function normalizeDraftStateDoc(doc: DraftsDoc): DraftsDoc {
  const planned = dedupNormalized(Array.isArray(doc.planned) ? doc.planned : []);
  const worked = dedupNormalized(Array.isArray(doc.worked) ? doc.worked : []).filter((name) => !planned.includes(name));
  const complete = dedupNormalized(Array.isArray(doc.complete) ? doc.complete : []).filter(
    (name) => !planned.includes(name) && !worked.includes(name)
  );
  return {
    ...doc,
    planned,
    worked,
    complete,
    failed: Array.isArray(doc.failed) ? doc.failed : []
  };
}

function reconcileDraftCompletionFromProjectFeatures(projectPath: string): void {
  const projectMd = projectMdPath(projectPath);
  if (!fs.existsSync(projectMd)) return;
  const parsedProject = readProjectMdAttributes(fs.readFileSync(projectMd, "utf8"));
  const featureSet = new Set(parsedProject.features.map((name) => normalizeFeatureName(name)).filter(Boolean));
  if (featureSet.size === 0) return;

  const draftsPath = draftsYamlPath(projectPath);
  if (fs.existsSync(draftsPath)) {
    const rawDrafts = fs.readFileSync(draftsPath, "utf8");
    const doc = normalizeDraftStateDoc(((YAML.parse(rawDrafts) ?? {}) as DraftsDoc) ?? {});
    let changed = false;
    const nextWorked: string[] = [];
    const nextComplete = new Set<string>(doc.complete ?? []);
    for (const name of doc.worked ?? []) {
      if (featureSet.has(normalizeFeatureName(name))) {
        nextComplete.add(normalizeFeatureName(name));
        changed = true;
      } else {
        nextWorked.push(normalizeFeatureName(name));
      }
    }
    if (changed) {
      const normalized: DraftsDoc = normalizeDraftStateDoc({
        ...doc,
        worked: nextWorked,
        complete: [...nextComplete]
      });
      fs.writeFileSync(draftsPath, YAML.stringify(normalized), "utf8");
    }
  }

  const planPath = planYamlPath(projectPath);
  if (fs.existsSync(planPath)) {
    const rawPlan = fs.readFileSync(planPath, "utf8");
    const planDoc = ((YAML.parse(rawPlan) ?? {}) as PlanDoc) ?? {};
    const drafts = planDoc.drafts ?? {};
    const planned = dedupNormalized(Array.isArray(drafts.planned) ? drafts.planned : []);
    const worked = dedupNormalized(Array.isArray(drafts.worked) ? drafts.worked : []);
    const complete = dedupNormalized(Array.isArray(drafts.complete) ? drafts.complete : []);
    let changed = false;
    const nextWorked: string[] = [];
    const nextComplete = new Set<string>(complete);
    for (const name of worked) {
      if (featureSet.has(normalizeFeatureName(name))) {
        nextComplete.add(normalizeFeatureName(name));
        changed = true;
      } else {
        nextWorked.push(normalizeFeatureName(name));
      }
    }
    if (changed) {
      const nextPlanned = planned.filter((name) => !nextComplete.has(name) && !nextWorked.includes(name));
      planDoc.drafts = {
        ...drafts,
        planned: nextPlanned,
        worked: nextWorked,
        complete: [...nextComplete]
      };
      fs.writeFileSync(planPath, YAML.stringify(planDoc), "utf8");
    }
  }
}

function isBootstrapCompleted(projectPath: string): boolean {
  if (!fs.existsSync(projectPath)) {
    return false;
  }
  const entries = fs.readdirSync(projectPath, { withFileTypes: true });
  return entries.some((entry) => ![".project", ".git", ".jj"].includes(entry.name));
}

function resolveProjectState(project: ProjectRecord): ProjectState {
  if (buildProcessesByProject.has(project.id)) {
    return "build";
  }
  if (runProcessesByProject.has(project.id)) {
    return "run";
  }
  const pmdPath = projectMdPath(project.path);
  if (!fs.existsSync(pmdPath)) {
    return "init";
  }

  const drafts = loadDraftsDoc(project.path);
  const plannedCount = listCount(drafts.planned);
  const workedCount = listCount(drafts.worked);
  const completeCount = listCount(drafts.complete);
  const failedCount = listCount(drafts.failed);
  if (plannedCount > 0 || workedCount > 0) {
    return "work";
  }
  if (completeCount > 0 && (failedCount > 0 || plannedCount > 0 || workedCount > 0)) {
    return "review";
  }
  if (completeCount > 0 && plannedCount === 0 && workedCount === 0) {
    return "review";
  }
  if (!isBootstrapCompleted(project.path)) {
    return "init";
  }
  return "basic";
}

function parseInputTitles(projectPath: string): {
  raw: string;
  titles: string[];
  items: Array<{ title: string; rule: string; step: string }>;
} {
  const inputPath = path.join(projectPath, "input.md");
  if (!fs.existsSync(inputPath)) {
    return { raw: "", titles: [], items: [] };
  }
  const raw = fs.readFileSync(inputPath, "utf8");
  const items: Array<{ title: string; rule: string; step: string }> = [];
  let active: { title: string; rule: string; step: string } | null = null;
  for (const line of raw.split(/\r?\n/)) {
    const t = line.trim();
    if (/^#{1,6}\s+/.test(t)) {
      if (active && active.title) {
        items.push(active);
      }
      active = {
        title: t.replace(/^#{1,6}\s+/, "").trim(),
        rule: "",
        step: ""
      };
      continue;
    }
    if (!active || !t.startsWith("- ")) {
      continue;
    }
    const body = t.slice(2).trim();
    const [rulePart, ...stepParts] = body.split(">");
    if (!active.rule) {
      active.rule = rulePart.trim();
      active.step = stepParts.join(">").trim();
    }
  }
  if (active && active.title) {
    items.push(active);
  }
  const uniqueItems: Array<{ title: string; rule: string; step: string }> = [];
  const seen = new Set<string>();
  for (const item of items) {
    const key = item.title.trim();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    uniqueItems.push(item);
  }
  return {
    raw,
    titles: uniqueItems.map((item) => item.title),
    items: uniqueItems
  };
}

function parseDraftItems(projectPath: string): {
  raw: string;
  items: Array<Record<string, unknown>>;
  cards: Array<{ name: string; status: "work" | "wait" | "complete"; draft: Record<string, unknown> }>;
} {
  const file = draftsYamlPath(projectPath);
  if (!fs.existsSync(file)) {
    return { raw: "", items: [], cards: [] };
  }
  const raw = fs.readFileSync(file, "utf8");
  const parsed = YAML.parse(raw) ?? {};
  const items = Array.isArray(parsed?.draft)
    ? parsed.draft.filter((row: unknown) => row && typeof row === "object")
    : [];
  const normalizeName = (value: unknown): string =>
    String(value ?? "")
      .trim()
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, "_")
      .replace(/^_+|_+$/g, "");
  const addNamesTo = (target: Set<string>, values: unknown) => {
    if (!Array.isArray(values)) return;
    for (const rawName of values) {
      const normalized = normalizeName(rawName);
      if (normalized) target.add(normalized);
    }
  };
  const planned = new Set<string>();
  const worked = new Set<string>();
  const complete = new Set<string>();
  const failed = new Set<string>();
  addNamesTo(planned, parsed?.planned);
  addNamesTo(worked, parsed?.worked);
  addNamesTo(complete, parsed?.complete);
  addNamesTo(failed, parsed?.failed);
  const planPath = planYamlPath(projectPath);
  if (fs.existsSync(planPath)) {
    const planRaw = fs.readFileSync(planPath, "utf8");
    const planParsed = YAML.parse(planRaw) ?? {};
    const planDrafts = (planParsed?.drafts ?? {}) as Record<string, unknown>;
    addNamesTo(planned, planDrafts.planned);
    addNamesTo(worked, planDrafts.worked);
    addNamesTo(complete, planDrafts.complete);
    addNamesTo(failed, planDrafts.failed);
  }
  const cards = (items as Array<Record<string, unknown>>)
    .map((row) => {
      const name = String(row.name ?? "").trim();
      if (!name) return null;
      const nameKey = normalizeName(name);
      let status: "work" | "wait" | "complete" = "wait";
      if (complete.has(nameKey)) {
        status = "complete";
      } else if (worked.has(nameKey)) {
        status = "work";
      } else if (planned.has(nameKey)) {
        status = "wait";
      }
      return { name, status, draft: row };
    })
    .filter((row): row is { name: string; status: "work" | "wait" | "complete"; draft: Record<string, unknown> } =>
      Boolean(row)
    );
  const cardNames = new Set(cards.map((card) => normalizeName(card.name)));
  const fromStatus = (nameKey: string): "work" | "wait" | "complete" => {
    if (complete.has(nameKey)) return "complete";
    if (worked.has(nameKey)) return "work";
    return "wait";
  };
  for (const nameKey of [...planned, ...worked, ...complete, ...failed]) {
    if (!nameKey || cardNames.has(nameKey)) continue;
    cards.push({
      name: nameKey,
      status: fromStatus(nameKey),
      draft: { name: nameKey }
    });
  }
  return { raw, items: items as Array<Record<string, unknown>>, cards };
}

function appendRuntimeLog(id: string, line: string): void {
  const current = runtimeLogsByProject.get(id) ?? [];
  current.push(line);
  if (current.length > 500) {
    current.splice(0, current.length - 500);
  }
  runtimeLogsByProject.set(id, current);
}

function normalizeRuntimeUrl(url: string): string {
  if (url.startsWith("http://0.0.0.0:")) {
    return url.replace("http://0.0.0.0:", "http://127.0.0.1:");
  }
  if (url.startsWith("https://0.0.0.0:")) {
    return url.replace("https://0.0.0.0:", "https://127.0.0.1:");
  }
  return url;
}

function maybeCaptureRuntimeUrl(id: string, line: string): void {
  const match = line.match(/https?:\/\/[^\s)]+/);
  if (!match) return;
  const nextUrl = normalizeRuntimeUrl(match[0]);
  runUrlsByProject.set(id, nextUrl);
}

function isNextProject(projectPath: string): boolean {
  return [".js", ".ts", ".mjs", ".cjs"]
    .map((ext) => path.join(projectPath, `next.config${ext}`))
    .some((file) => fs.existsSync(file));
}

function isAstroProject(projectPath: string): boolean {
  return [".mjs", ".ts", ".js", ".cjs"]
    .map((ext) => path.join(projectPath, `astro.config${ext}`))
    .some((file) => fs.existsSync(file));
}

function looksLikeSameNextProcess(pid: number, projectPath: string): boolean {
  const cmdlinePath = path.join("/proc", String(pid), "cmdline");
  if (!fs.existsSync(cmdlinePath)) return false;
  const cmdline = fs.readFileSync(cmdlinePath, "utf8").replace(/\0/g, " ");
  const normalizedProject = path.resolve(projectPath);
  return cmdline.includes("next") && cmdline.includes("dev") && cmdline.includes(normalizedProject);
}

function ensureNextLockState(projectPath: string): void {
  const lockPath = path.join(projectPath, ".next", "dev", "lock");
  if (!fs.existsSync(lockPath)) return;
  const raw = fs.readFileSync(lockPath, "utf8").trim();
  const pid = Number.parseInt(raw, 10);
  if (Number.isFinite(pid) && pid > 0) {
    try {
      process.kill(pid, 0);
      if (looksLikeSameNextProcess(pid, projectPath)) {
        process.kill(pid, "SIGTERM");
        fs.rmSync(lockPath, { force: true });
        return;
      }
      throw new Error(`next dev already running (pid=${pid}) at ${projectPath}`);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ESRCH") {
        throw error;
      }
    }
  }
  fs.rmSync(lockPath, { force: true });
}

function resolveDevCommand(projectPath: string, port: number): { cmd: string; args: string[]; kind: "next" | "bun" } {
  if (isNextProject(projectPath)) {
    ensureNextLockState(projectPath);
    return {
      cmd: "bunx",
      args: ["next", "dev", "--port", String(port), "--hostname", "127.0.0.1"],
      kind: "next"
    };
  }
  if (isAstroProject(projectPath)) {
    return {
      cmd: "bunx",
      args: ["astro", "dev", "--port", String(port), "--host", "127.0.0.1"],
      kind: "bun"
    };
  }
  return {
    cmd: "bun",
    args: ["run", "dev", "--", "--port", String(port)],
    kind: "bun"
  };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForRuntimeUrl(id: string, timeoutMs: number): Promise<string | undefined> {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const url = runUrlsByProject.get(id);
    if (url) return url;
    if (!runProcessesByProject.has(id)) return undefined;
    await sleep(80);
  }
  return runUrlsByProject.get(id);
}

export function getRuntimeLogs(id: string): string[] {
  const logs = runtimeLogsByProject.get(id) ?? [];
  return [...logs].reverse();
}

function hashProjectId(id: string): number {
  let hash = 0;
  for (let i = 0; i < id.length; i += 1) {
    hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  }
  return hash;
}

async function isPortAvailable(port: number): Promise<boolean> {
  return await new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.once("listening", () => {
      server.close(() => resolve(true));
    });
    server.listen(port, "127.0.0.1");
  });
}

async function allocateProjectPort(id: string): Promise<number> {
  const fixed = runPortsByProject.get(id);
  if (typeof fixed === "number") return fixed;

  const range = DEV_PORT_MAX - DEV_PORT_MIN + 1;
  const start = DEV_PORT_MIN + (hashProjectId(id) % range);
  const inUse = new Set<number>([...runPortsByProject.values()]);

  for (let offset = 0; offset < range; offset += 1) {
    const candidate = DEV_PORT_MIN + ((start - DEV_PORT_MIN + offset) % range);
    if (inUse.has(candidate)) continue;
    if (await isPortAvailable(candidate)) return candidate;
  }
  throw new Error(`no free dev port in range ${DEV_PORT_MIN}-${DEV_PORT_MAX}`);
}

async function run_dev_server(
  id: string,
  detail: Pick<ProjectDetail, "name" | "path">,
  port: number
): Promise<{ url?: string }> {
  const command = resolveDevCommand(detail.path, port);
  const proc = spawn(command.cmd, command.args, {
    cwd: detail.path,
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, PORT: String(port) },
    detached: true
  });
  runProcessesByProject.set(id, proc);
  runPortsByProject.set(id, port);
  appendRuntimeLog(
    id,
    `[run-dev] start: ${detail.name} (${detail.path}) port=${port} cmd=${command.cmd} ${command.args.join(" ")}`
  );

  proc.stdout.on("data", (chunk) => {
    const lines = String(chunk)
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
    for (const line of lines) {
      appendRuntimeLog(id, line);
      maybeCaptureRuntimeUrl(id, line);
    }
  });
  proc.stderr.on("data", (chunk) => {
    const lines = String(chunk)
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
    for (const line of lines) {
      appendRuntimeLog(id, line);
      maybeCaptureRuntimeUrl(id, line);
    }
  });
  proc.on("error", (error) => {
    appendRuntimeLog(id, `[run-dev] error: ${String(error)}`);
    runProcessesByProject.delete(id);
    runPortsByProject.delete(id);
    runUrlsByProject.delete(id);
  });
  proc.on("close", (code, signal) => {
    appendRuntimeLog(
      id,
      `[run-dev] exited: code=${code === null ? "null" : String(code)} signal=${signal ?? "none"}`
    );
    runProcessesByProject.delete(id);
    runPortsByProject.delete(id);
    runUrlsByProject.delete(id);
  });
  const detectedUrl = await waitForRuntimeUrl(id, 2500);
  return { url: detectedUrl };
}

export async function runProjectDev(
  id: string
): Promise<{ output: string; running: boolean; port?: number; url?: string }> {
  const detail = loadProjectDetail(id);
  const running = runProcessesByProject.get(id);
  if (running) {
    appendRuntimeLog(id, `[run-dev] stop requested: ${detail.name}`);
    if (typeof running.pid === "number") {
      try {
        process.kill(-running.pid, "SIGTERM");
      } catch {
        // fallback to single-process termination below
      }
    }
    runProcessesByProject.delete(id);
    runPortsByProject.delete(id);
    runUrlsByProject.delete(id);
    running.kill("SIGTERM");
    return { output: `bun run dev stopped: ${detail.name}`, running: false };
  }
  const port = await allocateProjectPort(id);
  const { url: detectedUrl } = await run_dev_server(id, detail, port);
  await sleep(400);
  const stillRunning = runProcessesByProject.has(id);
  const fallbackUrl = `http://127.0.0.1:${port}`;
  const resolvedUrl = detectedUrl ?? (stillRunning ? fallbackUrl : undefined);
  if (!stillRunning) {
    const latest = runtimeLogsByProject.get(id)?.slice(-1)[0] ?? "process exited early";
    return {
      output: `bun run dev failed: ${detail.name} | ${latest}`,
      running: false,
      port
    };
  }
  return {
    output: `bun run dev started: ${detail.name} (requested port ${port})`,
    running: true,
    port,
    url: resolvedUrl
  };
}

export function listProjects(): ProjectRecord[] {
  const registry = loadRegistry();
  return registry.projects.map((project) => ({
    ...project,
    state: resolveProjectState(project),
    current_job: buildCurrentJobByProject.get(project.id) || ""
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
  reconcileDraftCompletionFromProjectFeatures(project.path);
  const parsed = readProjectMdAttributes(fs.readFileSync(projectMdPath(project.path), "utf8"));
  const drafts = loadDraftsList(project.path);
  const hasDraftsYaml = fs.existsSync(draftsYamlPath(project.path));
  const planned = Array.isArray(drafts.planned) ? drafts.planned : [];
  const plannedItems = Array.isArray(drafts.planned_items) ? drafts.planned_items : [];
  const memo = fs.existsSync(memoPath(project.path)) ? fs.readFileSync(memoPath(project.path), "utf8") : "";
  const inputMd = parseInputTitles(project.path);
  const draftItems = parseDraftItems(project.path);

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
    current_job: buildCurrentJobByProject.get(project.id) || "",
    hasDraftsYaml,
    dev_server_url: runProcessesByProject.has(project.id) ? runUrlsByProject.get(project.id) : undefined
    ,
    draftsYamlRaw: draftItems.raw,
    inputMdRaw: inputMd.raw,
    inputTitles: inputMd.titles,
    inputItems: inputMd.items,
    draftItems: draftItems.items,
    draftsYamlItems: draftItems.cards
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

function loadPlanDoc(projectPath: string): PlanDoc {
  const file = planYamlPath(projectPath);
  if (!fs.existsSync(file)) return {};
  return ((YAML.parse(fs.readFileSync(file, "utf8")) ?? {}) as PlanDoc) ?? {};
}

function savePlanDoc(projectPath: string, doc: PlanDoc): void {
  fs.writeFileSync(planYamlPath(projectPath), YAML.stringify(doc), "utf8");
}

function saveDraftsDoc(projectPath: string, doc: DraftsDoc): void {
  fs.writeFileSync(draftsYamlPath(projectPath), YAML.stringify(doc), "utf8");
}

function retryIncompleteDrafts(id: string): string {
  const detail = loadProjectDetail(id);
  const draftsPath = draftsYamlPath(detail.path);
  if (!fs.existsSync(draftsPath)) {
    return "retry_incomplete skipped: drafts.yaml not found";
  }
  const drafts = normalizeDraftStateDoc(loadDraftsDoc(detail.path));
  const plan = loadPlanDoc(detail.path);
  const planDrafts = plan.drafts ?? {};

  const retrySet = new Set<string>([
    ...dedupNormalized(drafts.planned ?? []),
    ...dedupNormalized(drafts.failed ?? []),
    ...dedupNormalized((planDrafts.failed ?? []) as string[])
  ]);
  if (retrySet.size === 0) {
    return "retry_incomplete skipped: no red item";
  }

  const retryList = [...retrySet];
  drafts.planned = dedupNormalized([...(drafts.planned ?? []), ...retryList]);
  drafts.failed = dedupNormalized(drafts.failed ?? []).filter((name) => !retrySet.has(name));
  drafts.worked = dedupNormalized(drafts.worked ?? []).filter((name) => !retrySet.has(name));
  saveDraftsDoc(detail.path, drafts);

  plan.drafts = {
    ...planDrafts,
    planned: dedupNormalized([...(planDrafts.planned ?? []), ...retryList]),
    worked: dedupNormalized((planDrafts.worked ?? []) as string[]).filter((name) => !retrySet.has(name)),
    failed: dedupNormalized((planDrafts.failed ?? []) as string[]).filter((name) => !retrySet.has(name)),
    complete: dedupNormalized((planDrafts.complete ?? []) as string[])
  };
  savePlanDoc(detail.path, plan);

  const orcBin = process.env.ORC_BIN ?? "orc";
  const result = spawnSync(orcBin, ["impl_code_draft"], {
    cwd: detail.path,
    encoding: "utf8"
  });
  const output = (result.stdout || "").trim();
  const stderr = (result.stderr || "").trim();
  if (result.status !== 0) {
    throw new Error(stderr || `retry_incomplete failed: status=${String(result.status)}`);
  }
  return output || `retry_incomplete completed: ${retryList.join(", ")}`;
}

function finalizeCompletedDrafts(id: string): string {
  const detail = loadProjectDetail(id);
  const draftsPath = draftsYamlPath(detail.path);
  if (!fs.existsSync(draftsPath)) {
    return "finalize_complete skipped: drafts.yaml not found";
  }
  const drafts = normalizeDraftStateDoc(loadDraftsDoc(detail.path));
  const completed = dedupNormalized(drafts.complete ?? []);
  if (completed.length === 0) {
    return "finalize_complete skipped: no green item";
  }

  const parsed = readProjectMdAttributes(fs.readFileSync(projectMdPath(detail.path), "utf8"));
  const mergedFeatures = dedupNormalized([...parsed.features, ...completed]);
  writeProjectMd(detail.path, {
    name: parsed.name || detail.name,
    description: parsed.description || detail.description,
    spec: parsed.spec || detail.spec,
    goal: parsed.goal || detail.goal,
    rules: parsed.rules,
    constraints: parsed.constraints,
    features: mergedFeatures
  });

  const plan = loadPlanDoc(detail.path);
  const planDrafts = plan.drafts ?? {};
  const completeSet = new Set(completed);
  plan.drafts = {
    ...planDrafts,
    complete: dedupNormalized([...(planDrafts.complete ?? []), ...completed]),
    planned: dedupNormalized((planDrafts.planned ?? []) as string[]).filter((name) => !completeSet.has(name)),
    worked: dedupNormalized((planDrafts.worked ?? []) as string[]).filter((name) => !completeSet.has(name)),
    failed: dedupNormalized((planDrafts.failed ?? []) as string[]).filter((name) => !completeSet.has(name))
  };
  savePlanDoc(detail.path, plan);

  const inputPath = path.join(detail.path, "input.md");
  if (fs.existsSync(inputPath)) {
    fs.writeFileSync(inputPath, "", "utf8");
  }
  fs.rmSync(draftsPath, { force: true });
  return `finalize_complete completed: ${completed.join(", ")} | input.md cleared`;
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
  if (action === "retry_incomplete") {
    return retryIncompleteDrafts(id);
  }
  if (action === "finalize_complete") {
    return finalizeCompletedDrafts(id);
  }
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

export async function applyFormAddInput(
  id: string,
  items: Array<{ title: string; rule: string; step: string }>
): Promise<{ detail: ProjectDetail; stages: string[] }> {
  const detail = loadProjectDetail(id);
  if (!items.length) {
    throw new Error("at least one input item is required");
  }
  const normalized = items
    .map((item) => ({
      title: item.title.trim(),
      rule: item.rule.trim(),
      step: item.step.trim()
    }))
    .filter((item) => item.title.length > 0 && item.rule.length > 0 && item.step.length > 0);
  if (!normalized.length) {
    throw new Error("all title/rule/step are empty");
  }
  const inputBody = normalized
    .map((item) => `# ${item.title}\n- ${item.rule} > ${item.step}`)
    .join("\n\n");
  const inputPath = path.join(detail.path, "input.md");
  fs.writeFileSync(inputPath, `${inputBody}\n`, "utf8");

  const stages = await runInputMdSyncWorkflow(id, detail.path);
  return { detail: loadProjectDetail(id), stages };
}

export function saveRawInputMd(id: string, raw: string): ProjectDetail {
  const detail = loadProjectDetail(id);
  const inputPath = path.join(detail.path, "input.md");
  const body = raw.trimEnd();
  fs.writeFileSync(inputPath, body.length > 0 ? `${body}\n` : "", "utf8");
  return loadProjectDetail(id);
}

export async function applyRawInputMd(id: string, raw: string): Promise<{ detail: ProjectDetail; stages: string[] }> {
  const detail = saveRawInputMd(id, raw);
  const stages = await runInputMdSyncWorkflow(id, detail.path);
  return { detail: loadProjectDetail(id), stages };
}

export function generateInputMdFromMessage(id: string, message: string): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const prompt = message.trim();
  if (!prompt) {
    throw new Error("message is required");
  }
  const planCommand = resolveOrcCommandArgs(["add_code_plan", "-m", prompt]);
  appendRuntimeLog(id, `[input-generate] add_code_plan -m "${prompt}"`);
  const planResult = spawnSync(planCommand.bin, planCommand.args, {
    cwd: detail.path,
    encoding: "utf8"
  });
  if (planResult.status !== 0) {
    const stderr = (planResult.stderr || "").trim();
    throw new Error(stderr || `add_code_plan failed: status=${String(planResult.status)}`);
  }
  const inputCommand = resolveOrcCommandArgs(["create_input_md"]);
  appendRuntimeLog(id, "[input-generate] create_input_md");
  const inputResult = spawnSync(inputCommand.bin, inputCommand.args, {
    cwd: detail.path,
    encoding: "utf8"
  });
  if (inputResult.status !== 0) {
    const stderr = (inputResult.stderr || "").trim();
    throw new Error(stderr || `create_input_md failed: status=${String(inputResult.status)}`);
  }
  const output = (inputResult.stdout || "").trim() || "create_input_md completed";
  appendRuntimeLog(id, `[input-generate] ${output}`);
  return { detail: loadProjectDetail(id), output };
}

export function runAutoFromMessage(id: string, message: string): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const prompt = message.trim();
  if (!prompt) {
    throw new Error("message is required");
  }
  const command = resolveOrcCommandArgs(["auto", prompt]);
  appendRuntimeLog(id, `[auto] start: ${detail.name}`);
  const result = spawnSync(command.bin, command.args, {
    cwd: detail.path,
    encoding: "utf8"
  });
  const stdout = (result.stdout || "").trim();
  const stderr = (result.stderr || "").trim();
  if (result.status !== 0) {
    appendRuntimeLog(id, `[auto] failed: ${stderr || `status=${String(result.status)}`}`);
    throw new Error(stderr || `auto failed: status=${String(result.status)}`);
  }
  const output = stdout || "auto completed";
  appendRuntimeLog(id, `[auto] ${output}`);
  return { detail: loadProjectDetail(id), output };
}

async function runInputMdSyncWorkflow(id: string, projectPath: string): Promise<string[]> {
  const stages: Array<{ label: string; args: string[] }> = [
    { label: "plan.yaml", args: ["add_code_plan", "-f"] },
    { label: "drafts.yaml", args: ["add_code_draft", "-f"] },
    { label: "draft_item.yaml", args: ["add_code_draft_item", "-f"] }
  ];
  const outputs: string[] = [];
  for (const stage of stages) {
    appendRuntimeLog(id, `[form_add_input] ${stage.label} 작업중...`);
    const result = await runOrcStageWithLogs(id, projectPath, stage.args, stage.label);
    outputs.push(`${stage.label}: ${result}`);
    appendRuntimeLog(id, `[form_add_input] ${stage.label} 완료`);
  }
  return outputs;
}

function runOrcStageWithLogs(
  id: string,
  projectPath: string,
  args: string[],
  label: string
): Promise<string> {
  return new Promise((resolve, reject) => {
    const command = resolveOrcCommandArgs(args);
    const proc = spawn(command.bin, command.args, {
      cwd: projectPath,
      stdio: ["ignore", "pipe", "pipe"]
    });
    let lastStdout = "";
    proc.stdout.on("data", (chunk) => {
      const lines = String(chunk)
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter((line) => line.length > 0);
      for (const line of lines) {
        lastStdout = line;
        appendRuntimeLog(id, `[form_add_input] ${label} | ${line}`);
      }
    });
    proc.stderr.on("data", (chunk) => {
      const lines = String(chunk)
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter((line) => line.length > 0);
      for (const line of lines) {
        appendRuntimeLog(id, `[form_add_input] ${label} | ${line}`);
      }
    });
    proc.on("error", (error) => {
      reject(new Error(`[${label}] ${String(error)}`));
    });
    proc.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`[${label}] failed with code=${String(code)}`));
        return;
      }
      resolve(lastStdout || "ok");
    });
  });
}

export function startParallelBuild(id: string): { output: string } {
  const detail = loadProjectDetail(id);
  if (buildProcessesByProject.has(id)) {
    return { output: `build already running: ${detail.name}` };
  }
  const orcBin = process.env.ORC_BIN ?? "orc";
  const proc = spawn(orcBin, ["impl_code_draft"], {
    cwd: detail.path,
    stdio: ["ignore", "pipe", "pipe"],
    detached: true
  });
  buildProcessesByProject.set(id, proc);
  buildCurrentJobByProject.set(id, "starting");
  appendRuntimeLog(id, `[build] started: ${detail.name}`);

  const updateJob = (line: string) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    buildCurrentJobByProject.set(id, trimmed.slice(0, 200));
    appendRuntimeLog(id, `[build] ${trimmed}`);
  };
  proc.stdout.on("data", (chunk) => {
    const lines = String(chunk).split(/\r?\n/);
    for (const line of lines) updateJob(line);
  });
  proc.stderr.on("data", (chunk) => {
    const lines = String(chunk).split(/\r?\n/);
    for (const line of lines) updateJob(line);
  });
  proc.on("close", (code, signal) => {
    reconcileDraftCompletionFromProjectFeatures(detail.path);
    const message = `[build] finished: code=${code === null ? "null" : String(code)} signal=${signal ?? "none"}`;
    appendRuntimeLog(id, message);
    buildCompletionByProject.set(id, message);
    buildProcessesByProject.delete(id);
    buildCurrentJobByProject.delete(id);
  });
  proc.on("error", (error) => {
    reconcileDraftCompletionFromProjectFeatures(detail.path);
    const message = `[build] error: ${String(error)}`;
    appendRuntimeLog(id, message);
    buildCompletionByProject.set(id, message);
    buildProcessesByProject.delete(id);
    buildCurrentJobByProject.delete(id);
  });
  return { output: `build started: ${detail.name}` };
}

export function stopParallelBuild(id: string): { output: string } {
  const detail = loadProjectDetail(id);
  const running = buildProcessesByProject.get(id);
  if (!running) {
    return { output: `build not running: ${detail.name}` };
  }
  appendRuntimeLog(id, `[build] stop requested: ${detail.name}`);
  if (typeof running.pid === "number") {
    try {
      process.kill(-running.pid, "SIGTERM");
    } catch {
      // ignore process-group kill errors and fallback to direct kill
    }
  }
  running.kill("SIGTERM");
  buildProcessesByProject.delete(id);
  buildCurrentJobByProject.delete(id);
  buildCompletionByProject.set(id, `[build] stopped by user`);
  return { output: `build stopped: ${detail.name}` };
}

export function getBuildStatus(id: string): { state: ProjectState; current_job: string; completed?: string } {
  const detail = loadProjectDetail(id);
  const completed = buildCompletionByProject.get(id);
  if (completed) {
    buildCompletionByProject.delete(id);
  }
  return {
    state: resolveProjectState({
      id: detail.id,
      name: detail.name,
      path: detail.path,
      description: detail.description,
      created_at: "",
      updated_at: "",
      selected: true,
      project_type: detail.project_type
    }),
    current_job: buildCurrentJobByProject.get(id) || "",
    completed
  };
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
