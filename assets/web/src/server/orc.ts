import fs from "node:fs";
import path from "node:path";
import net from "node:net";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import YAML from "yaml";
import { parseRequirementBlocks } from "@/lib/requirement-parser";

export type ProjectRecord = {
  id: string;
  name: string;
  path: string;
  description: string;
  created_at: string;
  updated_at: string;
  selected: boolean;
  project_type: "code" | "mono";
  state?: ProjectState;
  current_job?: string;
  is_dev_running?: boolean;
  is_build_running?: boolean;
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

export type ProjectState = "wait" | "work" | "complete" | "auto";
export type ProfileType = "code" | "mono";

const runtimeLogsByProject = new Map<string, string[]>();
const runProcessesByProject = new Map<string, ChildProcess>();
const runPortsByProject = new Map<string, number>();
const runUrlsByProject = new Map<string, string>();
const buildProcessesByProject = new Map<string, ChildProcess>();
const buildCurrentJobByProject = new Map<string, string>();
const buildCompletionByProject = new Map<string, string>();
const autoProcessesByProject = new Map<string, ChildProcess>();
const autoCurrentJobByProject = new Map<string, string>();
const autoCompletionByProject = new Map<string, string>();
const DEV_PORT_MIN = 4300;
const DEV_PORT_MAX = 4999;
export type BrowseEntry = { name: string; path: string; hasProjectMeta: boolean };
export type MonorepoPackage = {
  id: string;
  name: string;
  path: string;
  kind: "app" | "feature" | "template";
};

export type DomainRow = { name: string; description: string; features: string[]; is_active?: boolean };

export type ProjectDetail = {
  id: string;
  name: string;
  description: string;
  path: string;
  memo: string;
  project_type: "code" | "mono";
  spec: string;
  goal: string;
  architecture: string;
  rules: string[];
  constraints: string[];
  features: string[];
  domains: DomainRow[];
  planned: string[];
  plannedDisplay: string[];
  generated: string[];
  state: ProjectState;
  current_job?: string;
  is_dev_running?: boolean;
  is_build_running?: boolean;
  hasDraftsYaml: boolean;
  hasJobMd: boolean;
  dev_server_url?: string;
  draftsYamlRaw?: string;
  jobMdRaw?: string;
  jobEditableRaw?: string;
  draftItems: Array<Record<string, unknown>>;
  draftsYamlItems: Array<{
    name: string;
    status: "work" | "wait" | "complete";
    draft: Record<string, unknown>;
  }>;
  checkSubject: string;
  checkSteps: Array<{
    subject: string;
    text: string;
    source: string;
  }>;
  feedbackMdRaw: string;
  hasFeedbackMd: boolean;
  requirementBlocks?: Array<{
    title: string;
    rules: string[];
    steps: string[];
  }>;
  screenshots: Array<{
    name: string;
    path: string;
    url: string;
    modifiedAt: string;
  }>;
};

export function repoRoot(): string {
  return process.env.ORC_ROOT ?? path.resolve(process.cwd(), "..", "..");
}

function browseRoot(): string {
  return process.env.ORC_BROWSE_ROOT ?? "/home/tree";
}

type ResolvedWorkspaceCommand = {
  bin: string;
  args: string[];
};

function resolveWorkspaceCommandArgs(binary: "orc" | "rc", args: string[]): ResolvedWorkspaceCommand {
  const directEnvBin =
    binary === "orc" ? (process.env.ORC_BIN ?? "").trim() : ((process.env.ORC_RC_BIN ?? process.env.RC_BIN) ?? "").trim();
  if (directEnvBin.length > 0) {
    return { bin: directEnvBin, args };
  }
  if (binary === "rc") {
    const orcBin = (process.env.ORC_BIN ?? "").trim();
    if (orcBin.length > 0) {
      const parsed = path.parse(orcBin);
      const siblingRc = path.join(parsed.dir, `rc${parsed.ext}`);
      return { bin: siblingRc, args };
    }
  }
  const root = repoRoot();
  const legacyAssets = path.join(root, "assets", "code");
  const presetsAssets = path.join(root, "assets", "presets", "code");
  if (!fs.existsSync(legacyAssets) && fs.existsSync(presetsAssets)) {
    return {
      bin: "cargo",
      args: ["run", "--quiet", "--manifest-path", path.join(root, "Cargo.toml"), "--bin", binary, "--", ...args]
    };
  }
  return { bin: binary, args };
}

function resolveOrcCommandArgs(args: string[]): ResolvedWorkspaceCommand {
  return resolveWorkspaceCommandArgs("orc", args);
}

function resolveRcCommandArgs(args: string[]): ResolvedWorkspaceCommand {
  return resolveWorkspaceCommandArgs("rc", args);
}

function createTaskSessionKey(label: string): string {
  return `${label}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function buildTaskCommandEnv(taskKey: string, extra: Record<string, string> = {}): NodeJS.ProcessEnv {
  return {
    ...process.env,
    ORC_TASK_SESSION_KEY: taskKey,
    ...extra
  };
}

function monorepoRoot(): string {
  const envRoot = (process.env.ORC_MONOREPO_ROOT ?? "").trim();
  const home = process.env.HOME ?? "/home/tree";
  const candidates = [envRoot, path.join(home, "oneMono"), path.join(home, "home")].filter(Boolean);
  for (const candidate of candidates) {
    const domainsDir = path.join(candidate, "packages", "domains");
    if (fs.existsSync(domainsDir) && fs.statSync(domainsDir).isDirectory()) {
      return candidate;
    }
  }
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
  if (raw === "mono") {
    return raw;
  }
  return "code";
}

function normalizeProfileType(raw: unknown): ProfileType {
  if (raw === "mono") {
    return raw;
  }
  return "code";
}

function profileTypeFromProjectType(projectType: ProjectRecord["project_type"]): ProfileType {
  if (projectType === "mono") return "mono";
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
  const draftItemTemplatePath = path.join(profileAssetsDir(profile), "templates", "draft_item.yaml");
  const raw = safeReadFile(draftItemTemplatePath);
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
      project_type: normalizeProjectType(project.project_type),
      state: normalizeProjectState(project.state)
    }))
  };
}

function normalizeProjectState(raw: unknown): ProjectState | undefined {
  if (raw === "auto") return "auto";
  if (raw === "complete") return "complete";
  if (raw === "work" || raw === "review" || raw === "run" || raw === "build") return "work";
  if (raw === "wait" || raw === "init" || raw === "basic") return "wait";
  return undefined;
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

function jobMdPath(projectPath: string): string {
  return path.join(projectPath, "job.md");
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

function screenshotDirPath(projectPath: string): string {
  return path.join(projectMetaDir(projectPath), "screenshot");
}

function instructionRetryPath(projectPath: string): string {
  return path.join(projectPath, "instruction_retry.md");
}

function ensureProjectFiles(project: ProjectRecord): void {
  fs.mkdirSync(project.path, { recursive: true });
  fs.mkdirSync(projectMetaDir(project.path), { recursive: true });

  const pmd = projectMdPath(project.path);
  if (!fs.existsSync(pmd)) {
    fs.writeFileSync(
      pmd,
      `# info\nname: ${project.name}\ndescription: ${project.description}\nspec: auto\ngoal: init\n\n# architecture\nname: \n\n# rules\n- \n\n# constraints\n- \n\n# features\n- \n`,
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

function monorepoDomainDetails(root: string): DomainRow[] {
  return collectMonorepoDomains(root).map((name) => {
    const domainPath = path.join(root, "packages", "domains", name);
    const files = collectSourceFiles(domainPath);
    const features = new Map<string, string>();
    for (const filePath of files) {
      let raw = "";
      try {
        raw = fs.readFileSync(filePath, "utf8");
      } catch {
        continue;
      }
      const relative = path.relative(root, filePath).replace(/\\/g, "/");
      const functions = extractFunctionNames(filePath, raw);
      for (const fn of functions) {
        const key = normalizeFunctionKey(fn);
        if (!key || features.has(key)) continue;
        features.set(key, `${fn}: ${relative} 함수`);
      }
    }
    return { name, description: "", features: [...features.values()] };
  });
}

function mergeDomainRows(base: DomainRow[], overlay: DomainRow[]): DomainRow[] {
  const overlayByName = new Map(
    overlay
      .map((domain) => [domain.name.trim(), domain] as const)
      .filter(([name]) => name.length > 0)
  );
  const merged = base.map((domain) => {
    const override = overlayByName.get(domain.name.trim());
    const features = [...new Set([...(override?.features ?? []), ...domain.features])];
    return {
      ...domain,
      description: override?.description?.trim() || domain.description,
      features
    };
  });
  for (const domain of overlay) {
    const name = domain.name.trim();
    if (!name || merged.some((item) => item.name.trim() === name)) continue;
    merged.push({
      name,
      description: domain.description,
      features: [...new Set(domain.features)]
    });
  }
  return merged;
}

function extractSelfDomainName(projectPath: string, root: string): string | null {
  const domainRoot = path.join(root, "packages", "domains");
  const relative = path.relative(domainRoot, projectPath);
  if (
    relative.startsWith("..") ||
    path.isAbsolute(relative) ||
    relative.length === 0
  ) {
    return null;
  }
  const [domainName] = relative.split(path.sep);
  return domainName?.trim() ? domainName : null;
}

function resolveMonoProjectDomains(projectPath: string, root: string, parsedDomains: DomainRow[]): DomainRow[] {
  const activeNames = new Set(
    parsedDomains
      .map((domain) => domain.name.trim())
      .filter((name) => name.length > 0)
  );
  const selfDomainName = extractSelfDomainName(projectPath, root);
  if (selfDomainName) {
    activeNames.add(selfDomainName);
  }
  return mergeDomainRows(monorepoDomainDetails(root), parsedDomains).map((domain) => ({
    ...domain,
    is_active: activeNames.has(domain.name)
  }));
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
      goal: detail.goal,
      architecture: detail.architecture
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
  const targetPath = path.resolve(String(target.path ?? "").trim());
  const rootPath = path.parse(targetPath).root;
  if (!targetPath || targetPath === rootPath) {
    throw new Error(`refusing to delete unsafe project path: ${target.path}`);
  }
  registry.projects = registry.projects.filter((p) => p.id !== id);
  if (registry.recentActivepane === id) {
    registry.recentActivepane = registry.projects[0]?.id ?? "";
  }
  if (registry.projects.length > 0 && !registry.projects.some((p) => p.selected)) {
    registry.projects[0].selected = true;
  }
  saveRegistry(registry);

  if (fs.existsSync(targetPath)) {
    fs.rmSync(targetPath, { recursive: true, force: true });
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
  architecture: string;
  rules: string[];
  constraints: string[];
  features: string[];
  domains: DomainRow[];
} {
  const out = {
    name: "",
    description: "",
    spec: "",
    goal: "",
    architecture: "",
    rules: [] as string[],
    constraints: [] as string[],
    features: [] as string[],
    domains: [] as DomainRow[]
  };

  let section: "rules" | "constraints" | "features" | "none" = "none";
  let inArchitecture = false;
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
    if (t.toLowerCase() === "# architecture") {
      section = "none";
      inArchitecture = true;
      continue;
    }
    if (t.toLowerCase() === "# domains") {
      section = "none";
      inArchitecture = false;
      inDomains = true;
      continue;
    }
    if (t.startsWith("#")) {
      section = "none";
      if (inArchitecture && t.toLowerCase() !== "# architecture") {
        inArchitecture = false;
      }
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
    if (inArchitecture && key === "name") out.architecture = value;
    if (!inArchitecture && key === "name") out.name = value;
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
  architecture: string;
  rules: string[];
  constraints: string[];
  features: string[];
  domains?: DomainRow[];
}): void {
  const lines = [
    "# info",
    `name: ${doc.name}`,
    `description: ${doc.description}`,
    `spec: ${doc.spec}`,
    `goal: ${doc.goal}`,
    "",
    "# architecture",
    `name: ${doc.architecture}`,
    "",
    "# rules",
    ...(doc.rules.length > 0 ? doc.rules : [""]).map((v) => `- ${v}`),
    "",
    "# constraints",
    ...(doc.constraints.length > 0 ? doc.constraints : [""]).map((v) => `- ${v}`),
    "",
    "# features",
    ...(doc.features.length > 0 ? doc.features : [""]).map((v) => `- ${v}`)
  ];
  const domains = Array.isArray(doc.domains) ? doc.domains : [];
  if (domains.length > 0) {
    lines.push("");
    lines.push("# domains");
    for (const domain of domains) {
      lines.push(`## ${domain.name}`);
      if (domain.description.trim().length > 0) {
        lines.push("### description");
        lines.push(`- ${domain.description.trim()}`);
      }
      lines.push("### feature");
      if (domain.features.length === 0) {
        lines.push("- ");
      } else {
        for (const feature of domain.features) {
          lines.push(`- ${feature}`);
        }
      }
      lines.push("");
    }
  } else {
    lines.push("");
  }
  fs.writeFileSync(projectMdPath(projectPath), `${lines.join("\n").trimEnd()}\n`, "utf8");
}

function normalizeFunctionKey(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "");
}

function shouldSkipSourceDir(name: string): boolean {
  return name === ".git" || name === ".jj" || name === ".project" || name === "node_modules" || name === "dist" || name === "target";
}

function collectSourceFiles(root: string): string[] {
  const out: string[] = [];
  const stack = [root];
  const allowedExt = new Set([".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go"]);
  while (stack.length > 0) {
    const current = stack.pop()!;
    let entries: fs.Dirent[] = [];
    try {
      entries = fs.readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (entry.isDirectory()) {
        if (shouldSkipSourceDir(entry.name)) continue;
        stack.push(path.join(current, entry.name));
        continue;
      }
      if (!entry.isFile()) continue;
      const ext = path.extname(entry.name).toLowerCase();
      if (!allowedExt.has(ext)) continue;
      out.push(path.join(current, entry.name));
    }
  }
  return out;
}

function extractFunctionNames(filePath: string, raw: string): string[] {
  const ext = path.extname(filePath).toLowerCase();
  const names = new Set<string>();
  const push = (value: string) => {
    const next = value.trim();
    if (!next) return;
    names.add(next);
  };
  const readMatches = (pattern: RegExp) => {
    for (const match of raw.matchAll(pattern)) {
      const hit = String(match[1] ?? "").trim();
      if (hit.length > 0) push(hit);
    }
  };

  if (ext === ".rs") {
    readMatches(/\b(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g);
  } else if (ext === ".ts" || ext === ".tsx" || ext === ".js" || ext === ".jsx") {
    readMatches(/\b(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g);
    readMatches(/\b(?:export\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>/g);
  } else if (ext === ".py") {
    readMatches(/^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/gm);
  } else if (ext === ".go") {
    readMatches(/^\s*func\s+(?:\([^)]+\)\s*)?([A-Za-z_][A-Za-z0-9_]*)\s*\(/gm);
  }
  return [...names];
}

function domainMatchesFunction(domainName: string, relativePath: string, functionName: string): boolean {
  const domain = domainName.trim().toLowerCase();
  if (!domain) return false;
  const rel = relativePath.toLowerCase().replace(/\\/g, "/");
  const file = path.basename(rel);
  const fn = functionName.toLowerCase();
  if (rel.includes(`/${domain}/`) || rel.startsWith(`${domain}/`) || rel.includes(`/domains/${domain}/`)) return true;
  if (file.includes(domain)) return true;
  if (fn.startsWith(domain) || fn.includes(`_${domain}_`) || fn.includes(`${domain}_`) || fn.includes(domain)) return true;
  return false;
}

function syncDomainFeaturesFromSource(
  projectPath: string,
  targetDomainName?: string
): { updatedDomains: number; addedFeatures: number } {
  const projectMd = projectMdPath(projectPath);
  if (!fs.existsSync(projectMd)) return { updatedDomains: 0, addedFeatures: 0 };
  if (isMonorepoManagedPath(projectPath, monorepoRoot())) return { updatedDomains: 0, addedFeatures: 0 };
  const parsed = readProjectMdAttributes(fs.readFileSync(projectMd, "utf8"));
  if (parsed.domains.length === 0) return { updatedDomains: 0, addedFeatures: 0 };
  const targetDomain = String(targetDomainName ?? "").trim().toLowerCase();

  const files = collectSourceFiles(projectPath);
  let changed = false;
  let updatedDomains = 0;
  let addedFeatures = 0;
  const nextDomains: DomainRow[] = parsed.domains.map((domain) => {
    const isTarget = !targetDomain || domain.name.trim().toLowerCase() === targetDomain;
    if (!isTarget) {
      return domain;
    }
    const existingByKey = new Map<string, string>();
    for (const feature of domain.features) {
      const key = normalizeFunctionKey(feature.split(":")[0] ?? feature);
      if (!key || existingByKey.has(key)) continue;
      existingByKey.set(key, feature);
    }
    let addedForDomain = 0;

    for (const filePath of files) {
      const relative = path.relative(projectPath, filePath).replace(/\\/g, "/");
      let raw = "";
      try {
        raw = fs.readFileSync(filePath, "utf8");
      } catch {
        continue;
      }
      const functions = extractFunctionNames(filePath, raw);
      for (const fn of functions) {
        if (!domainMatchesFunction(domain.name, relative, fn)) continue;
        const key = normalizeFunctionKey(fn);
        if (!key || existingByKey.has(key)) continue;
        existingByKey.set(key, `${fn}: ${relative} 함수`);
        changed = true;
        addedForDomain += 1;
      }
    }
    if (addedForDomain > 0) {
      updatedDomains += 1;
      addedFeatures += addedForDomain;
    }
    return { ...domain, features: [...existingByKey.values()] };
  });

  if (!changed) return { updatedDomains: 0, addedFeatures: 0 };
  writeProjectMd(projectPath, {
    name: parsed.name,
    description: parsed.description,
    spec: parsed.spec,
    goal: parsed.goal,
    architecture: parsed.architecture,
    rules: parsed.rules,
    constraints: parsed.constraints,
    features: parsed.features,
    domains: nextDomains
  });
  return { updatedDomains, addedFeatures };
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

function toTextList(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value
      .map((item) => String(item ?? "").trim())
      .filter((item) => item.length > 0);
  }
  const single = String(value ?? "").trim();
  return single ? [single] : [];
}

function guessMimeType(filePath: string): string {
  const ext = path.extname(filePath).toLowerCase();
  if (ext === ".png") return "image/png";
  if (ext === ".jpg" || ext === ".jpeg") return "image/jpeg";
  if (ext === ".webp") return "image/webp";
  if (ext === ".gif") return "image/gif";
  if (ext === ".svg") return "image/svg+xml";
  return "application/octet-stream";
}

function ensureFeedbackFile(projectPath: string): string {
  const file = jobMdPath(projectPath);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  if (!fs.existsSync(file)) {
    const fallback =
      "# plan\n\n# requirement\n\n# task\n## planned\n## work\n## verify\n## complete\n## failed\n\n# problems\n\n# check\n";
    fs.writeFileSync(file, fallback, "utf8");
    return file;
  }
  return file;
}

function defaultInstructionRetryContent(): string {
  return [
    "# instruction_retry",
    "- job.md의 # problems 와 # check 내용을 먼저 읽고, 현재 미해결 문제를 기준으로 재시도 범위를 다시 정리한다.",
    "- 현재 ORC 워크플로 산출물 기준으로 drafts.yaml을 다시 만들고 병렬 처리 과정을 다시 시작한다.",
    "- 이미 완료된 수정 요약이 아니라, # problems 문제를 해결하기 위한 새 계획과 구현 순서를 우선 작성한다.",
    "- 필요한 경우 job.md를 다시 갱신하되, 최종 목적은 job.md -> drafts.yaml -> 병렬 처리 재실행이다."
  ].join("\n");
}

function ensureInstructionRetryFile(projectPath: string): string {
  const file = instructionRetryPath(projectPath);
  if (!fs.existsSync(file) || !fs.readFileSync(file, "utf8").trim()) {
    fs.writeFileSync(file, `${defaultInstructionRetryContent()}\n`, "utf8");
  }
  return file;
}

function appendMarkdownBullet(filePath: string, headers: string[], bullet: string): string {
  const raw = fs.existsSync(filePath) ? fs.readFileSync(filePath, "utf8") : "";
  const lines = raw.length > 0 ? raw.split(/\r?\n/) : [];
  const normalizedHeaders = headers.map((header) => header.trim().toLowerCase());
  let startIndex = -1;
  for (let i = 0; i < lines.length; i += 1) {
    if (normalizedHeaders.includes(lines[i].trim().toLowerCase())) {
      startIndex = i;
      break;
    }
  }
  if (startIndex < 0) {
    const next = `${raw.trimEnd()}\n\n${headers[0]}\n${bullet}\n`.replace(/^\n+/, "");
    fs.writeFileSync(filePath, next, "utf8");
    return next;
  }

  let endIndex = lines.length;
  for (let i = startIndex + 1; i < lines.length; i += 1) {
    if (lines[i].trim().startsWith("#")) {
      endIndex = i;
      break;
    }
  }
  const insert: string[] = [];
  if (endIndex > startIndex + 1 && lines[endIndex - 1].trim().length > 0) {
    insert.push("");
  }
  insert.push(bullet);
  lines.splice(endIndex, 0, ...insert);
  const next = `${lines.join("\n").trimEnd()}\n`;
  fs.writeFileSync(filePath, next, "utf8");
  return next;
}

function readFeedbackMarkdown(projectPath: string): string {
  const raw = safeReadFile(jobMdPath(projectPath));
  const sections = [
    { header: "# problems", body: extractMarkdownSection(raw, "# problems") },
    { header: "# check", body: extractMarkdownSection(raw, "# check") }
  ].filter((section) => section.body.trim().length > 0);
  if (sections.length === 0) {
    return "";
  }
  return sections.map((section) => `${section.header}\n${section.body}`.trim()).join("\n\n");
}

function extractMarkdownSection(raw: string, header: string): string {
  const lines = raw.split(/\r?\n/);
  const normalizedHeader = header.trim().toLowerCase();
  let start = -1;
  for (let i = 0; i < lines.length; i += 1) {
    if (lines[i].trim().toLowerCase() === normalizedHeader) {
      start = i + 1;
      break;
    }
  }
  if (start < 0) {
    return "";
  }
  const out: string[] = [];
  for (let i = start; i < lines.length; i += 1) {
    if (lines[i].trim().startsWith("# ")) {
      break;
    }
    out.push(lines[i]);
  }
  return out.join("\n").trim();
}

function listScreenshotItems(
  projectId: string,
  projectPath: string
): Array<{ name: string; path: string; url: string; modifiedAt: string }> {
  const dir = screenshotDirPath(projectPath);
  fs.mkdirSync(dir, { recursive: true });
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => {
      const fullPath = path.join(dir, entry.name);
      const modifiedAt = fs
        .statSync(fullPath)
        .mtimeMs.toString();
      return {
        name: entry.name,
        path: fullPath,
        url: `/api/check-screenshot?id=${encodeURIComponent(projectId)}&name=${encodeURIComponent(entry.name)}`,
        modifiedAt
      };
    })
    .sort((a, b) => Number(b.modifiedAt) - Number(a.modifiedAt));
}

function collectCheckPlan(projectPath: string): {
  subject: string;
  steps: Array<{ subject: string; text: string; source: string }>;
} {
  const parsed = parseDraftItems(projectPath);
  const subjects: string[] = [];
  const steps: Array<{ subject: string; text: string; source: string }> = [];
  for (const rawItem of parsed.items) {
    const name = String(rawItem.name ?? "").trim();
    if (!name) continue;
    subjects.push(name);
    const checks = toTextList(rawItem.check);
    const explicitSteps = toTextList(rawItem.step);
    const tasks = toTextList(rawItem.tasks);
    const sourceRows =
      checks.length > 0
        ? checks.map((text) => ({ subject: name, text, source: "check" }))
        : explicitSteps.length > 0
          ? explicitSteps.map((text) => ({ subject: name, text, source: "step" }))
          : tasks.map((text) => ({ subject: name, text, source: "tasks" }));
    steps.push(...sourceRows);
  }
  const uniqueSubjects = [...new Set(subjects)];
  return {
    subject:
      uniqueSubjects.length > 0
        ? uniqueSubjects.join(" / ")
        : "drafts.yaml 기반 수동 check 대상이 아직 없습니다.",
    steps:
      steps.length > 0
        ? steps
        : uniqueSubjects.map((subject) => ({
            subject,
            text: `${subject} 구현 결과를 수동 check 대상으로 확인한다.`,
            source: "generated"
          }))
  };
}

function moveRcScreenshotArtifacts(projectPath: string): string[] {
  const rootCandidates = [path.join(projectPath, "rc-web.png")];
  const screenshotDir = screenshotDirPath(projectPath);
  fs.mkdirSync(screenshotDir, { recursive: true });
  const moved: string[] = [];
  for (const source of rootCandidates) {
    if (!fs.existsSync(source)) continue;
    const ext = path.extname(source) || ".png";
    const target = path.join(screenshotDir, `rc-web-${Date.now()}-${Math.random().toString(36).slice(2, 8)}${ext}`);
    fs.renameSync(source, target);
    moved.push(target);
  }
  return moved;
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
}

function isBootstrapCompleted(projectPath: string): boolean {
  if (!fs.existsSync(projectPath)) {
    return false;
  }
  const entries = fs.readdirSync(projectPath, { withFileTypes: true });
  return entries.some((entry) => ![".project", ".git", ".jj"].includes(entry.name));
}

function resolveProjectState(project: ProjectRecord): ProjectState {
  if (autoProcessesByProject.has(project.id)) {
    return "auto";
  }
  if (buildProcessesByProject.has(project.id) || runProcessesByProject.has(project.id)) {
    return "work";
  }
  if (project.state === "complete") {
    return "complete";
  }
  const pmdPath = projectMdPath(project.path);
  if (!fs.existsSync(pmdPath)) {
    return "wait";
  }

  const drafts = loadDraftsDoc(project.path);
  const plannedCount = listCount(drafts.planned);
  const workedCount = listCount(drafts.worked);
  const completeCount = listCount(drafts.complete);
  const failedCount = listCount(drafts.failed);
  if (plannedCount > 0 || workedCount > 0 || failedCount > 0) {
    return "work";
  }
  if (completeCount > 0 && plannedCount === 0 && workedCount === 0) {
    return "complete";
  }
  if (!isBootstrapCompleted(project.path)) {
    return "wait";
  }
  return "wait";
}

function normalizeRequirementHeader(raw: string): string {
  return raw
    .replace(/^#\s*requriements\b/gim, "# requirement")
    .replace(/^#\s*requirements\b/gim, "# requirement");
}

function extractJobManagedSection(raw: string): string {
  const normalized = normalizeRequirementHeader(raw);
  const lines = normalized.split(/\r?\n/);
  let start = -1;
  for (let i = 0; i < lines.length; i += 1) {
    const t = lines[i].trim().toLowerCase();
    if (t === "# task" || t === "# problems") {
      start = i;
      break;
    }
  }
  if (start < 0) {
    return "# task\n## planned\n## work\n## verify\n## complete\n## fail\n\n# problems\n\n# check\n";
  }
  return `${lines.slice(start).join("\n").trimEnd()}\n`;
}

function extractJobEditableSection(raw: string): string {
  const normalized = normalizeRequirementHeader(raw);
  const lines = normalized.split(/\r?\n/);
  const out: string[] = [];
  let inEditable = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const t = line.trim().toLowerCase();
    if (t === "# plan") {
      inEditable = true;
      out.push("# plan");
      continue;
    }
    if (t === "# requirement") {
      inEditable = true;
      out.push("# requirement");
      continue;
    }
    if (t === "# task" || t === "# problems") {
      break;
    }
    if (!inEditable) {
      continue;
    }
    out.push(line);
  }
  const body = out.join("\n").trim();
  if (body.length === 0) {
    return "# plan\n\n# requirement\n";
  }
  const hasPlan = /^#\s*plan\b/im.test(body);
  const hasReq = /^#\s*requirement\b/im.test(body);
  const withPlan = hasPlan ? body : `# plan\n\n${body}`;
  const withReq = hasReq ? withPlan : `${withPlan}\n\n# requirement`;
  return `${withReq.trimEnd()}\n`;
}

function extractRequirementSection(raw: string): string {
  const normalized = normalizeRequirementHeader(raw);
  const lines = normalized.split(/\r?\n/);
  const out: string[] = [];
  let inRequirement = false;
  for (const line of lines) {
    const trimmed = line.trim().toLowerCase();
    if (trimmed === "# requirement") {
      inRequirement = true;
      continue;
    }
    if (inRequirement && trimmed.startsWith("# ")) {
      break;
    }
    if (inRequirement) out.push(line);
  }
  return out.join("\n").trim();
}

function extractPlanSection(editableRaw: string): string {
  const lines = editableRaw.split(/\r?\n/);
  const out: string[] = [];
  let inPlan = false;
  for (const line of lines) {
    const trimmed = line.trim().toLowerCase();
    if (trimmed === "# plan") {
      inPlan = true;
      out.push("# plan");
      continue;
    }
    if (trimmed === "# requirement") {
      break;
    }
    if (inPlan) out.push(line);
  }
  if (out.length === 0) return "# plan\n";
  return `${out.join("\n").trimEnd()}\n`;
}

function renderRequirementBlocks(blocks: Array<{ title: string; rules: string[]; steps: string[] }>): string {
  if (blocks.length === 0) return "";
  const out: string[] = [];
  for (const block of blocks) {
    out.push(`## ${block.title}`);
    for (const rule of block.rules) out.push(`- ${rule}`);
    for (const [index, step] of block.steps.entries()) out.push(`${index + 1}. ${step}`);
    out.push("");
  }
  return out.join("\n").trimEnd();
}

function parseRequirementItemsFromRaw(raw: string): Array<{ title: string; rule: string; step: string }> {
  const blocks = parseRequirementBlocks(raw);
  const items: Array<{ title: string; rule: string; step: string }> = [];
  for (const block of blocks) {
    const rules = block.rules.length > 0 ? block.rules : [""];
    const steps = block.steps.length > 0 ? block.steps : [""];
    const max = Math.max(rules.length, steps.length, 1);
    for (let index = 0; index < max; index += 1) {
      items.push({
        title: block.title,
        rule: rules[index] ?? "",
        step: steps[index] ?? ""
      });
    }
  }
  return items;
}

function buildJobMdFromEditable(editableRaw: string, currentRaw: string): string {
  const editable = extractJobEditableSection(editableRaw);
  const managed = extractJobManagedSection(currentRaw);
  return `${editable.trimEnd()}\n\n${managed.trimStart()}`;
}

function readJobMd(projectPath: string): { raw: string; editableRaw: string } {
  const jobPath = path.join(projectPath, "job.md");
  if (!fs.existsSync(jobPath)) {
    const editableRaw = "# plan\n\n# requirement\n";
    return {
      raw: `${editableRaw}\n# task\n## planned\n## work\n## verify\n## complete\n## fail\n\n# problems\n\n# check\n`,
      editableRaw
    };
  }
  const raw = fs.readFileSync(jobPath, "utf8");
  return { raw, editableRaw: extractJobEditableSection(raw) };
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
  if (autoProcessesByProject.has(id)) {
    return { output: `auto already running: ${detail.name}`, running: false };
  }
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
    current_job:
      autoCurrentJobByProject.get(project.id) ||
      buildCurrentJobByProject.get(project.id) ||
      (runProcessesByProject.has(project.id) ? "dev server" : ""),
    is_dev_running: runProcessesByProject.has(project.id),
    is_build_running: buildProcessesByProject.has(project.id)
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
  syncDomainFeaturesFromSource(project.path);
  const parsed = readProjectMdAttributes(fs.readFileSync(projectMdPath(project.path), "utf8"));
  const drafts = loadDraftsList(project.path);
  const hasDraftsYaml = fs.existsSync(draftsYamlPath(project.path));
  const hasJobMd = fs.existsSync(path.join(project.path, "job.md"));
  const planned = Array.isArray(drafts.planned) ? drafts.planned : [];
  const plannedItems = Array.isArray(drafts.planned_items) ? drafts.planned_items : [];
  const memo = fs.existsSync(memoPath(project.path)) ? fs.readFileSync(memoPath(project.path), "utf8") : "";
  const jobMd = readJobMd(project.path);
  const requirementBlocks = parseRequirementBlocks(extractRequirementSection(jobMd.editableRaw));
  const draftItems = parseDraftItems(project.path);
  const checkPlan = collectCheckPlan(project.path);
  const feedbackMdRaw = readFeedbackMarkdown(project.path);
  const screenshots = listScreenshotItems(project.id, project.path);

  const root = monorepoRoot();
  const domains = project.project_type === "mono"
    ? resolveMonoProjectDomains(project.path, root, parsed.domains)
    : parsed.domains;
  return {
    id: project.id,
    name: parsed.name || project.name,
    description: parsed.description || project.description,
    path: project.path,
    memo,
    project_type: project.project_type,
    spec: parsed.spec,
    goal: parsed.goal,
    architecture: parsed.architecture,
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
    current_job:
      autoCurrentJobByProject.get(project.id) ||
      buildCurrentJobByProject.get(project.id) ||
      (runProcessesByProject.has(project.id) ? "dev server" : ""),
    is_dev_running: runProcessesByProject.has(project.id),
    is_build_running: buildProcessesByProject.has(project.id),
    hasDraftsYaml,
    hasJobMd,
    dev_server_url: runProcessesByProject.has(project.id) ? runUrlsByProject.get(project.id) : undefined
    ,
    draftsYamlRaw: draftItems.raw,
    jobMdRaw: jobMd.raw,
    jobEditableRaw: jobMd.editableRaw,
    draftItems: draftItems.items,
    draftsYamlItems: draftItems.cards,
    checkSubject: checkPlan.subject,
    checkSteps: checkPlan.steps,
    feedbackMdRaw,
    hasFeedbackMd: feedbackMdRaw.trim().length > 0,
    requirementBlocks,
    screenshots
  };
}

export function refreshDomainFeatures(id: string, domainName?: string): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const result = syncDomainFeaturesFromSource(detail.path, domainName);
  return {
    detail: loadProjectDetail(id),
    output: `domain sync updated=${result.updatedDomains} added=${result.addedFeatures}`
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
  architecture: string;
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
    architecture: input.architecture,
    rules: current.rules,
    constraints: current.constraints,
    features: current.features,
    domains: current.domains
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
    architecture: current.architecture,
    rules: input.rules,
    constraints: input.constraints,
    features: input.features,
    domains: current.domains
  });

  const drafts = loadDraftsList(current.path);
  drafts.features = input.features;
  saveDraftsList(current.path, drafts);
  return loadProjectDetail(id);
}

export function saveDomains(id: string, input: {
  domains: DomainRow[];
}): ProjectDetail {
  const current = loadProjectDetail(id);
  const normalizedDomains = input.domains
    .map((domain) => ({
      name: String(domain.name ?? "").trim(),
      description: String(domain.description ?? "").trim(),
      features: Array.isArray(domain.features)
        ? domain.features.map((feature) => String(feature ?? "").trim()).filter((feature) => feature.length > 0)
        : []
    }))
    .filter((domain) => domain.name.length > 0);
  writeProjectMd(current.path, {
    name: current.name,
    description: current.description,
    spec: current.spec,
    goal: current.goal,
    architecture: current.architecture,
    rules: current.rules,
    constraints: current.constraints,
    features: current.features,
    domains: normalizedDomains
  });
  return loadProjectDetail(id);
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

  const retrySet = new Set<string>([
    ...dedupNormalized(drafts.planned ?? []),
    ...dedupNormalized(drafts.failed ?? [])
  ]);
  if (retrySet.size === 0) {
    return "retry_incomplete skipped: no red item";
  }

  const retryList = [...retrySet];
  drafts.planned = dedupNormalized([...(drafts.planned ?? []), ...retryList]);
  drafts.failed = dedupNormalized(drafts.failed ?? []).filter((name) => !retrySet.has(name));
  drafts.worked = dedupNormalized(drafts.worked ?? []).filter((name) => !retrySet.has(name));
  saveDraftsDoc(detail.path, drafts);

  const taskKey = createTaskSessionKey("retry-incomplete");
  const command = resolveOrcCommandArgs(["impl_code_draft"]);
  const result = spawnSync(command.bin, command.args, {
    cwd: detail.path,
    encoding: "utf8",
    env: buildTaskCommandEnv(taskKey)
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
    architecture: parsed.architecture || detail.architecture,
    rules: parsed.rules,
    constraints: parsed.constraints,
    features: mergedFeatures,
    domains: parsed.domains
  });

  fs.rmSync(draftsPath, { force: true });
  return `finalize_complete completed: ${completed.join(", ")}`;
}

export function runOrcAction(id: string, action: string, payload?: string): string {
  const detail = loadProjectDetail(id);
  const projectDraftsPath = draftsYamlPath(detail.path);
  const projectJobPath = path.join(detail.path, "job.md");
  if (action === "add_draft" && !fs.existsSync(projectDraftsPath)) {
    throw new Error("add_draft blocked: drafts.yaml not found");
  }
  if (action === "check_code" && !fs.existsSync(projectDraftsPath)) {
    throw new Error("check_code blocked: drafts.yaml not found");
  }
  if (action === "impl_draft" && !fs.existsSync(projectDraftsPath)) {
    throw new Error("impl_draft blocked: drafts.yaml not found");
  }
  if (action === "create_draft" && !fs.existsSync(projectJobPath)) {
    throw new Error("create_draft blocked: job.md not found");
  }
  const argsMap: Record<string, string[]> = {
    create_draft: ["create_code_draft"],
    add_draft: payload?.trim().length ? ["add_code_draft", "-m", payload] : ["add_code_draft", "-a"],
    impl_draft: ["impl_code_draft"],
    check_code: ["check_orc_code"]
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

  const taskKey = createTaskSessionKey(action);
  const command = resolveOrcCommandArgs(args);
  const result = spawnSync(command.bin, command.args, {
    cwd: detail.path,
    encoding: "utf8",
    env: buildTaskCommandEnv(taskKey)
  });

  if (result.status !== 0) {
    const stderr = (result.stderr || "").trim();
    throw new Error(stderr.length > 0 ? stderr : `command failed: ${command.bin} ${command.args.join(" ")}`);
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
    .filter((item) => item.title.length > 0);
  if (!normalized.length) {
    throw new Error("all title are empty");
  }

  const currentRaw = fs.existsSync(path.join(detail.path, "job.md"))
    ? fs.readFileSync(path.join(detail.path, "job.md"), "utf8")
    : "";
  const currentEditable = extractJobEditableSection(currentRaw);
  const currentBlocks = parseRequirementBlocks(extractRequirementSection(currentEditable));
  const incomingBlocks = parseRequirementBlocks(
    normalized
      .map((item) => {
        const lines: string[] = [`## ${item.title}`];
        if (item.rule.length > 0) lines.push(`- ${item.rule}`);
        if (item.step.length > 0) lines.push(`> ${item.step}`);
        return lines.join("\n");
      })
      .join("\n")
  );
  const mergedBlocks = [...currentBlocks, ...incomingBlocks];
  const planSection = extractPlanSection(currentEditable);
  const requirementBody = renderRequirementBlocks(mergedBlocks);
  const editableBody = `${planSection.trimEnd()}\n\n# requirement${requirementBody ? `\n${requirementBody}` : ""}\n`;
  const jobBody = buildJobMdFromEditable(editableBody, currentRaw);
  const jobPath = path.join(detail.path, "job.md");
  fs.writeFileSync(jobPath, jobBody, "utf8");

  const stages = await runJobMdSyncWorkflow(id, detail.path);
  return { detail: loadProjectDetail(id), stages };
}

export async function applyRawRequirementInput(
  id: string,
  raw: string
): Promise<{ detail: ProjectDetail; stages: string[]; parsedCount: number }> {
  const detail = loadProjectDetail(id);
  const normalizedRaw = raw.trim();
  if (!normalizedRaw) {
    throw new Error("requirement input is empty");
  }
  const parsedItems = parseRequirementItemsFromRaw(normalizedRaw);
  if (parsedItems.length === 0) {
    throw new Error("no parseable requirement blocks");
  }
  const { detail: nextDetail, stages } = await applyFormAddInput(id, parsedItems);
  return { detail: nextDetail, stages, parsedCount: parsedItems.length };
}

export function deleteRequirementItem(
  id: string,
  index: number
): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  if (!Number.isInteger(index) || index < 0) {
    throw new Error("index must be a non-negative integer");
  }

  const jobPath = path.join(detail.path, "job.md");
  if (!fs.existsSync(jobPath)) {
    throw new Error("job.md not found");
  }

  const currentRaw = fs.readFileSync(jobPath, "utf8");
  const currentEditable = extractJobEditableSection(currentRaw);
  const currentBlocks = parseRequirementBlocks(extractRequirementSection(currentEditable));
  if (index >= currentBlocks.length) {
    throw new Error("requirement index out of range");
  }

  const removed = currentBlocks[index];
  const nextBlocks = currentBlocks.filter((_, i) => i !== index);
  const planSection = extractPlanSection(currentEditable);
  const requirementBody = renderRequirementBlocks(nextBlocks);
  const editableBody = `${planSection.trimEnd()}\n\n# requirement${requirementBody ? `\n${requirementBody}` : ""}\n`;
  const jobBody = buildJobMdFromEditable(editableBody, currentRaw);
  fs.writeFileSync(jobPath, jobBody, "utf8");
  appendRuntimeLog(id, `[requirement] removed index=${index} title=${removed?.title ?? "unknown"}`);
  return { detail: loadProjectDetail(id), output: `requirement removed: ${removed?.title ?? index}` };
}

export function saveRawJobMd(id: string, raw: string): ProjectDetail {
  const detail = loadProjectDetail(id);
  const jobPath = path.join(detail.path, "job.md");
  const currentRaw = fs.existsSync(jobPath) ? fs.readFileSync(jobPath, "utf8") : "";
  const merged = buildJobMdFromEditable(raw, currentRaw);
  fs.writeFileSync(jobPath, merged, "utf8");
  return loadProjectDetail(id);
}
export function saveRawDraftsYaml(id: string, raw: string): ProjectDetail {
  const detail = loadProjectDetail(id);
  const nextRaw = raw.trim();
  if (nextRaw.length === 0) {
    fs.rmSync(draftsYamlPath(detail.path), { force: true });
    return loadProjectDetail(id);
  }
  const parsed = YAML.parse(nextRaw);
  if (!parsed || typeof parsed !== "object") {
    throw new Error("drafts.yaml must be a valid YAML object");
  }
  fs.writeFileSync(draftsYamlPath(detail.path), `${nextRaw}\n`, "utf8");
  return loadProjectDetail(id);
}


export function deleteDraftPaneFile(
  id: string,
  target: "job" | "drafts"
): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  if (target === "job") {
    const jobPath = path.join(detail.path, "job.md");
    if (!fs.existsSync(jobPath)) {
      return { detail: loadProjectDetail(id), output: "job.md already missing" };
    }
    fs.rmSync(jobPath, { force: true });
    appendRuntimeLog(id, "[drafts-pane] job.md deleted");
    return { detail: loadProjectDetail(id), output: "job.md deleted" };
  }
  const file = draftsYamlPath(detail.path);
  if (!fs.existsSync(file)) {
    return { detail: loadProjectDetail(id), output: "drafts.yaml already missing" };
  }
  fs.rmSync(file, { force: true });
  appendRuntimeLog(id, "[drafts-pane] drafts.yaml deleted");
  return { detail: loadProjectDetail(id), output: "drafts.yaml deleted" };
}

export async function applyRawJobMd(id: string, raw: string): Promise<{ detail: ProjectDetail; stages: string[] }> {
  const detail = saveRawJobMd(id, raw);
  const stages = await runJobMdSyncWorkflow(id, detail.path);
  return { detail: loadProjectDetail(id), stages };
}

export function generateJobMdFromMessage(id: string, message: string): { detail: ProjectDetail; output: string } {
  const normalized = message.trim();
  if (!normalized) throw new Error("message is required");
  const parsedItems = parseRequirementItemsFromRaw(normalized);
  if (parsedItems.length === 0) {
    throw new Error("message parse failed: no requirement blocks");
  }
  const detail = loadProjectDetail(id);
  const currentRaw = fs.existsSync(path.join(detail.path, "job.md"))
    ? fs.readFileSync(path.join(detail.path, "job.md"), "utf8")
    : "";
  const currentEditable = extractJobEditableSection(currentRaw);
  const currentBlocks = parseRequirementBlocks(extractRequirementSection(currentEditable));
  const incomingBlocks = parseRequirementBlocks(normalized);
  const mergedBlocks = [...currentBlocks, ...incomingBlocks];
  const planSection = extractPlanSection(currentEditable);
  const requirementBody = renderRequirementBlocks(mergedBlocks);
  const editableBody = `${planSection.trimEnd()}\n\n# requirement${requirementBody ? `\n${requirementBody}` : ""}\n`;
  const nextRaw = buildJobMdFromEditable(editableBody, currentRaw);
  fs.writeFileSync(path.join(detail.path, "job.md"), nextRaw, "utf8");
  const taskKey = createTaskSessionKey("job-generate");
  const outputs: string[] = [];
  const stages: Array<{ label: string; args: string[] }> = [{ label: "drafts.yaml", args: ["add_orc_drafts"] }];
  for (const stage of stages) {
    appendRuntimeLog(id, `[job-generate] ${stage.label}`);
    const command = resolveOrcCommandArgs(stage.args);
    const result = spawnSync(command.bin, command.args, {
      cwd: detail.path,
      encoding: "utf8",
      env: buildTaskCommandEnv(taskKey)
    });
    if (result.status !== 0) {
      const stderr = (result.stderr || "").trim();
      throw new Error(stderr || `${stage.label} failed: status=${String(result.status)}`);
    }
    outputs.push((result.stdout || "").trim() || `${stage.label} completed`);
  }
  return { detail: loadProjectDetail(id), output: outputs.join(" | ") };
}

export function syncDraftsFromJobRequirements(id: string): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const jobPath = path.join(detail.path, "job.md");
  if (!fs.existsSync(jobPath)) {
    throw new Error("job.md not found");
  }
  const taskKey = createTaskSessionKey("job-to-drafts");
  appendRuntimeLog(id, "[job->drafts] drafts.yaml sync start");
  const command = resolveOrcCommandArgs(["add_orc_drafts"]);
  const result = spawnSync(command.bin, command.args, {
    cwd: detail.path,
    encoding: "utf8",
    env: buildTaskCommandEnv(taskKey)
  });
  if (result.status !== 0) {
    const stderr = (result.stderr || "").trim();
    throw new Error(stderr || `drafts.yaml sync failed: status=${String(result.status)}`);
  }
  const output = (result.stdout || "").trim() || "drafts.yaml synced";
  appendRuntimeLog(id, `[job->drafts] ${output}`);
  return { detail: loadProjectDetail(id), output };
}

export function startAutoFromMessage(id: string, message: string): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const prompt = message.trim();
  if (!prompt) {
    throw new Error("message is required");
  }
  if (autoProcessesByProject.has(id)) {
    return { detail: loadProjectDetail(id), output: `auto already running: ${detail.name}` };
  }
  const command = resolveOrcCommandArgs(["auto_add_function", prompt]);
  const taskKey = createTaskSessionKey("auto");
  appendRuntimeLog(id, `[auto] start: ${detail.name}`);
  const proc = spawn(command.bin, command.args, {
    cwd: detail.path,
    stdio: ["ignore", "pipe", "pipe"],
    env: buildTaskCommandEnv(taskKey)
  });
  autoProcessesByProject.set(id, proc);
  autoCurrentJobByProject.set(id, "starting");

  const updateJob = (line: string) => {
    const trimmed = line.trim();
    if (!trimmed) return;
    autoCurrentJobByProject.set(id, trimmed.slice(0, 200));
    appendRuntimeLog(id, `[auto] ${trimmed}`);
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
    autoProcessesByProject.delete(id);
    autoCurrentJobByProject.delete(id);
    const message =
      code === 0
        ? "[auto] completed"
        : `[auto] failed: code=${code === null ? "null" : String(code)} signal=${signal ?? "none"}`;
    appendRuntimeLog(id, message);
    autoCompletionByProject.set(id, message);
    if (code === 0) {
      setProjectState(id, "complete");
    }
  });
  proc.on("error", (error) => {
    autoProcessesByProject.delete(id);
    autoCurrentJobByProject.delete(id);
    const message = `[auto] error: ${String(error)}`;
    appendRuntimeLog(id, message);
    autoCompletionByProject.set(id, message);
  });

  return { detail: loadProjectDetail(id), output: `auto started: ${detail.name}` };
}

export function getAutoStatus(id: string): { detail: ProjectDetail; state: ProjectState; current_job: string; completed?: string } {
  const detail = loadProjectDetail(id);
  const completed = autoCompletionByProject.get(id);
  if (completed) {
    autoCompletionByProject.delete(id);
  }
  return {
    detail,
    state: detail.state,
    current_job: autoCurrentJobByProject.get(id) || "",
    completed
  };
}

function setProjectState(id: string, nextState: ProjectState): void {
  const registry = loadRegistry();
  const index = registry.projects.findIndex((project) => project.id === id);
  if (index < 0) return;
  registry.projects[index] = {
    ...registry.projects[index],
    state: nextState,
    updated_at: nowUnix()
  };
  saveRegistry(registry);
}

async function runJobMdSyncWorkflow(id: string, projectPath: string): Promise<string[]> {
  const taskKey = createTaskSessionKey("form-add-input");
  const stages: Array<{ label: string; args: string[] }> = [
    { label: "job.md", args: ["init_orc_job"] },
    { label: "drafts.yaml", args: ["add_orc_drafts"] }
  ];
  const outputs: string[] = [];
  for (const stage of stages) {
    appendRuntimeLog(id, `[form_add_input] ${stage.label} 작업중...`);
    const result = await runOrcStageWithLogs(id, projectPath, stage.args, stage.label, taskKey);
    outputs.push(`${stage.label}: ${result}`);
    appendRuntimeLog(id, `[form_add_input] ${stage.label} 완료`);
  }
  return outputs;
}

function runOrcStageWithLogs(
  id: string,
  projectPath: string,
  args: string[],
  label: string,
  taskKey: string
): Promise<string> {
  return new Promise((resolve, reject) => {
    const command = resolveOrcCommandArgs(args);
    const proc = spawn(command.bin, command.args, {
      cwd: projectPath,
      stdio: ["ignore", "pipe", "pipe"],
      env: buildTaskCommandEnv(taskKey)
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
  if (autoProcessesByProject.has(id)) {
    return { output: `auto already running: ${detail.name}` };
  }
  if (buildProcessesByProject.has(id)) {
    return { output: `build already running: ${detail.name}` };
  }
  const command = resolveOrcCommandArgs(["impl_code_draft"]);
  const taskKey = createTaskSessionKey("build");
  const proc = spawn(command.bin, command.args, {
    cwd: detail.path,
    stdio: ["ignore", "pipe", "pipe"],
    detached: true,
    env: buildTaskCommandEnv(taskKey, {
      ORC_WEB_MANUAL_CHECK: "1"
    })
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
    const message =
      code === 0
        ? "[build] finished: manual rc check pending"
        : `[build] finished: code=${code === null ? "null" : String(code)} signal=${signal ?? "none"}`;
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

export function getBuildStatus(id: string): {
  state: ProjectState;
  current_job: string;
  is_build_running: boolean;
  completed?: string;
} {
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
    is_build_running: buildProcessesByProject.has(id),
    completed
  };
}

export function runManualRcCheck(id: string): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const subject = detail.checkSubject.trim() || detail.name;
  const mission = `manual rc check | ${subject}`;
  appendRuntimeLog(id, `[check] rc start: ${mission}`);
  const command = resolveRcCommandArgs(["clit", "test", "-p", ".", "-m", mission]);
  const result = spawnSync(command.bin, command.args, {
    cwd: detail.path,
    encoding: "utf8"
  });
  const stdout = (result.stdout || "").trim();
  const stderr = (result.stderr || "").trim();
  const movedScreenshots = moveRcScreenshotArtifacts(detail.path);
  for (const screenshotPath of movedScreenshots) {
    appendRuntimeLog(id, `[check] screenshot saved: ${screenshotPath}`);
  }
  if (result.status !== 0) {
    const reason = stderr || stdout || `rc check failed: status=${String(result.status)}`;
    appendRuntimeLog(id, `[check] failed: ${reason}`);
    throw new Error(reason);
  }
  appendRuntimeLog(id, `[check] completed: ${stdout || mission}`);
  return {
    detail: loadProjectDetail(id),
    output: stdout || `rc check completed: ${mission}`
  };
}

export function appendCheckFeedback(
  id: string,
  input: { screenshotPath: string; message: string }
): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const message = input.message.trim();
  if (!message) {
    throw new Error("feedback message is required");
  }
  const screenshotPath = path.resolve(input.screenshotPath.trim());
  const screenshotRoot = path.resolve(screenshotDirPath(detail.path));
  if (!screenshotPath || !screenshotPath.startsWith(`${screenshotRoot}${path.sep}`)) {
    throw new Error("selected screenshot path is invalid");
  }
  const file = ensureFeedbackFile(detail.path);
  const bullet = `- {${screenshotPath}}에서 ${message}`;
  appendMarkdownBullet(file, ["# problems"], bullet);
  appendRuntimeLog(id, `[check-feedback] ${bullet}`);
  return {
    detail: loadProjectDetail(id),
    output: "job.md updated"
  };
}

export function retryFromFeedback(id: string): { detail: ProjectDetail; output: string } {
  const detail = loadProjectDetail(id);
  const feedback = readFeedbackMarkdown(detail.path).trim();
  if (!feedback) {
    throw new Error("job.md # problems or # check not found");
  }
  const instructionFile = ensureInstructionRetryFile(detail.path);
  const instruction = fs.readFileSync(instructionFile, "utf8").trim();
  const prompt = [
    instruction,
    "",
    `project: ${detail.name}`,
    `project_path: ${detail.path}`,
    "",
    "# current verification state:",
    feedback,
    "",
    "반드시 # problems 와 # check 를 기준으로 drafts.yaml을 다시 만들고 병렬 처리 과정을 처음부터 다시 시작한다."
  ].join("\n");
  appendRuntimeLog(id, `[check-retry] instruction loaded: ${instructionFile}`);
  appendRuntimeLog(id, `[check-retry] auto retry start: ${detail.name}`);
  return startAutoFromMessage(id, prompt);
}

export function readCheckScreenshot(id: string, name: string): { body: Buffer; contentType: string } {
  const detail = loadProjectDetail(id);
  const safeName = path.basename(name);
  if (!safeName || safeName !== name) {
    throw new Error("invalid screenshot name");
  }
  const filePath = path.join(screenshotDirPath(detail.path), safeName);
  if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
    throw new Error(`screenshot not found: ${safeName}`);
  }
  return {
    body: fs.readFileSync(filePath),
    contentType: guessMimeType(filePath)
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
