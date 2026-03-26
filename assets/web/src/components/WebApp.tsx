import { useEffect, useMemo, useRef, useState } from "react";
import {
  Ban,
  CheckCircle2,
  ChevronDown,
  Code2,
  CornerUpLeft,
  FileText,
  FlaskConical,
  FolderOpen,
  GraduationCap,
  Menu,
  LayoutGrid,
  List,
  Pencil,
  Plus,
  RefreshCw,
  RotateCcw,
  Search,
  Settings,
  Shapes,
  Sparkles,
  Trash2,
  X
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { DraftYamlItemCard } from "@/components/drafts/DraftYamlItemCard";
import { CodeDraftItem } from "@/components/drafts/code_draft_item";
import { useOrcStore, type Detail, type Project } from "@/store/orc-store";
import { DetailLayoutProvider } from "@/layouts/detail";
import { resolveDraftItemStatus } from "@/lib/draft-item-state";
import { parseRequirementBlocks } from "@/lib/requirement-parser";
import YAML from "yaml";

const sectionLabelClass = "mt-4 mb-2 px-2 text-base font-bold uppercase tracking-wide text-foreground/80 lg:mt-8 lg:mb-3";
const projectContainerItemClass =
  "project-container-item relative rounded-xl border border-border bg-card p-3 text-left text-sm hover:bg-muted/40";
const projectContainerItemMinimalClass =
  "project-container-item-minimal relative rounded-xl border border-border bg-card p-3 text-left text-sm hover:bg-muted/40";
type DraftModalAction = "add_draft" | "impl_draft" | "check_code";
type BrowseTarget = "create" | "load";
type BrowseEntry = { name: string; path: string; hasProjectMeta: boolean };
type ProjectItemViewMode = "card" | "minimal";
type ProfileType = "code" | "mono";
type DraftFormField = { key: string; value: string };
type TemplateAssetFile = { name: string; path: string; content: string };
type CheckScreenshotItem = NonNullable<Detail["screenshots"]>[number];

function stateLabel(state?: Project["state"]): string {
  if (state === "complete") return "complete";
  if (state === "auto") return "auto";
  if (state === "work") return "work";
  if (state === "wait") return "wait";
  return "wait";
}

function stateClass(state?: Project["state"]): string {
  if (state === "complete") return "border-emerald-600/80 bg-emerald-200 text-emerald-900";
  if (state === "auto") return "border-sky-500/70 bg-sky-100 text-sky-800";
  if (state === "work") return "border-orange-500/60 bg-orange-100 text-orange-800";
  if (state === "wait") return "border-zinc-400/70 bg-zinc-100 text-zinc-700";
  return "border-zinc-400/70 bg-zinc-100 text-zinc-700";
}

function projectTypeLabel(type?: Project["project_type"]): string {
  if (type === "mono") return "monorepo";
  return "code";
}

function profileTypeFromProjectType(type?: Project["project_type"]): ProfileType {
  if (type === "mono") return "mono";
  return "code";
}

function ProjectTypeIcon({ type }: { type: Project["project_type"] }) {
  if (type === "mono") return <Shapes className="h-5 w-5 text-muted-foreground" />;
  return <Code2 className="h-5 w-5 text-muted-foreground" />;
}

function parseLines(input: string): string[] {
  return input
    .split("\n")
    .map((v) => v.trim())
    .filter((v) => v.length > 0);
}

function normalizeAutoMessageInput(message: string): string {
  const normalized = message.trim();
  if (!normalized) return "";
  if (normalized === "auto" || normalized === "자동") {
    return "현재 프로젝트를 분석해서 필요한 기능을 자동으로 계획/구현/검증까지 진행해줘.";
  }
  return normalized;
}

function compactPath(path: string): string {
  const normalized = path.trim().replace(/\/+/g, "/");
  const parts = normalized.split("/").filter((v) => v.length > 0);
  if (parts.length === 0) return "/";
  if (parts.length <= 2) return `/${parts.join("/")}`;
  return `/${parts.slice(-2).join("/")}`;
}

function classifyMonorepoKind(projectPath: string, root: string): "app" | "feature" | "template" | "other" {
  const normalizedRoot = root.replace(/\/+$/, "");
  if (projectPath.startsWith(`${normalizedRoot}/apps/`) || projectPath.startsWith(`${normalizedRoot}/app/`)) {
    return "app";
  }
  if (
    projectPath.startsWith(`${normalizedRoot}/packages/features/`) ||
    projectPath.startsWith(`${normalizedRoot}/features/`) ||
    projectPath.startsWith(`${normalizedRoot}/feature/`)
  ) {
    return "feature";
  }
  if (projectPath.startsWith(`${normalizedRoot}/template/`) || projectPath.startsWith(`${normalizedRoot}/templates/`)) {
    return "template";
  }
  return "other";
}

function splitSidebarParent(name: string): { parent: string | null; leaf: string } {
  const parts = name.split("/").filter((v) => v.length > 0);
  if (parts.length <= 1) {
    return { parent: null, leaf: name };
  }
  return {
    parent: parts[0],
    leaf: parts.slice(1).join("/")
  };
}

function apiUrl(path: string): string {
  const base = (import.meta.env.PUBLIC_ORC_API_BASE ?? "").trim().replace(/\/+$/, "");
  return base ? `${base}${path}` : path;
}

function renderEpisodeMarkdown(markdown: string) {
  const lines = markdown.split(/\r?\n/);
  const blocks: Array<{ type: "heading" | "list" | "paragraph"; level?: number; text?: string; items?: string[] }> = [];
  let listBuffer: string[] = [];
  let paragraphBuffer: string[] = [];

  const flushList = () => {
    if (listBuffer.length === 0) return;
    blocks.push({ type: "list", items: listBuffer });
    listBuffer = [];
  };

  const flushParagraph = () => {
    if (paragraphBuffer.length === 0) return;
    blocks.push({ type: "paragraph", text: paragraphBuffer.join(" ").trim() });
    paragraphBuffer = [];
  };

  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    const trimmed = line.trim();
    if (!trimmed) {
      flushList();
      flushParagraph();
      continue;
    }
    const heading = trimmed.match(/^(#{1,6})\s+(.*)$/);
    if (heading) {
      flushList();
      flushParagraph();
      blocks.push({ type: "heading", level: heading[1].length, text: heading[2].trim() });
      continue;
    }
    if (trimmed.startsWith("- ")) {
      flushParagraph();
      listBuffer.push(trimmed.slice(2).trim());
      continue;
    }
    flushList();
    paragraphBuffer.push(trimmed);
  }

  flushList();
  flushParagraph();

  return blocks.map((block, index) => {
    if (block.type === "heading") {
      if (block.level === 1) {
        return (
          <h1 key={`md-${index}`} className="text-3xl font-black tracking-tight text-foreground">
            {block.text}
          </h1>
        );
      }
      if (block.level === 2) {
        return (
          <h2 key={`md-${index}`} className="text-2xl font-bold text-foreground">
            {block.text}
          </h2>
        );
      }
      return (
        <h3 key={`md-${index}`} className="text-xl font-semibold text-foreground">
          {block.text}
        </h3>
      );
    }
    if (block.type === "list") {
      return (
        <ul key={`md-${index}`} className="space-y-2 pl-5 text-base leading-7 text-foreground/85 list-disc">
          {(block.items ?? []).map((item) => (
            <li key={`${index}-${item}`}>{item}</li>
          ))}
        </ul>
      );
    }
    return (
      <p key={`md-${index}`} className="text-base leading-7 text-foreground/85">
        {block.text}
      </p>
    );
  });
}

export default function WebApp() {
  const [createOpenLocal, setCreateOpenLocal] = useState(false);
  const [loadOpen, setLoadOpen] = useState(false);
  const [loadPath, setLoadPath] = useState("");
  const [browseOpen, setBrowseOpen] = useState(false);
  const [browseTarget, setBrowseTarget] = useState<BrowseTarget>("create");
  const [browsePath, setBrowsePath] = useState("/home/tree");
  const [browseParentPath, setBrowseParentPath] = useState<string | null>(null);
  const [browseEntries, setBrowseEntries] = useState<BrowseEntry[]>([]);
  const [browseLoading, setBrowseLoading] = useState(false);
  const [browseError, setBrowseError] = useState("");
  const [browseShowHidden, setBrowseShowHidden] = useState(false);
  const [browseQuery, setBrowseQuery] = useState("");
  const [browseKeyword, setBrowseKeyword] = useState("");
  const [projectSectionType, setProjectSectionType] = useState<Project["project_type"]>("code");
  const [syncingMonorepo, setSyncingMonorepo] = useState(false);
  const [memoDraft, setMemoDraft] = useState("");
  const [memoSaving, setMemoSaving] = useState(false);
  const [runningImplDraft, setRunningImplDraft] = useState(false);
  const [runningImplDraftName, setRunningImplDraftName] = useState("");
  const [draftModalAction, setDraftModalAction] = useState<DraftModalAction | null>(null);
  const [formDraftsRaw, setFormDraftsRaw] = useState("");
  const [draftsRawSaving, setDraftsRawSaving] = useState(false);
  const [addInputStatus, setAddInputStatus] = useState("");
  const [addInputApplying, setAddInputApplying] = useState(false);
  const [deletingRequirementIndex, setDeletingRequirementIndex] = useState<number | null>(null);
  const [requirementModalOpen, setRequirementModalOpen] = useState(false);
  const [requirementModalInput, setRequirementModalInput] = useState("");
  const [jobMessageModalOpen, setJobMessageModalOpen] = useState(false);
  const [jobMessageModalInput, setJobMessageModalInput] = useState("");
  const [jobMessageGenerating, setJobMessageGenerating] = useState(false);
  const [autoModalOpen, setAutoModalOpen] = useState(false);
  const [autoModalInput, setAutoModalInput] = useState("");
  const [autoRunning, setAutoRunning] = useState(false);
  const [domainLoading, setDomainLoading] = useState(false);
  const [domainError, setDomainError] = useState("");
  const [selectedDraftYamlItem, setSelectedDraftYamlItem] = useState<{
    name: string;
    draft: Record<string, unknown>;
  } | null>(null);
  const [checkRunning, setCheckRunning] = useState(false);
  const [checkFeedbackSaving, setCheckFeedbackSaving] = useState(false);
  const [checkRetrying, setCheckRetrying] = useState(false);
  const [checkFeedbackInput, setCheckFeedbackInput] = useState("");
  const [selectedScreenshotPath, setSelectedScreenshotPath] = useState("");
  const [screenshotPreviewItem, setScreenshotPreviewItem] = useState<CheckScreenshotItem | null>(null);
  const [reportOpen, setReportOpen] = useState(false);
  const [buildToast, setBuildToast] = useState("");
  const [draftModalName, setDraftModalName] = useState("edit_code_drafts");
  const [draftFormFields, setDraftFormFields] = useState<DraftFormField[]>([]);
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [templateModalType, setTemplateModalType] = useState<ProfileType>("code");
  const [templateModalLoading, setTemplateModalLoading] = useState(false);
  const [templateSelectedKey, setTemplateSelectedKey] = useState("");
  const [templatePromptsOpen, setTemplatePromptsOpen] = useState(true);
  const [templateTemplatesOpen, setTemplateTemplatesOpen] = useState(true);
  const [templateEditing, setTemplateEditing] = useState(false);
  const [templateEditorValue, setTemplateEditorValue] = useState("");
  const [templateSaving, setTemplateSaving] = useState(false);
  const [templateAssets, setTemplateAssets] = useState<{
    prompts: TemplateAssetFile[];
    templates: TemplateAssetFile[];
  }>({ prompts: [], templates: [] });
  const [projectItemViewMode, setProjectItemViewMode] = useState<ProjectItemViewMode>("card");
  const [bulkDeleteMode, setBulkDeleteMode] = useState(false);
  const [bulkDeleteIds, setBulkDeleteIds] = useState<string[]>([]);
  const [draggingProjectId, setDraggingProjectId] = useState<string>("");
  const [dragOverProjectId, setDragOverProjectId] = useState<string>("");
  const [sidebarFoldOpen, setSidebarFoldOpen] = useState<Record<string, boolean>>({});
  const [sidebarSearch, setSidebarSearch] = useState("");
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [projectInfoSaving, setProjectInfoSaving] = useState(false);
  const [detailListSaving, setDetailListSaving] = useState(false);
  const lastSavedMemoRef = useRef("");
  const codeSectionRef = useRef<HTMLDivElement | null>(null);
  const monorepoSectionRef = useRef<HTMLDivElement | null>(null);
  const templateContentRef = useRef<HTMLDivElement | null>(null);
  const {
    tab,
    projects,
    selectedId,
    detail,
    selectedPane,
    logs,
    newName,
    newDescription,
    newPath,
    newSpec,
    createOpen,
    addDraftPayload,
    editOpen,
    selectedDomain,
    editName,
    editDescription,
    editSpec,
    editGoal,
    editRules,
    editConstraints,
    editFeatures,
    activeRunProjectIds,
    activeAutoProjectIds,
    setTab,
    setProjects,
    setSelectedId,
    setDetail,
    setSelectedPane,
    pushLog,
    setLogs,
    setNewName,
    setNewDescription,
    setNewPath,
    setNewSpec,
    resetNewProjectForm,
    setCreateOpen,
    setAddDraftPayload,
    setEditOpen,
    setSelectedDomain,
    setEditName,
    setEditDescription,
    setEditSpec,
    setEditGoal,
    setEditRules,
    setEditConstraints,
    setEditFeatures,
    setActiveRunProjectIds,
    setActiveAutoProjectIds
  } = useOrcStore();
  const isCreateOpen = createOpen || createOpenLocal;

  const selectedProject = useMemo(
    () => projects.find((p) => p.id === selectedId) ?? null,
    [projects, selectedId]
  );
  const groupedProjects = useMemo(
    () => ({
      code: projects.filter((v) => v.project_type !== "mono"),
      monorepo: projects.filter((v) => v.project_type === "mono")
    }),
    [projects]
  );
  const sidebarMonorepoGroups = useMemo(() => {
    const root = "/home/tree/home";
    const monoProjects = groupedProjects.monorepo;
    return {
      app: monoProjects.filter((p) => classifyMonorepoKind(p.path, root) === "app"),
      feature: monoProjects.filter((p) => classifyMonorepoKind(p.path, root) === "feature"),
      template: monoProjects.filter((p) => classifyMonorepoKind(p.path, root) === "template")
    };
  }, [groupedProjects.monorepo]);

  function visualProjectState(project: Project): Project["state"] {
    if (activeAutoProjectIds.includes(project.id)) return "auto";
    if (project.is_build_running || project.is_dev_running || activeRunProjectIds.includes(project.id)) return "work";
    return project.state;
  }
  const templateSelectedFile = useMemo(() => {
    const selectedPrompt = templateAssets.prompts.find((file) => `prompts:${file.name}` === templateSelectedKey);
    if (selectedPrompt) return selectedPrompt;
    const selectedTemplate = templateAssets.templates.find((file) => `templates:${file.name}` === templateSelectedKey);
    return selectedTemplate ?? null;
  }, [templateAssets, templateSelectedKey]);
  const draftsYamlCards = detail?.draftsYamlItems ?? [];
  const requirementBlocks = useMemo(
    () => parseRequirementBlocks(String(detail?.jobEditableRaw ?? "")),
    [detail?.jobEditableRaw]
  );
  const selectedDraftItemName = selectedDraftYamlItem?.name ?? "";
  const selectedDraftCard = draftsYamlCards.find((item) => item.name === selectedDraftItemName) ?? null;
  const selectedDraftYamlText = selectedDraftYamlItem ? YAML.stringify(selectedDraftYamlItem.draft ?? {}) : "";
  const isSelectedDraftRunning = runningImplDraft && runningImplDraftName === selectedDraftItemName;
  const hasDraftItems = draftsYamlCards.length > 0;
  const hasGreenDraft = draftsYamlCards.some((item) => item.status === "complete");
  const isWorkPaneLocked = !hasDraftItems;
  const isCheckPaneLocked = !hasGreenDraft;
  const checkSubject = detail?.checkSubject?.trim() || "drafts.yaml 기반 수동 check 대상이 없습니다.";
  const checkSteps = detail?.checkSteps ?? [];
  const checkScreenshots = detail?.screenshots ?? [];
  const feedbackMdRaw = detail?.feedbackMdRaw ?? "";
  const selectedCheckScreenshot =
    checkScreenshots.find((item) => item.path === selectedScreenshotPath) ?? null;
  const hasFeedbackReport = feedbackMdRaw.trim().length > 0;

  async function loadProjects() {
    const res = await fetch(apiUrl("/api/projects"));
    const data = await res.json();
    const next: Project[] = data.projects ?? [];
    setProjects(next);
    setActiveAutoProjectIds((prev) => {
      const nextAutoIds = next.filter((project) => project.state === "auto").map((project) => project.id);
      const merged = new Set([...prev, ...nextAutoIds]);
      for (const id of [...merged]) {
        const project = next.find((row) => row.id === id);
        if (!project || project.state !== "auto") {
          merged.delete(id);
        }
      }
      return [...merged];
    });
    setActiveRunProjectIds((prev) => {
      const nextRunIds = next.filter((project) => project.is_dev_running).map((project) => project.id);
      const merged = new Set([...prev, ...nextRunIds]);
      for (const id of [...merged]) {
        if (!next.some((project) => project.id === id)) {
          merged.delete(id);
        }
      }
      for (const project of next) {
        if (!project.is_dev_running && merged.has(project.id) && !prev.includes(project.id)) {
          merged.delete(project.id);
        }
      }
      return [...merged];
    });
    if (!selectedId && next.length > 0) {
      setSelectedId(next.find((p) => p.selected)?.id ?? next[0].id);
    }
  }

  async function syncMonorepo() {
    setSyncingMonorepo(true);
    const res = await fetch(apiUrl("/api/monorepo-sync"), {
      method: "POST",
      headers: { "content-type": "application/json" }
    });
    const data = await res.json();
    setSyncingMonorepo(false);
    if (!res.ok) {
      pushLog(`monorepo sync failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    pushLog(`monorepo synced: created ${Number(data.created ?? 0)}, updated ${Number(data.updated ?? 0)}`);
    await loadProjects();
  }

  function reorderProjectList(current: Project[], fromId: string, toId: string): Project[] {
    const fromIndex = current.findIndex((project) => project.id === fromId);
    const toIndex = current.findIndex((project) => project.id === toId);
    if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return current;
    const next = [...current];
    const [moved] = next.splice(fromIndex, 1);
    next.splice(toIndex, 0, moved);
    return next;
  }

  async function persistProjectOrder(next: Project[]) {
    const res = await fetch(apiUrl("/api/project-reorder"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ ids: next.map((project) => project.id) })
    });
    if (!res.ok) {
      pushLog("project reorder failed");
      await loadProjects();
      return;
    }
    const data = await res.json();
    if (Array.isArray(data.projects)) {
      setProjects(data.projects as Project[]);
    }
  }

  async function dropProjectOn(targetId: string) {
    const sourceId = draggingProjectId;
    setDragOverProjectId("");
    setDraggingProjectId("");
    if (!sourceId || sourceId === targetId) return;
    const source = projects.find((project) => project.id === sourceId);
    const target = projects.find((project) => project.id === targetId);
    if (!source || !target || source.project_type !== target.project_type) return;
    const next = reorderProjectList(projects, sourceId, targetId);
    setProjects(next);
    await persistProjectOrder(next);
  }

  function renderProjectContainerItem(p: Project) {
    const visualState = visualProjectState(p);
    return (
      <div
        key={p.id}
        data-testid={`project-item-${p.id}`}
        className={`${projectContainerItemClass} ${selectedProject?.id === p.id ? "border-primary bg-secondary/70" : ""} ${
          dragOverProjectId === p.id ? "border-dashed border-primary" : ""
        }`}
        draggable
        onDragStart={() => {
          setDraggingProjectId(p.id);
          setDragOverProjectId("");
        }}
        onDragOver={(event) => {
          event.preventDefault();
          if (draggingProjectId && draggingProjectId !== p.id) {
            setDragOverProjectId(p.id);
          }
        }}
        onDragLeave={() => {
          if (dragOverProjectId === p.id) setDragOverProjectId("");
        }}
        onDrop={(event) => {
          event.preventDefault();
          void dropProjectOn(p.id);
        }}
        onDragEnd={() => {
          setDraggingProjectId("");
          setDragOverProjectId("");
        }}
        onClick={() => {
          void markSelected(p.id);
        }}
        onDoubleClick={() => {
          void markSelected(p.id);
          setTab("detail");
        }}
      >
        <div className="mb-1 flex items-center gap-2 pr-16">
          <div className="truncate text-base font-extrabold leading-tight">{p.name}</div>
        </div>
        {selectedProject?.id === p.id && (
          <div className="absolute right-2 top-2 flex items-center gap-1">
            <button
              data-testid="project-item-edit"
              className="rounded p-1 text-muted-foreground hover:bg-muted"
              aria-label="project-item-edit"
              onClick={(e) => {
                e.stopPropagation();
                void openProjectItemEdit(p.id);
              }}
            >
              <Pencil className="h-4 w-4" />
            </button>
            <button
              data-testid="project-item-delete"
              className="rounded p-1 text-muted-foreground hover:bg-muted"
              aria-label="project-item-delete"
              onClick={(e) => {
                e.stopPropagation();
                void removeProject(p.id);
              }}
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </div>
        )}
        <div className="mt-[5px] min-h-[2.25rem] overflow-hidden text-[11px] leading-[1.125rem] text-muted-foreground [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:2]">
          {(p.description ?? "").trim() || "\u00A0"}
        </div>
        <div className="mt-1 min-h-[1.1rem] truncate text-[11px] text-muted-foreground/80">
          {p.current_job ? `{${p.current_job}}` : "\u00A0"}
        </div>
        <div className="mt-2 flex items-center justify-between gap-2">
          <div className="flex items-center gap-1">
            <span className={`rounded-full border px-2 py-1 text-[11px] uppercase tracking-wide ${stateClass(visualState)}`}>
              {stateLabel(visualState)}
            </span>
            {visualState === "auto" && <Sparkles className="h-3.5 w-3.5 text-muted-foreground" />}
            {p.is_dev_running && <FlaskConical className="h-3.5 w-3.5 text-muted-foreground" />}
          </div>
          <div className="flex min-w-0 items-center gap-1 text-[11px] text-muted-foreground/80">
            <FolderOpen className="h-3.5 w-3.5 shrink-0" />
            <span className="truncate">{compactPath(p.path)}</span>
          </div>
        </div>
      </div>
    );
  }

  function renderProjectContainerItemMinimal(p: Project) {
    const checked = bulkDeleteIds.includes(p.id);
    return (
      <div
        key={`project-minimal-${p.id}`}
        data-testid={`project-item-minimal-${p.id}`}
        className={`${projectContainerItemMinimalClass} ${selectedProject?.id === p.id ? "border-primary bg-secondary/70" : ""} ${
          dragOverProjectId === p.id ? "border-dashed border-primary" : ""
        }`}
        draggable
        onDragStart={() => {
          setDraggingProjectId(p.id);
          setDragOverProjectId("");
        }}
        onDragOver={(event) => {
          event.preventDefault();
          if (draggingProjectId && draggingProjectId !== p.id) {
            setDragOverProjectId(p.id);
          }
        }}
        onDragLeave={() => {
          if (dragOverProjectId === p.id) setDragOverProjectId("");
        }}
        onDrop={(event) => {
          event.preventDefault();
          void dropProjectOn(p.id);
        }}
        onDragEnd={() => {
          setDraggingProjectId("");
          setDragOverProjectId("");
        }}
        onClick={() => {
          if (bulkDeleteMode) {
            setBulkDeleteIds((prev) => (prev.includes(p.id) ? prev.filter((id) => id !== p.id) : [...prev, p.id]));
            return;
          }
          void markSelected(p.id);
        }}
        onDoubleClick={() => {
          if (bulkDeleteMode) return;
          void markSelected(p.id);
          setTab("detail");
        }}
      >
        <div className="flex items-center justify-between gap-2">
          {bulkDeleteMode && (
            <input
              type="checkbox"
              checked={checked}
              onChange={() => {
                setBulkDeleteIds((prev) => (prev.includes(p.id) ? prev.filter((id) => id !== p.id) : [...prev, p.id]));
              }}
              onClick={(e) => e.stopPropagation()}
              className="h-4 w-4"
            />
          )}
          <div className="truncate text-sm font-semibold">{p.name}</div>
          <span className={`rounded-full border px-2 py-1 text-[11px] uppercase tracking-wide ${stateClass(visualProjectState(p))}`}>
            {stateLabel(visualProjectState(p))}
          </span>
        </div>
      </div>
    );
  }

  function openCreateFor(type: Project["project_type"]) {
    setProjectSectionType(type);
    setCreateOpen(true);
    setCreateOpenLocal(true);
  }

  function openLoadFor(type: Project["project_type"]) {
    setProjectSectionType(type);
    setLoadOpen(true);
  }

  function scrollToProjectSection(section: "code" | "monorepo") {
    const ref = section === "code" ? codeSectionRef : monorepoSectionRef;
    const run = () => ref.current?.scrollIntoView({ behavior: "smooth", block: "start" });
    if (tab !== "project") {
      setTab("project");
      setTimeout(run, 0);
      return;
    }
    run();
  }

  function renderProjectItemByMode(p: Project) {
    return projectItemViewMode === "minimal" ? renderProjectContainerItemMinimal(p) : renderProjectContainerItem(p);
  }

  const projectItemsContainerClass =
    projectItemViewMode === "minimal" ? "space-y-2" : "grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-5";

  async function loadDetail(id: string) {
    const res = await fetch(apiUrl(`/api/project-detail?id=${encodeURIComponent(id)}`));
    const data = await res.json();
    if (data.detail) {
      setDetail(data.detail);
      const memo = String(data.detail.memo ?? "");
      setMemoDraft(memo);
      lastSavedMemoRef.current = memo;
    }
  }

  async function refreshDomainFeatures(domainName?: string): Promise<boolean> {
    if (!detail) return false;
    setDomainLoading(true);
    setDomainError("");
    try {
      const res = await fetch(apiUrl("/api/domain-refresh"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id, domain: domainName || selectedDomain || undefined })
      });
      const data = await res.json();
      if (!res.ok) {
        const errorMessage = String(data.error ?? "unknown error");
        pushLog(`domain refresh failed: ${errorMessage}`);
        setDomainError(errorMessage);
        return false;
      }
      if (data.detail) {
        setDetail(data.detail);
      } else {
        await loadDetail(detail.id);
      }
      pushLog(String(data.output ?? "domain features synced"));
      return true;
    } catch (error) {
      const errorMessage = String(error);
      pushLog(`domain refresh failed: ${errorMessage}`);
      setDomainError(errorMessage);
      return false;
    } finally {
      setDomainLoading(false);
    }
  }

  useEffect(() => {
    void loadProjects();
    void syncMonorepo();
  }, []);

  useEffect(() => {
    if (selectedId) {
      void loadDetail(selectedId);
    }
  }, [selectedId]);


  useEffect(() => {
    setFormDraftsRaw(String(detail?.draftsYamlRaw ?? ""));
  }, [detail?.id, detail?.jobEditableRaw, detail?.draftsYamlRaw]);

  useEffect(() => {
    if (!detail) {
      return;
    }
    setEditName(detail.name);
    setEditDescription(detail.description);
    setEditSpec(detail.spec);
    setEditGoal(detail.goal);
    setEditRules(detail.rules.join("\n"));
    setEditConstraints(detail.constraints.join("\n"));
    setEditFeatures(detail.features.join("\n"));
  }, [detail?.id, detail?.name, detail?.description, detail?.spec, detail?.goal, detail?.rules, detail?.constraints, detail?.features]);

  useEffect(() => {
    if (tab === "project") {
      void loadProjects();
    }
  }, [tab]);

  useEffect(() => {
    if (tab === "detail") {
      window.scrollTo({ top: 0, behavior: "auto" });
    }
  }, [tab]);

  useEffect(() => {
    if (!detail || detail.project_type !== "code") {
      return;
    }
    const names = detail.domains.map((domain) => domain.name);
    if (names.length === 0) {
      if (selectedDomain) setSelectedDomain("");
      return;
    }
    if (!selectedDomain || !names.includes(selectedDomain)) {
      setSelectedDomain(names[0]);
    }
  }, [detail, selectedDomain, setSelectedDomain]);

  useEffect(() => {
    if (!detail?.id) return;
    if (detail.is_dev_running) {
      setActiveRunProjectIds((prev) => [...new Set([...prev, detail.id])]);
      return;
    }
    setActiveRunProjectIds((prev) => prev.filter((id) => id !== detail.id));
  }, [detail?.id, detail?.is_dev_running]);

  useEffect(() => {
    if (!detail?.id) return;
    if (detail.state === "auto") {
      setActiveAutoProjectIds((prev) => [...new Set([...prev, detail.id])]);
      return;
    }
    setActiveAutoProjectIds((prev) => prev.filter((id) => id !== detail.id));
  }, [detail?.id, detail?.state, setActiveAutoProjectIds]);

  useEffect(() => {
    if (!templateSelectedFile) {
      setTemplateEditorValue("");
      return;
    }
    setTemplateEditorValue(templateSelectedFile.content ?? "");
    setTemplateEditing(false);
    if (templateContentRef.current) {
      templateContentRef.current.scrollTop = 0;
    }
  }, [templateSelectedFile?.name, templateSelectedFile?.content]);

  useEffect(() => {
    if (!detail?.id) return;
    let timer: ReturnType<typeof setInterval> | null = null;
    let disposed = false;
    void (async () => {
      const first = await refreshRuntimeLogs(detail.id);
      if (disposed || first === "missing") return;
      timer = setInterval(() => {
        void refreshRuntimeLogs(detail.id);
      }, 1200);
    })();
    return () => {
      disposed = true;
      if (timer) clearInterval(timer);
    };
  }, [detail?.id]);



  useEffect(() => {
    if (!detail?.id) return;
    if (!detail.is_build_running) return;
    const timer = setInterval(() => {
      void pollBuildStatus(detail.id);
    }, 1200);
    return () => clearInterval(timer);
  }, [detail?.id, detail?.is_build_running]);

  useEffect(() => {
    if (activeRunProjectIds.length === 0) return;
    const timer = setInterval(() => {
      void loadProjects();
    }, 2000);
    return () => clearInterval(timer);
  }, [activeRunProjectIds.length]);

  useEffect(() => {
    if (activeAutoProjectIds.length === 0) return;
    const tick = () => {
      for (const id of activeAutoProjectIds) {
        void pollAutoStatus(id);
      }
    };
    tick();
    const timer = setInterval(tick, 1200);
    return () => clearInterval(timer);
  }, [activeAutoProjectIds.join("|"), selectedId]);

  useEffect(() => {
    if (!selectedDraftYamlItem) return;
    const exists = (detail?.draftsYamlItems ?? []).some((item) => item.name === selectedDraftYamlItem.name);
    if (!exists) {
      setSelectedDraftYamlItem(null);
    }
  }, [detail?.draftsYamlItems, selectedDraftYamlItem]);

  useEffect(() => {
    if (draftsYamlCards.length === 0) {
      if (selectedDraftYamlItem) setSelectedDraftYamlItem(null);
      return;
    }
    if (!selectedDraftYamlItem) {
      const first = draftsYamlCards[0];
      setSelectedDraftYamlItem({ name: first.name, draft: first.draft });
    }
  }, [draftsYamlCards, selectedDraftYamlItem]);

  useEffect(() => {
    if (checkScreenshots.length === 0) {
      setSelectedScreenshotPath("");
      setScreenshotPreviewItem(null);
      return;
    }
    if (!selectedScreenshotPath || !checkScreenshots.some((item) => item.path === selectedScreenshotPath)) {
      setSelectedScreenshotPath(checkScreenshots[0].path);
    }
  }, [checkScreenshots, selectedScreenshotPath]);

  useEffect(() => {
    if (!screenshotPreviewItem) return;
    const next = checkScreenshots.find((item) => item.path === screenshotPreviewItem.path) ?? null;
    if (!next) {
      setScreenshotPreviewItem(null);
    }
  }, [checkScreenshots, screenshotPreviewItem]);

  async function createProject() {
    const res = await fetch(apiUrl("/api/projects"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        name: newName,
        description: newDescription,
        path: newPath,
        spec: newSpec,
        project_type: projectSectionType
      })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`create failed: ${data.error}`);
      return;
    }
    pushLog(`project created: ${data.project.name}`);
    resetNewProjectForm();
    setCreateOpen(false);
    setCreateOpenLocal(false);
    await loadProjects();
    setSelectedId(data.project.id);
  }

  async function loadProjectByPath(createIfMissing = false) {
    const res = await fetch(apiUrl("/api/project-load"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        path: loadPath,
        create_if_missing: createIfMissing,
        project_type: projectSectionType
      })
    });
    const data = await res.json();
    if (!res.ok) {
      const message = String(data.error ?? "");
      if (!createIfMissing && message.includes("PROJECT_META_MISSING")) {
        const ok = window.confirm(".project 폴더가 없습니다. 생성할까요?");
        if (ok) {
          await loadProjectByPath(true);
        }
        return;
      }
      pushLog(`load failed: ${message}`);
      return;
    }
    setLoadOpen(false);
    setLoadPath("");
    pushLog(`project loaded: ${data.project.name}`);
    await loadProjects();
    setSelectedId(data.project.id);
  }

  async function browseDirs(pathValue: string) {
    setBrowseLoading(true);
    setBrowseError("");
    const res = await fetch(apiUrl(`/api/project-browse?path=${encodeURIComponent(pathValue)}`));
    const data = await res.json();
    setBrowseLoading(false);
    if (!res.ok) {
      setBrowseError(String(data.error ?? "browse failed"));
      return;
    }
    setBrowsePath(String(data.currentPath ?? pathValue));
    setBrowseParentPath(data.parentPath ? String(data.parentPath) : null);
    setBrowseEntries(Array.isArray(data.entries) ? data.entries : []);
  }

  function applyBrowsePath(pathValue: string) {
    if (browseTarget === "create") {
      setNewPath(pathValue);
    } else {
      setLoadPath(pathValue);
    }
  }

  function openBrowse(target: BrowseTarget) {
    setBrowseTarget(target);
    setBrowseOpen(true);
    setBrowseQuery("");
    setBrowseKeyword("");
    const seed = (target === "create" ? newPath : loadPath).trim() || "/home/tree";
    void browseDirs(seed);
  }

  async function removeProject(id: string) {
    const res = await fetch(apiUrl("/api/project-delete"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`delete failed: ${data.error}`);
      return;
    }
    pushLog("project deleted");
    setDetail(null);
    setSelectedId("");
    await loadProjects();
  }

  async function removeSelectedProjects() {
    const targets = [...new Set(bulkDeleteIds.map((id) => id.trim()).filter((id) => id.length > 0))];
    if (targets.length === 0) {
      pushLog("project delete skipped: no valid targets");
      return;
    }
    let deleted = 0;
    const failed: Array<{ id: string; error: string }> = [];
    for (const id of targets) {
      try {
        const res = await fetch(apiUrl("/api/project-delete"), {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ id })
        });
        if (res.ok) {
          deleted += 1;
          continue;
        }
        const data = await res.json().catch(() => ({}));
        const error = String((data as { error?: unknown }).error ?? "unknown error");
        failed.push({ id, error });
      } catch (error) {
        failed.push({ id, error: String(error) });
      }
    }
    setDetail(null);
    setSelectedId("");
    if (failed.length > 0) {
      setBulkDeleteIds(failed.map((entry) => entry.id).filter((id) => id.length > 0));
      setBulkDeleteMode(true);
    } else {
      setBulkDeleteIds([]);
      setBulkDeleteMode(false);
    }
    await loadProjects();
    if (failed.length > 0) {
      pushLog(`project deleted: ${deleted}, failed: ${failed.length}`);
      for (const row of failed) {
        pushLog(`delete failed: ${row.id} (${row.error})`);
      }
      return;
    }
    pushLog(`project deleted: ${deleted}`);
  }

  async function startBuildJob() {
    if (!detail) return;
    const res = await fetch(apiUrl("/api/build-start"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id: detail.id })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`build start failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    pushLog(String(data.output ?? "build started"));
    setDetail((prev) => (prev ? { ...prev, state: "work", current_job: "starting", is_build_running: true } : prev));
    await loadProjects();
  }

  async function stopBuildJob() {
    if (!detail) return;
    const res = await fetch(apiUrl("/api/build-stop"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id: detail.id })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`build stop failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    pushLog(String(data.output ?? "build stopped"));
    await loadProjects();
    await loadDetail(detail.id);
  }

  async function pollBuildStatus(id: string) {
    const res = await fetch(apiUrl(`/api/build-status?id=${encodeURIComponent(id)}`));
    const data = await res.json();
    if (!res.ok) {
      return;
    }
    const nextState = data.state as Detail["state"];
    setProjects((prev) =>
      prev.map((project) =>
        project.id === id
          ? {
              ...project,
              state: nextState,
              current_job: String(data.current_job ?? ""),
              is_build_running: Boolean(data.is_build_running)
            }
          : project
      )
    );
    setDetail((prev) =>
      prev && prev.id === id
        ? {
            ...prev,
            state: nextState,
            current_job: String(data.current_job ?? ""),
            is_build_running: Boolean(data.is_build_running)
          }
        : prev
    );
    if (typeof data.completed === "string" && data.completed.length > 0) {
      setBuildToast(data.completed);
      setTimeout(() => setBuildToast(""), 3200);
      await loadProjects();
      if (detail?.id === id) {
        await loadDetail(id);
      }
    }
  }

  async function runManualCheck() {
    if (!detail) return;
    setCheckRunning(true);
    try {
      const res = await fetch(apiUrl("/api/check-run"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`manual check failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      pushLog(String(data.output ?? "manual check completed"));
      setDetail(data.detail);
      await loadProjects();
    } finally {
      setCheckRunning(false);
    }
  }

  async function appendScreenshotFeedback() {
    if (!detail || !selectedCheckScreenshot) return;
    const message = checkFeedbackInput.trim();
    if (!message) {
      pushLog("feedback add failed: message is empty");
      return;
    }
    setCheckFeedbackSaving(true);
    try {
      const res = await fetch(apiUrl("/api/check-feedback"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          screenshotPath: selectedCheckScreenshot.path,
          message
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`feedback add failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      pushLog(String(data.output ?? "job.md updated"));
      setCheckFeedbackInput("");
      setDetail(data.detail);
    } finally {
      setCheckFeedbackSaving(false);
    }
  }

  async function retryFromFeedback() {
    if (!detail) return;
    setCheckRetrying(true);
    try {
      const res = await fetch(apiUrl("/api/check-retry"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`feedback retry failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      pushLog(String(data.output ?? "feedback retry started"));
      setReportOpen(false);
      setDetail(data.detail);
      await loadProjects();
    } finally {
      setCheckRetrying(false);
    }
  }

  function composeDraftPayload(fields: DraftFormField[]): string {
    return fields
      .map((field) => `${field.key}: ${field.value}`.trimEnd())
      .join("\n")
      .trim();
  }

  async function openDraftEditorModal() {
    const profile = profileTypeFromProjectType(detail?.project_type ?? selectedProject?.project_type);
    const res = await fetch(apiUrl(`/api/draft-form?type=${encodeURIComponent(profile)}`));
    const data = await res.json();
    if (!res.ok || !data.draft) {
      pushLog(`draft form load failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    const fields: DraftFormField[] = Array.isArray(data.draft.fields)
      ? data.draft.fields.map((row: { key?: unknown; value?: unknown }) => ({
          key: String(row.key ?? ""),
          value: String(row.value ?? "")
        }))
      : [];
    setDraftModalName(String(data.draft.modalName ?? `edit_${profile}_drafts`));
    setDraftFormFields(fields);
    setAddDraftPayload(composeDraftPayload(fields));
    setDraftModalAction("add_draft");
  }

  function updateDraftField(index: number, value: string) {
    setDraftFormFields((prev) => {
      const next = [...prev];
      if (!next[index]) return prev;
      next[index] = { ...next[index], value };
      setAddDraftPayload(composeDraftPayload(next));
      return next;
    });
  }

  async function openTemplateAssetsModal(type: ProfileType) {
    setTemplateModalType(type);
    setTemplateModalOpen(true);
    setTemplateModalLoading(true);
    const res = await fetch(apiUrl(`/api/profile-assets?type=${encodeURIComponent(type)}`));
    const data = await res.json();
    setTemplateModalLoading(false);
    if (!res.ok || !data.assets) {
      pushLog(`template assets load failed: ${String(data.error ?? "unknown error")}`);
      setTemplateAssets({ prompts: [], templates: [] });
      setTemplateSelectedKey("");
      return;
    }
    const prompts: TemplateAssetFile[] = Array.isArray(data.assets.prompts) ? data.assets.prompts : [];
    const templates: TemplateAssetFile[] = Array.isArray(data.assets.templates) ? data.assets.templates : [];
    setTemplateAssets({
      prompts,
      templates
    });
    if (prompts.length > 0) {
      setTemplateSelectedKey(`prompts:${prompts[0].name}`);
    } else if (templates.length > 0) {
      setTemplateSelectedKey(`templates:${templates[0].name}`);
    } else {
      setTemplateSelectedKey("");
    }
    setTemplatePromptsOpen(true);
    setTemplateTemplatesOpen(true);
    setTemplateEditing(false);
    setTemplateEditorValue("");
  }

  function parseTemplateSelectedKey(key: string): { section: "prompts" | "templates"; name: string } | null {
    if (!key.includes(":")) return null;
    const [section, ...rest] = key.split(":");
    if (section !== "prompts" && section !== "templates") return null;
    const name = rest.join(":").trim();
    if (!name) return null;
    return { section, name };
  }

  function selectTemplateAsset(key: string) {
    setTemplateSelectedKey(key);
    setTemplateEditing(false);
    setTimeout(() => {
      if (templateContentRef.current) {
        templateContentRef.current.scrollTop = 0;
      }
    }, 0);
  }

  async function saveTemplateAsset() {
    const selectedMeta = parseTemplateSelectedKey(templateSelectedKey);
    if (!selectedMeta) return;
    setTemplateSaving(true);
    const res = await fetch(apiUrl("/api/profile-asset-update"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        type: templateModalType,
        section: selectedMeta.section,
        name: selectedMeta.name,
        content: templateEditorValue
      })
    });
    const data = await res.json();
    setTemplateSaving(false);
    if (!res.ok) {
      pushLog(`template update failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    pushLog(String(data.output ?? "template updated"));
    await openTemplateAssetsModal(templateModalType);
    selectTemplateAsset(`${selectedMeta.section}:${selectedMeta.name}`);
  }

  async function runAction(action: DraftModalAction, targetDraftName?: string): Promise<boolean> {
    if (!detail) return false;
    if (action === "add_draft" && !detail.hasDraftsYaml) {
      pushLog("add_draft blocked: drafts.yaml not found");
      return false;
    }
    const isImpl = action === "impl_draft";
    if (isImpl) {
      setRunningImplDraft(true);
      setRunningImplDraftName(targetDraftName ?? "");
    }
    try {
      const res = await fetch(apiUrl("/api/run"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          action,
          payload: action === "add_draft" ? addDraftPayload : ""
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`action failed: ${data.error}`);
        return false;
      }
      pushLog(data.output);
      await loadDetail(detail.id);
      return true;
    } finally {
      if (isImpl) {
        setRunningImplDraft(false);
        setRunningImplDraftName("");
      }
    }
  }

  async function runQuickAction(action: "check_code" | "retry_incomplete" | "finalize_complete") {
    if (!detail) return;
    if (!detail.hasDraftsYaml) {
      pushLog(`${action} blocked: drafts.yaml not found`);
      return;
    }
    const res = await fetch(apiUrl("/api/run"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        id: detail.id,
        action,
        payload: ""
      })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`action failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    pushLog(String(data.output ?? `${action} completed`));
    await loadProjects();
    await loadDetail(detail.id);
  }

  async function removeDraftPaneFile(target: "job" | "drafts") {
    if (!detail) return;
    const targetLabel = target === "job" ? "job.md" : "drafts.yaml";
    const answer = window.prompt(`${targetLabel} 파일을 삭제합니다. 진행하려면 y 를 입력하세요. (y/n)`, "n");
    if (!answer || answer.trim().toLowerCase() !== "y") {
      pushLog(`${targetLabel} delete cancelled`);
      return;
    }
    const res = await fetch(apiUrl("/api/drafts-pane-file-delete"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id: detail.id, target })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`delete failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    pushLog(String(data.output ?? `${targetLabel} deleted`));
    setDetail(data.detail);
    if (target === "drafts") {
      setSelectedDraftYamlItem(null);
    }
  }

  async function saveRawDraftsYaml(nextRaw: string) {
    if (!detail) return;
    setDraftsRawSaving(true);
    try {
      const res = await fetch(apiUrl("/api/drafts-yaml-raw"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id, raw: nextRaw })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`drafts.yaml save failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      setDetail(data.detail);
      setFormDraftsRaw(String(data.detail?.draftsYamlRaw ?? ""));
      pushLog("drafts.yaml saved");
    } finally {
      setDraftsRawSaving(false);
    }
  }

  async function submitRequirementBlocks() {
    if (!detail) return;
    const raw = requirementModalInput.trim();
    if (!raw) {
      pushLog("requirement add failed: input is empty");
      return;
    }
    setAddInputApplying(true);
    setAddInputStatus("요구사항 반영중...");
    try {
      const res = await fetch(apiUrl("/api/form-add-input"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id, raw })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`requirement add failed: ${String(data.error ?? "unknown error")}`);
        setAddInputStatus("실패");
        return;
      }
      if (Array.isArray(data.stages)) {
        for (const line of data.stages) pushLog(`[job->drafts] ${String(line)}`);
      }
      setDetail(data.detail);
      setFormDraftsRaw(String(data.detail?.draftsYamlRaw ?? ""));
      setRequirementModalOpen(false);
      setRequirementModalInput("");
      setAddInputStatus("완료");
      await loadProjects();
    } finally {
      setAddInputApplying(false);
    }
  }

  async function deleteRequirementBlock(index: number) {
    if (!detail) return;
    if (index < 0) return;
    setDeletingRequirementIndex(index);
    try {
      const res = await fetch(apiUrl("/api/requirement-item-delete"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id, index })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`requirement delete failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      setDetail(data.detail);
      setFormDraftsRaw(String(data.detail?.draftsYamlRaw ?? ""));
      pushLog(String(data.output ?? "requirement removed"));
      await loadProjects();
    } finally {
      setDeletingRequirementIndex(null);
    }
  }

  async function submitMessageJobGenerate() {
    if (!detail) return;
    const message = jobMessageModalInput.trim();
    if (!message) {
      pushLog("job generate failed: input is empty");
      return;
    }
    setJobMessageGenerating(true);
    try {
      const res = await fetch(apiUrl("/api/job-md-generate"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id, message })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`job generate failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      setDetail(data.detail);
      setFormDraftsRaw(String(data.detail?.draftsYamlRaw ?? ""));
      setJobMessageModalOpen(false);
      setJobMessageModalInput("");
      pushLog(String(data.output ?? "job.md generated"));
      await loadProjects();
    } finally {
      setJobMessageGenerating(false);
    }
  }

  async function syncDraftsFromJobRequirements() {
    if (!detail) return;
    if (requirementBlocks.length === 0) {
      pushLog("draft sync failed: no requirement blocks");
      return;
    }
    setJobMessageGenerating(true);
    try {
      const res = await fetch(apiUrl("/api/drafts-sync-from-job"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id: detail.id })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`draft sync failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      setDetail(data.detail);
      setFormDraftsRaw(String(data.detail?.draftsYamlRaw ?? ""));
      pushLog(String(data.output ?? "drafts.yaml synced from job.md requirements"));
      await loadProjects();
    } finally {
      setJobMessageGenerating(false);
    }
  }

  function syncProjectRuntimeState(id: string, state: Project["state"], currentJob = "") {
    setProjects((prev) =>
      prev.map((project) =>
        project.id === id
          ? {
              ...project,
              state,
              current_job: currentJob
            }
          : project
      )
    );
  }

  async function runAutoFlowFromMessage() {
    if (!detail) return;
    const targetId = detail.id;
    const message = normalizeAutoMessageInput(autoModalInput);
    if (!message) {
      pushLog("auto run failed: message is empty");
      return;
    }
    setAutoModalOpen(false);
    setAutoModalInput("");
    setAutoRunning(true);
    setActiveAutoProjectIds((prev) => [...new Set([...prev, targetId])]);
    setDetail((prev) => (prev && prev.id === targetId ? { ...prev, state: "auto", current_job: "starting" } : prev));
    syncProjectRuntimeState(targetId, "auto", "starting");
    try {
      const res = await fetch(apiUrl("/api/auto-run"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: targetId,
          message
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`auto run failed: ${String(data.error ?? "unknown error")}`);
        setActiveAutoProjectIds((prev) => prev.filter((id) => id !== targetId));
        await loadProjects();
        if (selectedId === targetId) {
          await loadDetail(targetId);
        }
        return;
      }
      if (data.detail) {
        setDetail((prev) => (prev && prev.id === targetId ? data.detail : prev));
        syncProjectRuntimeState(
          targetId,
          (data.detail.state as Project["state"]) ?? "auto",
          String(data.detail.current_job ?? "starting")
        );
      }
      pushLog(String(data.output ?? "auto started"));
    } finally {
      setAutoRunning(false);
    }
  }

  function focusDraftsRawEditor() {
    const editor = document.querySelector<HTMLTextAreaElement>('[data-testid="drafts-raw-editor"]');
    editor?.focus();
  }

  async function pollAutoStatus(id: string): Promise<void> {
    const res = await fetch(apiUrl(`/api/auto-status?id=${encodeURIComponent(id)}`));
    const text = await res.text();
    let data: { detail?: Detail; state?: Project["state"]; current_job?: string; completed?: string; error?: unknown } = {};
    try {
      data = JSON.parse(text) as { detail?: Detail; state?: Project["state"]; current_job?: string; completed?: string; error?: unknown };
    } catch {
      pushLog("auto status failed: invalid api response");
      return;
    }
    if (!res.ok) {
      pushLog(`auto status failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    const nextState = data.state ?? data.detail?.state ?? "wait";
    const currentJob = String(data.current_job ?? data.detail?.current_job ?? "");
    if (data.detail && selectedId === id) {
      setDetail(data.detail);
    }
    syncProjectRuntimeState(id, nextState, currentJob);
    setActiveAutoProjectIds((prev) =>
      nextState === "auto" ? [...new Set([...prev, id])] : prev.filter((projectId) => projectId !== id)
    );
    if (data.completed) {
      pushLog(data.completed);
      await loadProjects();
      if (selectedId === id) {
        await loadDetail(id);
      }
    }
  }

  async function runDevServer() {
    if (!detail) return;
    const res = await fetch(apiUrl("/api/run-dev"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id: detail.id })
    });
    if (res.status === 404) {
      pushLog("run dev endpoint not found: start orc web api server and set PUBLIC_ORC_API_BASE");
      return;
    }
    const text = await res.text();
    let data: { output?: unknown; error?: unknown; running?: unknown; url?: unknown } = {};
    try {
      data = JSON.parse(text) as { output?: unknown; error?: unknown; running?: unknown; url?: unknown };
    } catch {
      pushLog("run dev failed: invalid api response");
      return;
    }
    if (!res.ok) {
      pushLog(`run dev failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    pushLog(String(data.output ?? "bun run dev started"));
    setDetail((prev) => {
      if (!prev) return prev;
      if (typeof data.running !== "boolean") return prev;
      return {
        ...prev,
        state: prev.state,
        is_dev_running: data.running,
        dev_server_url: data.running && typeof data.url === "string" ? data.url : undefined
      };
    });
    setActiveRunProjectIds((prev) =>
      typeof data.running === "boolean"
        ? data.running
          ? [...new Set([...prev, detail.id])]
          : prev.filter((id) => id !== detail.id)
        : prev
    );
    await loadProjects();
    await loadDetail(detail.id);
  }

  async function refreshRuntimeLogs(id: string): Promise<"ok" | "missing" | "error"> {
    const res = await fetch(apiUrl(`/api/runtime-log?id=${encodeURIComponent(id)}`));
    if (res.status === 404) {
      return "missing";
    }
    const text = await res.text();
    let data: { logs?: unknown; error?: unknown } = {};
    try {
      data = JSON.parse(text) as { logs?: unknown; error?: unknown };
    } catch {
      return "error";
    }
    if (!res.ok) {
      pushLog(`runtime log failed: ${String(data.error ?? "unknown error")}`);
      return "error";
    }
    const next = Array.isArray(data.logs) ? data.logs.map((v: unknown) => String(v)) : [];
    setLogs(next);
    return "ok";
  }

  async function saveMemoContent(content: string) {
    if (!detail) return;
    setMemoSaving(true);
    const res = await fetch(apiUrl("/api/project-memo"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id: detail.id, memo: content })
    });
    const data = await res.json();
    setMemoSaving(false);
    if (!res.ok) {
      pushLog(`memo save failed: ${data.error}`);
      return;
    }
    setDetail(data.detail);
    lastSavedMemoRef.current = content;
  }

  function updateMemoRealtime(value: string) {
    setMemoDraft(value);
    if (detail) {
      setDetail({ ...detail, memo: value });
    }
  }

  function flushMemo() {
    if (!detail) return;
    if (memoDraft === lastSavedMemoRef.current) return;
    void saveMemoContent(memoDraft);
  }

  async function markSelected(id: string) {
    const res = await fetch(apiUrl("/api/project-select"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ id })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`select failed: ${data.error}`);
      return;
    }
    setSelectedId(id);
    pushLog(`selected project: ${data.project.name}`);
    await loadProjects();
  }

  async function openProjectItemEdit(id: string) {
    const res = await fetch(apiUrl(`/api/project-detail?id=${encodeURIComponent(id)}`));
    const data = await res.json();
    if (!res.ok || !data.detail) {
      pushLog(`load project detail failed: ${data.error ?? "unknown error"}`);
      return;
    }
    setSelectedId(id);
    setSelectedPane("project_info");
    setEditName(data.detail.name ?? "");
    setEditDescription(data.detail.description ?? "");
    setEditSpec(data.detail.spec ?? "");
    setEditGoal(data.detail.goal ?? "");
    setEditOpen(true);
  }

  function openEditor() {
    if (!detail) {
      return;
    }
    if (selectedPane === "project_info") {
      setEditName(detail.name);
      setEditDescription(detail.description);
      setEditSpec(detail.spec);
      setEditGoal(detail.goal);
    } else if (selectedPane === "rules") {
      setEditRules(detail.rules.join("\n"));
    } else if (selectedPane === "constraints") {
      setEditConstraints(detail.constraints.join("\n"));
    } else {
      setEditFeatures(detail.features.join("\n"));
    }
    setEditOpen(true);
  }

  async function saveEditor() {
    if (!detail) return;

    if (selectedPane === "project_info") {
      const res = await fetch(apiUrl("/api/project-info"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          name: editName,
          description: editDescription,
          spec: editSpec,
          goal: editGoal
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`save info failed: ${data.error}`);
        return;
      }
      setDetail(data.detail);
      pushLog("project info saved");
      await loadProjects();
    } else {
      const nextRules = selectedPane === "rules" ? parseLines(editRules) : detail.rules;
      const nextConstraints =
        selectedPane === "constraints" ? parseLines(editConstraints) : detail.constraints;
      const nextFeatures = selectedPane === "features" ? parseLines(editFeatures) : detail.features;
      const res = await fetch(apiUrl("/api/project-lists"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          rules: nextRules,
          constraints: nextConstraints,
          features: nextFeatures
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`save lists failed: ${data.error}`);
        return;
      }
      setDetail(data.detail);
      pushLog("rules/constraints/features saved");
    }
    setEditOpen(false);
  }

  async function saveProjectInfo() {
    if (!detail) return;
    setProjectInfoSaving(true);
    try {
      const res = await fetch(apiUrl("/api/project-info"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          name: editName,
          description: editDescription,
          spec: editSpec,
          goal: editGoal
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`save info failed: ${data.error}`);
        return;
      }
      setDetail(data.detail);
      pushLog("project info saved");
      await loadProjects();
    } catch (error) {
      pushLog(`save info failed: ${String(error)}`);
    } finally {
      setProjectInfoSaving(false);
    }
  }

  async function saveListPane(pane: "rules" | "constraints" | "features") {
    if (!detail) return;
    setDetailListSaving(true);
    try {
      const nextRules = pane === "rules" ? parseLines(editRules) : detail.rules;
      const nextConstraints = pane === "constraints" ? parseLines(editConstraints) : detail.constraints;
      const nextFeatures = pane === "features" ? parseLines(editFeatures) : detail.features;
      const res = await fetch(apiUrl("/api/project-lists"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          rules: nextRules,
          constraints: nextConstraints,
          features: nextFeatures
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`save lists failed: ${data.error}`);
        return;
      }
      setDetail(data.detail);
      pushLog("rules/constraints/features saved");
    } catch (error) {
      pushLog(`save lists failed: ${String(error)}`);
    } finally {
      setDetailListSaving(false);
    }
  }

  useEffect(() => {
    if (!detail) {
      return;
    }
    if (memoDraft === lastSavedMemoRef.current) {
      return;
    }
    const timer = setTimeout(() => {
      void saveMemoContent(memoDraft);
    }, 450);
    return () => clearTimeout(timer);
  }, [detail?.id, memoDraft]);

  const isBuildRunning = Boolean(detail?.is_build_running);
  const isDevRunning = Boolean(detail?.is_dev_running) || (!!detail?.id && activeRunProjectIds.includes(detail.id));
  const isReviewState = hasGreenDraft;
  const isAutoRunningDetail = detail?.state === "auto" || (!!detail?.id && activeAutoProjectIds.includes(detail.id));
  const isAiBusy = jobMessageGenerating || autoRunning || isAutoRunningDetail;
  const canRunManualCheck =
    Boolean(detail?.id) &&
    Boolean(detail?.hasDraftsYaml) &&
    !isBuildRunning &&
    !isAutoRunningDetail &&
    !checkRunning;
  const canAppendCheckFeedback =
    Boolean(detail?.id) &&
    Boolean(selectedCheckScreenshot) &&
    checkFeedbackInput.trim().length > 0 &&
    !checkFeedbackSaving &&
    !checkRunning;
  const detailActionLocked = addInputApplying || isAiBusy;
  const detailVisualState: Project["state"] = isAutoRunningDetail ? "auto" : isBuildRunning || isDevRunning ? "work" : detail?.state;
  const detailDevServerUrl = detail?.dev_server_url;

  function renderSidebarProjectList(items: Array<Pick<Project, "id" | "name">>, groupKey: string) {
    const search = sidebarSearch.trim().toLowerCase();
    const sourceItems =
      search.length === 0
        ? items
        : items.filter((item) => item.name.toLowerCase().includes(search));
    const parentMap = new Map<string, Array<Pick<Project, "id" | "name">>>();
    const plainItems: Array<Pick<Project, "id" | "name">> = [];
    for (const item of sourceItems) {
      const split = splitSidebarParent(item.name);
      if (!split.parent) {
        plainItems.push(item);
        continue;
      }
      const current = parentMap.get(split.parent) ?? [];
      current.push({ ...item, name: split.leaf });
      parentMap.set(split.parent, current);
    }
    const parentRows = [...parentMap.entries()].sort((a, b) => a[0].localeCompare(b[0]));
    return (
      <div className="mt-1 space-y-1">
        {plainItems.map((p) => (
          <button
            key={`detail-sidebar-plain-${groupKey}-${p.id}`}
            className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
              selectedProject?.id === p.id ? "bg-muted font-semibold text-foreground" : "text-muted-foreground hover:bg-muted/50"
            }`}
            onClick={() => {
              void markSelected(p.id);
              setMobileSidebarOpen(false);
            }}
          >
            {p.name}
          </button>
        ))}
        {parentRows.map(([parent, children]) => {
          const foldKey = `${groupKey}:${parent}`;
          const opened = sidebarFoldOpen[foldKey] ?? true;
          return (
            <div key={`detail-sidebar-parent-${foldKey}`} className="space-y-1">
              <button
                type="button"
                className="flex w-full items-center gap-1 rounded-lg px-2 py-1 text-left text-xs font-semibold uppercase tracking-wide text-muted-foreground hover:bg-muted/50"
                onClick={() =>
                  setSidebarFoldOpen((prev) => ({
                    ...prev,
                    [foldKey]: !opened
                  }))
                }
              >
                {opened ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
                <span>{parent}</span>
              </button>
              {opened && (
                <div className="space-y-1 pl-2">
                  {children.map((p) => (
                    <button
                      key={`detail-sidebar-child-${groupKey}-${p.id}`}
                      className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                        selectedProject?.id === p.id
                          ? "bg-muted font-semibold text-foreground"
                          : "text-muted-foreground hover:bg-muted/50"
                      }`}
                      onClick={() => {
                        void markSelected(p.id);
                        setMobileSidebarOpen(false);
                      }}
                    >
                      {p.name}
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    );
  }

  return (
    <>
      <div className="fixed inset-x-0 top-0 z-50 border-b border-border bg-background/70 backdrop-blur-md">
        <div className="mx-auto flex max-w-[1500px] items-center justify-between px-4 py-3">
          <div className="flex items-center gap-3">
            <span className="rounded-full border border-border px-2 py-1 text-[11px] font-bold uppercase tracking-[0.16em] text-foreground/80">
              ORC
            </span>
            <button
              className="text-sm font-semibold text-muted-foreground hover:text-foreground"
              onClick={() => scrollToProjectSection("code")}
            >
              code
            </button>
            <button
              className="text-sm font-semibold text-muted-foreground hover:text-foreground"
              onClick={() => scrollToProjectSection("monorepo")}
            >
              monorepo
            </button>
          </div>
          <div className="flex items-center gap-2">
            <Button
              data-testid="tab-project"
              variant="outline"
              className={`border-0 bg-transparent px-2 font-bold shadow-none hover:bg-transparent ${
                tab === "project" ? "text-foreground/70" : "text-muted-foreground/70"
              }`}
              onClick={() => setTab("project")}
            >
              project
            </Button>
            <Button
              data-testid="tab-detail"
              variant="outline"
              className={`border-0 bg-transparent px-2 font-bold shadow-none hover:bg-transparent ${
                tab === "detail" ? "text-foreground/70" : "text-muted-foreground/70"
              }`}
              onClick={() => setTab("detail")}
            >
              detail
            </Button>
          </div>
        </div>
      </div>
    <main className="mx-auto max-w-[1500px] space-y-4 p-4 pt-20">

      {tab === "project" ? (
        <div className="space-y-4 bg-background">
          <div className="flex items-center justify-end gap-2">
            <Button
              size="sm"
              variant={projectItemViewMode === "card" ? "default" : "outline"}
              aria-label="project-item-view-card"
              onClick={() => {
                setProjectItemViewMode("card");
                setBulkDeleteMode(false);
                setBulkDeleteIds([]);
              }}
            >
              <LayoutGrid className="h-4 w-4" />
              <span className="ml-2">card</span>
            </Button>
            <Button
              size="sm"
              variant={projectItemViewMode === "minimal" ? "default" : "outline"}
              aria-label="project-item-view-minimal"
              onClick={() => setProjectItemViewMode("minimal")}
            >
              <List className="h-4 w-4" />
              <span className="ml-2">list</span>
            </Button>
          </div>
          <div className={projectItemViewMode === "minimal" ? "grid grid-cols-1 gap-4 xl:grid-cols-4" : "space-y-4"}>
          <div ref={codeSectionRef}>
            <Card className="project-container-pane rounded-2xl">
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <Code2 className="h-4 w-4" />
                <span>Code</span>
              </CardTitle>
              <div className="flex items-center gap-2">
                <Button
                  data-testid="open-create-project"
                  size="sm"
                  variant="outline"
                  onClick={() => openCreateFor("code")}
                  aria-label="create-project"
                >
                  <Plus className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => openLoadFor("code")} aria-label="load-project">
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void openTemplateAssetsModal("code")} aria-label="open-code-template-assets">
                  <Settings className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void loadProjects()} aria-label="refresh-projects">
                  <RefreshCw className="h-4 w-4" />
                </Button>
                {projectItemViewMode === "minimal" && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setBulkDeleteMode((prev) => !prev);
                      setBulkDeleteIds([]);
                    }}
                    aria-label="toggle-delete-mode-code"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className={projectItemsContainerClass}>
                {groupedProjects.code.map((p) => renderProjectItemByMode(p))}
              </div>
              {groupedProjects.code.length === 0 && <div className="text-xs text-muted-foreground">no code projects</div>}
            </CardContent>
          </Card>
          </div>
          <div ref={monorepoSectionRef}>
          <Card className="project-container-pane rounded-2xl">
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <Shapes className="h-4 w-4" />
                <span>Monorepo</span>
              </CardTitle>
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={() => openCreateFor("mono")} aria-label="create-monorepo-project">
                  <Plus className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => openLoadFor("mono")} aria-label="load-monorepo-project">
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void openTemplateAssetsModal("mono")} aria-label="open-mono-template-assets">
                  <Settings className="h-4 w-4" />
                </Button>
                {projectItemViewMode === "minimal" && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setBulkDeleteMode((prev) => !prev);
                      setBulkDeleteIds([]);
                    }}
                    aria-label="toggle-delete-mode-monorepo"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void syncMonorepo()}
                  disabled={syncingMonorepo}
                  aria-label="sync-monorepo-projects"
                >
                  <RefreshCw className={`h-4 w-4 ${syncingMonorepo ? "animate-spin" : ""}`} />
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="space-y-3">
                <div>
                  <div className="mb-2 text-xs font-bold uppercase tracking-wide text-muted-foreground">app</div>
                  <div className={projectItemsContainerClass}>
                    {sidebarMonorepoGroups.app.map((p) => renderProjectItemByMode(p))}
                    {sidebarMonorepoGroups.app.length === 0 && <div className="text-xs text-muted-foreground">no app packages</div>}
                  </div>
                </div>
                <div>
                  <div className="mb-2 text-xs font-bold uppercase tracking-wide text-muted-foreground">feature</div>
                  <div className={projectItemsContainerClass}>
                    {sidebarMonorepoGroups.feature.map((p) => renderProjectItemByMode(p))}
                    {sidebarMonorepoGroups.feature.length === 0 && <div className="text-xs text-muted-foreground">no feature packages</div>}
                  </div>
                </div>
                <div>
                  <div className="mb-2 text-xs font-bold uppercase tracking-wide text-muted-foreground">templates</div>
                  <div className={projectItemsContainerClass}>
                    {sidebarMonorepoGroups.template.map((p) => renderProjectItemByMode(p))}
                    {sidebarMonorepoGroups.template.length === 0 && <div className="text-xs text-muted-foreground">no template packages</div>}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
          </div>
          </div>
          {projectItemViewMode === "minimal" && bulkDeleteMode && (
            <div className="fixed bottom-4 left-1/2 z-40 -translate-x-1/2">
              <Button
                variant="destructive"
                onClick={() => void removeSelectedProjects()}
                disabled={bulkDeleteIds.length === 0}
                aria-label="delete-selected-projects"
              >
                삭제하기 ({bulkDeleteIds.length})
              </Button>
            </div>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          <div
            data-testid="detail-pane-project"
            className="relative border-b border-border px-2 pb-7 pt-1"
            onClick={() => setSelectedPane("project_info")}
          >
            <div className="flex items-start gap-4">
              <div className="min-w-0">
                <div data-testid="detail-project-name" className="text-5xl font-extrabold tracking-tight text-foreground">
                  {detail?.name ?? selectedProject?.name ?? ""}
                </div>
                <div className="my-3 text-sm text-muted-foreground">{detail?.description ?? selectedProject?.description ?? ""}</div>
                <div className="mt-2 flex items-center gap-2">
                  <span className="rounded-md border border-border px-2 py-1 text-xs font-semibold text-foreground/80">
                    {projectTypeLabel(selectedProject?.project_type)}
                  </span>
                  <span className={`rounded-full border px-2 py-1 text-[11px] uppercase tracking-wide ${stateClass(detailVisualState)}`}>
                    {stateLabel(detailVisualState)}
                  </span>
                </div>
              </div>
            </div>
            <div
              data-testid="pane-actions"
              className="mt-3 rounded lg:absolute lg:bottom-2 lg:right-0 lg:mt-0"
              onClick={(e) => e.stopPropagation()}
              aria-label="detail-actions"
            >
              <div className="flex flex-col items-end gap-1 py-1">
                {isDevRunning && detailDevServerUrl && (
                  <a
                    href={detailDevServerUrl}
                    target="_blank"
                    rel="noreferrer"
                    className="text-xs font-medium text-blue-600 underline underline-offset-2 hover:text-blue-700"
                  >
                    {detailDevServerUrl}
                  </a>
                )}
                {isAutoRunningDetail && (
                  <div data-testid="detail-auto-indicator" className="text-xs font-semibold text-sky-700">
                    auto 중{detail?.current_job ? ` · ${detail.current_job}` : ""}
                  </div>
                )}
                {addInputStatus && <div className="text-xs text-muted-foreground">{addInputStatus}</div>}
                <div className="flex w-full items-center justify-end gap-2 overflow-x-auto whitespace-nowrap">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="relative z-10 h-10 shrink-0 lg:hidden"
                  onClick={() => setMobileSidebarOpen((prev) => !prev)}
                  aria-label="toggle-detail-sidebar"
                >
                  <Menu className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  className="h-10 shrink-0 gap-2 px-3 text-sm font-semibold"
                  onClick={() => {
                    setAutoModalInput("");
                    setAutoModalOpen(true);
                  }}
                  disabled={detailActionLocked}
                  aria-label="auto_from_message"
                  data-testid="detail-auto-button"
                >
                  <GraduationCap className="h-4 w-4" />
                  <span className="hidden lg:inline">auto</span>
                </Button>
                <Button
                  variant="outline"
                  className={`h-10 shrink-0 gap-2 px-3 text-sm font-semibold ${
                    isDevRunning ? "border-red-600 bg-red-600 text-white hover:bg-red-700 hover:text-white" : ""
                  }`}
                  onClick={() => void runDevServer()}
                  disabled={detailActionLocked}
                  aria-label="run_project_test"
                  data-testid="detail-test-button"
                >
                  {isDevRunning ? <Ban className="h-4 w-4" /> : <FlaskConical className="h-4 w-4" />}
                  <span className="hidden lg:inline">{isDevRunning ? "stop" : "test"}</span>
                </Button>
                </div>
              </div>
            </div>
          </div>
        <div className="relative">
          {mobileSidebarOpen && (
            <>
              <div className="fixed inset-0 z-40 bg-black/30 lg:hidden" onClick={() => setMobileSidebarOpen(false)} />
              <div className="fixed left-0 top-20 z-50 h-[calc(100vh-5rem)] w-[82vw] max-w-[320px] overflow-y-auto border-r border-border bg-white p-3 shadow-lg lg:hidden">
                <div className="mb-1">
                  <div className="relative">
                    <Search
                      data-testid="detail-sidebar-search-mobile-icon"
                      className="pointer-events-none absolute right-10 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground"
                    />
                    <input
                      value={sidebarSearch}
                      onChange={(e) => setSidebarSearch(e.target.value)}
                      placeholder="search folders..."
                      className="h-9 w-full rounded-xl border border-input bg-white px-3 pr-14 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      aria-label="detail-sidebar-search-mobile"
                      data-testid="detail-sidebar-search-mobile"
                    />
                    <button
                      type="button"
                      className="absolute right-1 top-1/2 -translate-y-1/2 rounded p-1 text-muted-foreground hover:bg-muted"
                      onClick={() => setMobileSidebarOpen(false)}
                      aria-label="close-detail-sidebar"
                    >
                      <X className="h-4 w-4" />
                    </button>
                  </div>
                </div>
                <Card className="rounded-2xl bg-white" data-testid="detail-sidebar-card-mobile">
                  <CardContent className="space-y-2 pt-4">
                    {selectedProject?.project_type === "mono" ? (
                      <div className="space-y-3">
                        <div>
                          <div className="mb-1 flex items-center gap-1 px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">
                            <FolderOpen className="h-3.5 w-3.5" />
                            <span>APP</span>
                          </div>
                          {renderSidebarProjectList(sidebarMonorepoGroups.app, "mono-app")}
                        </div>
                        <div>
                          <div className="mb-1 flex items-center gap-1 px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">
                            <FolderOpen className="h-3.5 w-3.5" />
                            <span>FEATURE</span>
                          </div>
                          {renderSidebarProjectList(sidebarMonorepoGroups.feature, "mono-feature")}
                        </div>
                        <div>
                          <div className="mb-1 flex items-center gap-1 px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">
                            <FolderOpen className="h-3.5 w-3.5" />
                            <span>TEMPLATE</span>
                          </div>
                          {renderSidebarProjectList(sidebarMonorepoGroups.template, "mono-template")}
                        </div>
                      </div>
                    ) : (
                      renderSidebarProjectList(projects.map((p) => ({ id: p.id, name: p.name })), "default")
                    )}
                  </CardContent>
                </Card>
              </div>
            </>
          )}
          <div className={`${addInputApplying ? "blur-sm" : ""}`}>
          <div className="hidden pt-4 lg:block">
            <div className="relative w-[220px]">
              <Search
                data-testid="detail-sidebar-search-icon"
                className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground"
              />
              <input
                value={sidebarSearch}
                onChange={(e) => setSidebarSearch(e.target.value)}
                placeholder="search folders..."
                className="h-9 w-full rounded-xl border border-input bg-white px-3 pr-8 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                aria-label="detail-sidebar-search"
                data-testid="detail-sidebar-search"
              />
            </div>
          </div>
          <div className="grid gap-4 lg:grid-cols-[220px_1fr] lg:gap-0">
          <div className="hidden lg:block lg:border-r lg:border-border lg:pr-4" data-testid="detail-sidebar-shell">
            <Card className="rounded-2xl bg-white" data-testid="detail-sidebar-card">
              <CardContent className="space-y-2 pt-4">
                {selectedProject?.project_type === "mono" ? (
                  <div className="space-y-3">
                    <div>
                      <div className="mb-1 flex items-center gap-1 px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">
                        <FolderOpen className="h-3.5 w-3.5" />
                        <span>APP</span>
                      </div>
                      {renderSidebarProjectList(sidebarMonorepoGroups.app, "mono-app")}
                    </div>
                    <div>
                      <div className="mb-1 flex items-center gap-1 px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">
                        <FolderOpen className="h-3.5 w-3.5" />
                        <span>FEATURE</span>
                      </div>
                      {renderSidebarProjectList(sidebarMonorepoGroups.feature, "mono-feature")}
                    </div>
                    <div>
                      <div className="mb-1 flex items-center gap-1 px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">
                        <FolderOpen className="h-3.5 w-3.5" />
                        <span>TEMPLATE</span>
                      </div>
                      {renderSidebarProjectList(sidebarMonorepoGroups.template, "mono-template")}
                    </div>
                  </div>
                ) : (
                  renderSidebarProjectList(projects.map((p) => ({ id: p.id, name: p.name })), "default")
                )}
              </CardContent>
            </Card>
          </div>
          <div className="space-y-4 lg:pl-4" data-testid="detail-main-shell">
            <DetailLayoutProvider
              detail={detail}
              showProjectInfo={false}
              selectedProject={selectedProject}
              selectedPane={selectedPane}
              setSelectedPane={setSelectedPane}
              selectedDomain={selectedDomain}
              setSelectedDomain={setSelectedDomain}
              refreshDomainFeatures={refreshDomainFeatures}
              domainLoading={domainLoading}
              domainError={domainError}
              openEditor={openEditor}
              actionsDisabled={isAutoRunningDetail}
              memoDraft={memoDraft}
              updateMemo={updateMemoRealtime}
              flushMemo={flushMemo}
              memoSaving={memoSaving}
              editName={editName}
              editDescription={editDescription}
              editSpec={editSpec}
              editGoal={editGoal}
              setEditName={setEditName}
              setEditDescription={setEditDescription}
              setEditSpec={setEditSpec}
              setEditGoal={setEditGoal}
              editRules={editRules}
              editConstraints={editConstraints}
              editFeatures={editFeatures}
              setEditRules={setEditRules}
              setEditConstraints={setEditConstraints}
              setEditFeatures={setEditFeatures}
              saveProjectInfo={saveProjectInfo}
              saveListPane={saveListPane}
              projectInfoSaving={projectInfoSaving}
              listSaving={detailListSaving}
            />
            <div>
              <div className={sectionLabelClass}>drafts</div>
              <Card data-testid="draft-pane" className={`rounded-2xl border border-border bg-white ${runningImplDraft ? "bg-amber-50" : "bg-white"}`}>
                <CardContent className="space-y-4 pt-6">
                  <div className="grid gap-4 xl:grid-cols-2">
                    <div data-testid="requirements-pane" className="relative space-y-3 pb-12">
                      <div className="flex items-center justify-between gap-2">
                        <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">requirements (## / - / &gt;)</div>
                        <div data-testid="requirements-top-right-actions">
                          <Button
                            type="button"
                            variant="outline"
                            size="icon"
                            onClick={() => void removeDraftPaneFile("job")}
                            disabled={isAutoRunningDetail}
                            aria-label="delete-job-md"
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </div>
                      <div>
                        <div
                          data-testid="requirements-container"
                          className="relative h-[320px] rounded-xl border border-border bg-white"
                        >
                        <div
                          data-testid="requirements-scroll"
                          className="h-full space-y-2 overflow-y-auto px-3 pb-14 pt-3"
                        >
                          {requirementBlocks.length === 0 && (
                            <div className="px-1 py-2 text-xs text-muted-foreground">no requirement blocks</div>
                          )}
                          {requirementBlocks.map((block, index) => (
                            <div key={`req-${block.title}-${index}`} className="group relative rounded-lg border border-border/70 p-2 pr-10 text-xs">
                              <div className="font-semibold">{block.title}</div>
                              {block.rules.length > 0 && (
                                <div className="mt-1 space-y-1 text-muted-foreground">
                                  {block.rules.map((rule, i) => (
                                    <div key={`rule-${index}-${i}`}>- {rule}</div>
                                  ))}
                                </div>
                              )}
                              {block.steps.length > 0 && (
                                <div className="mt-1 space-y-1 text-muted-foreground">
                                  {block.steps.map((step, i) => (
                                    <div key={`step-${index}-${i}`}>&gt; {step}</div>
                                  ))}
                                </div>
                              )}
                              <Button
                                type="button"
                                variant="outline"
                                size="icon"
                                className="absolute right-2 top-2 h-7 w-7 opacity-0 transition group-hover:opacity-100"
                                aria-label={`delete-requirement-item-${index}`}
                                data-testid={`delete-requirement-item-${index}`}
                                onClick={() => void deleteRequirementBlock(index)}
                                disabled={deletingRequirementIndex === index || isAutoRunningDetail}
                              >
                                <Trash2 className="h-3.5 w-3.5" />
                              </Button>
                            </div>
                          ))}
                        </div>
                        <button
                          type="button"
                          className="absolute bottom-3 right-3 inline-flex h-9 w-9 items-center justify-center rounded-full bg-emerald-600 text-white shadow-sm hover:bg-emerald-700 disabled:opacity-60"
                          onClick={() => void (isBuildRunning ? stopBuildJob() : startBuildJob())}
                          disabled={addInputApplying || isAiBusy}
                          aria-label="build_parallel"
                          data-testid="draft-action-build"
                        >
                          <Pencil className="h-4 w-4" />
                        </button>
                        </div>
                      </div>
                    </div>
                    <div data-testid="drafts-raw-pane" className="relative space-y-3 pb-12">
                      <div className="flex items-center justify-between gap-2">
                        <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">drafts.yaml (editable)</div>
                        <Button
                          type="button"
                          variant="outline"
                          size="icon"
                          onClick={() => void removeDraftPaneFile("drafts")}
                          disabled={isAutoRunningDetail}
                          aria-label="delete-drafts-yaml"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                      <div className="relative h-[320px] rounded-xl border border-border bg-white">
                        <textarea
                          value={formDraftsRaw}
                          onChange={(e) => setFormDraftsRaw(e.target.value)}
                          className="h-full w-full resize-none rounded-xl border-0 bg-transparent px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                          data-testid="drafts-raw-editor"
                          placeholder="draft:
  - name: ..."
                        />
                        <button
                          type="button"
                          onClick={focusDraftsRawEditor}
                          disabled={isAutoRunningDetail}
                          aria-label="open-draft-pane-settings"
                          data-testid="open-draft-pane-settings"
                          className="absolute bottom-3 right-3 inline-flex h-9 w-9 items-center justify-center rounded-full bg-emerald-600 text-white shadow-sm hover:bg-emerald-700 disabled:opacity-60"
                        >
                          <Pencil className="h-4 w-4" />
                        </button>
                      </div>
                    </div>
                  </div>
                  <div className="mt-2 flex items-center gap-2">
                      <Button
                        type="button"
                        variant="outline"
                        className="h-9 gap-2 px-3 text-sm font-semibold"
                        onClick={() => setRequirementModalOpen(true)}
                        disabled={addInputApplying || jobMessageGenerating || isAutoRunningDetail || isBuildRunning}
                        aria-label="open-requirement-modal"
                        data-testid="open-requirement-modal"
                      >
                        <Plus className="h-4 w-4" />
                        <span>add</span>
                      </Button>
                  </div>
                </CardContent>
              </Card>
            </div>
            <div>
              <div className={sectionLabelClass}>work pane</div>
              <Card data-testid="draft-work-pane" className="relative rounded-2xl border border-border bg-white">
                <CardContent className={`pb-16 pt-6 transition ${isWorkPaneLocked ? "pointer-events-none blur-[1.5px] select-none" : ""}`}>
                  <div
                    data-testid="draft-work-pane-grid"
                    className="grid gap-3 rounded-2xl border border-border bg-muted/20 p-3 lg:grid-cols-[minmax(0,0.92fr)_minmax(0,1.08fr)] lg:items-stretch"
                  >
                    <div className="flex min-h-[320px] flex-col space-y-2">
                      <div className="flex h-9 items-center">
                        <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">draft_item list</div>
                      </div>
                      <div data-testid="draft-work-list" className="h-[320px] space-y-2 overflow-y-auto rounded-xl border border-border bg-white p-2">
                        {draftsYamlCards.length === 0 && (
                          <div className="px-1 py-2 text-xs text-muted-foreground">no drafts.yaml items</div>
                        )}
                        {draftsYamlCards.map((item) => {
                          const mergedStatus = resolveDraftItemStatus(item.status, {
                            isRunningNow: runningImplDraft && runningImplDraftName === item.name
                          });
                          return (
                            <DraftYamlItemCard
                              key={`draft-work-item-${item.name}`}
                              item={{ name: item.name, status: mergedStatus }}
                              selected={item.name === selectedDraftItemName}
                              onClick={() => setSelectedDraftYamlItem({ name: item.name, draft: item.draft })}
                            />
                          );
                        })}
                      </div>
                    </div>
                    <div className="flex min-h-[320px] flex-col space-y-2">
                      <div className="flex h-9 items-center justify-between gap-2">
                        <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">draft_item detail</div>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() => void runAction("impl_draft", selectedDraftItemName)}
                          disabled={!selectedDraftYamlItem || isAutoRunningDetail || runningImplDraft}
                          data-testid="draft-work-impl"
                        >
                          impl draft_item
                        </Button>
                      </div>
                      {!selectedDraftYamlItem && (
                        <div className="flex h-[320px] items-center justify-center rounded-xl border border-dashed border-border bg-white text-sm text-muted-foreground">
                          draft_item을 선택하면 상세가 표시됩니다.
                        </div>
                      )}
                      {selectedDraftYamlItem && (
                        <div data-testid="draft-work-detail" className="relative h-[320px] rounded-xl border border-border bg-white p-3">
                          <div className="mb-2 flex items-center justify-between gap-2 text-xs text-muted-foreground">
                            <span className="font-semibold text-foreground">{selectedDraftYamlItem.name}</span>
                            <span>{selectedDraftCard?.status ?? "wait"}</span>
                          </div>
                          <div
                            data-testid="draft-work-detail-body"
                            className={`transition ${isSelectedDraftRunning ? "pointer-events-none blur-sm select-none" : ""}`}
                          >
                            <CodeDraftItem yamlText={selectedDraftYamlText} />
                          </div>
                          {isSelectedDraftRunning && (
                            <div
                              data-testid="draft-work-running-overlay"
                              className="absolute inset-0 z-10 flex items-center justify-center rounded-xl bg-white/65"
                            >
                              <div className="rounded-lg border border-amber-300 bg-amber-100 px-3 py-1.5 text-xs font-semibold text-amber-900">
                                impl draft_item 작업중...
                              </div>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                </CardContent>
                <div data-testid="work-pane-review-actions" className="absolute bottom-3 right-3 flex items-center gap-2">
                  <Button
                    variant="outline"
                    className="h-9 gap-2 px-3 text-sm font-semibold"
                    onClick={() => void runQuickAction("retry_incomplete")}
                    disabled={!isReviewState || addInputApplying || isBuildRunning || isAutoRunningDetail}
                    aria-label="retry_red_items"
                  >
                    <RotateCcw className="h-4 w-4" />
                    <span>retry red</span>
                  </Button>
                  <Button
                    variant="outline"
                    className="h-9 gap-2 px-3 text-sm font-semibold"
                    onClick={() => void runQuickAction("finalize_complete")}
                    disabled={!isReviewState || !hasGreenDraft || addInputApplying || isBuildRunning || isAutoRunningDetail}
                    aria-label="finalize_green_items"
                  >
                    <CheckCircle2 className="h-4 w-4" />
                    <span>complete</span>
                  </Button>
                </div>
                {isWorkPaneLocked && (
                  <div
                    data-testid="draft-work-pane-lock-overlay"
                    className="absolute inset-0 z-20 flex items-center justify-center rounded-2xl bg-white/60"
                  >
                    <div className="rounded-lg border border-amber-300 bg-amber-50 px-4 py-2 text-sm font-semibold text-amber-900">
                      먼저 add 버튼으로 draft_item을 추가하세요.
                    </div>
                  </div>
                )}
              </Card>
            </div>

            <div>
              <div className={sectionLabelClass}>check</div>
              <Card data-testid="check-pane" className="relative rounded-2xl border border-border bg-white">
                <CardContent className={`space-y-5 pt-6 transition ${isCheckPaneLocked ? "pointer-events-none blur-[1.5px] select-none" : ""}`}>
                  <div className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
                    <div className="space-y-2">
                      <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">subject</div>
                      <div
                        data-testid="check-pane-subject"
                        className="rounded-2xl border border-border bg-white px-4 py-3 text-sm font-semibold text-foreground"
                      >
                        {checkSubject}
                      </div>
                    </div>
                    <div className="space-y-2">
                      <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">manual flow</div>
                      <div className="rounded-2xl border border-border bg-white px-4 py-3 text-sm text-muted-foreground">
                        {isAutoRunningDetail
                          ? "auto mode에서는 현재 workflow가 check까지 관리합니다."
                          : "run parallel 이후 수동 check는 이 pane에서 rc로 실행합니다."}
                      </div>
                    </div>
                  </div>

                  <div className="space-y-2">
                    <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">step pane</div>
                    <div
                      data-testid="check-step-pane"
                      className="max-h-72 overflow-y-auto rounded-2xl border border-border bg-white p-3"
                    >
                      {checkSteps.length === 0 && (
                        <div className="text-sm text-muted-foreground">drafts.yaml에서 추출한 check step이 없습니다.</div>
                      )}
                      {checkSteps.length > 0 && (
                        <div className="space-y-2">
                          {checkSteps.map((item, index) => (
                            <div
                              key={`check-step-${item.subject}-${index}`}
                              className="rounded-xl border border-border/70 bg-muted/20 px-3 py-2"
                            >
                              <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                                {item.subject} · {item.source}
                              </div>
                              <div className="mt-1 text-sm text-foreground">
                                {index + 1}. {item.text}
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="space-y-3">
                    <div className="flex items-center justify-between gap-3">
                      <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                        screenshots
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {selectedCheckScreenshot ? selectedCheckScreenshot.path : ".project/screenshot 선택 필요"}
                      </div>
                    </div>
                    <div data-testid="check-screenshot-grid" className="overflow-x-auto">
                      <div className="flex w-max gap-3 pb-2">
                        {checkScreenshots.length === 0 && (
                          <div className="w-[240px] rounded-2xl border border-dashed border-border bg-muted/20 px-4 py-10 text-sm text-muted-foreground">
                            no screenshots
                          </div>
                        )}
                        {checkScreenshots.map((item) => (
                          <button
                            key={`check-screenshot-${item.path}`}
                            type="button"
                            data-testid={`check-screenshot-card-${item.name}`}
                            className={`w-[240px] shrink-0 rounded-2xl border bg-white p-3 text-left ${
                              selectedCheckScreenshot?.path === item.path
                                ? "border-primary shadow-sm"
                                : "border-border hover:bg-muted/20"
                            }`}
                            onClick={() => setSelectedScreenshotPath(item.path)}
                            onDoubleClick={() => {
                              setSelectedScreenshotPath(item.path);
                              setScreenshotPreviewItem(item);
                            }}
                          >
                            <div className="h-36 overflow-hidden rounded-xl border border-border bg-muted/20">
                              <img src={item.url} alt={item.name} className="h-full w-full object-cover" loading="lazy" />
                            </div>
                            <div className="mt-2 truncate text-sm font-semibold text-foreground">{item.name}</div>
                          </button>
                        ))}
                      </div>
                    </div>
                    <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
                      <Textarea
                        value={checkFeedbackInput}
                        onChange={(e) => setCheckFeedbackInput(e.target.value)}
                        rows={4}
                        className="min-h-[120px] resize-y bg-white"
                        placeholder="{파일 위치명}에서 어떤 점을 개선해야 하는지 적으세요"
                        data-testid="check-feedback-input"
                      />
                      <Button
                        type="button"
                        variant="outline"
                        className="h-10 gap-2 px-4 text-sm font-semibold"
                        onClick={() => void appendScreenshotFeedback()}
                        disabled={!canAppendCheckFeedback}
                        data-testid="check-feedback-add"
                      >
                        <Plus className="h-4 w-4" />
                        <span>{checkFeedbackSaving ? "saving..." : "add feedback"}</span>
                      </Button>
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center justify-end gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      size="icon"
                      aria-label="open-feedback-report"
                      data-testid="check-report-button"
                      onClick={() => setReportOpen(true)}
                      disabled={!hasFeedbackReport}
                    >
                      <FileText className="h-4 w-4" />
                    </Button>
                    <Button
                      type="button"
                      className="h-10 gap-2 px-4 text-sm font-semibold"
                      onClick={() => void runManualCheck()}
                      disabled={!canRunManualCheck}
                      data-testid="check-pane-run"
                    >
                      <FlaskConical className="h-4 w-4" />
                      <span>{checkRunning ? "checking..." : "check"}</span>
                    </Button>
                  </div>
                </CardContent>
                {isCheckPaneLocked && (
                  <div
                    data-testid="check-pane-lock-overlay"
                    className="absolute inset-0 z-20 flex items-center justify-center rounded-2xl bg-white/60"
                  >
                    <div className="rounded-lg border border-amber-300 bg-amber-50 px-4 py-2 text-sm font-semibold text-amber-900">
                      먼저 work pane에서 draft_item 구현을 완료하세요.
                    </div>
                  </div>
                )}
              </Card>
            </div>

            <div>
              <div className={sectionLabelClass}>runtime log</div>
              <Card className="rounded-2xl">
                <CardContent className="pt-6">
                <div
                  data-testid="runtime-log"
                  className="max-h-64 overflow-y-auto rounded-2xl border border-border bg-card p-3 text-xs"
                >
                  {logs.length === 0 && <div>no logs</div>}
                  {logs.map((line, i) => (
                    <div key={`${line}-${i}`}>{line}</div>
                  ))}
                </div>
                </CardContent>
              </Card>
            </div>
          </div>
          </div>
          {addInputApplying && (
            <div className="absolute inset-0 z-20 flex items-center justify-center">
              <div className="rounded-xl border border-border bg-white/95 px-4 py-2 text-sm font-semibold text-foreground shadow">
                job.md 반영중...
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
      )}
      {screenshotPreviewItem && (
        <div className="fixed inset-0 z-50 bg-black/40 p-4">
          <Card className="mx-auto flex h-full w-full max-w-6xl flex-col rounded-2xl">
            <CardHeader className="flex flex-row items-start justify-between gap-3 space-y-0">
              <div className="min-w-0">
                <CardTitle className="truncate">{screenshotPreviewItem.name}</CardTitle>
                <div className="mt-1 truncate text-sm text-muted-foreground">{screenshotPreviewItem.path}</div>
              </div>
              <button
                type="button"
                className="rounded p-1 text-muted-foreground hover:bg-muted"
                onClick={() => setScreenshotPreviewItem(null)}
                aria-label="close-screenshot-preview"
              >
                <X className="h-5 w-5" />
              </button>
            </CardHeader>
            <CardContent className="min-h-0 flex-1 overflow-hidden">
              <div className="flex h-full items-center justify-center overflow-auto rounded-2xl border border-border bg-muted/10 p-4">
                <img
                  src={screenshotPreviewItem.url}
                  alt={screenshotPreviewItem.name}
                  className="max-h-full w-auto max-w-full rounded-xl border border-border bg-white object-contain"
                />
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {reportOpen && (
        <div className="fixed inset-0 z-50 bg-black/40 p-4">
          <Card className="mx-auto flex h-full w-full max-w-5xl flex-col rounded-2xl">
            <CardHeader className="flex flex-row items-start justify-between gap-3 space-y-0">
              <div className="min-w-0">
                <CardTitle className="truncate">job.md feedback report</CardTitle>
                <div className="mt-1 text-sm text-muted-foreground">{detail?.name ?? "selected project"}</div>
              </div>
              <button
                type="button"
                className="rounded p-1 text-muted-foreground hover:bg-muted"
                onClick={() => setReportOpen(false)}
                aria-label="close-feedback-report"
              >
                <X className="h-5 w-5" />
              </button>
            </CardHeader>
            <CardContent className="min-h-0 flex-1 overflow-hidden">
              <div
                data-testid="feedback-report-modal"
                className="h-full overflow-y-auto rounded-2xl border border-border bg-white p-6"
              >
                <div className="mx-auto flex max-w-3xl flex-col gap-5">
                  {renderEpisodeMarkdown(feedbackMdRaw)}
                </div>
              </div>
            </CardContent>
            <div className="flex justify-end gap-2 p-4 pt-0">
              <Button variant="outline" onClick={() => setReportOpen(false)}>
                취소
              </Button>
              <Button
                className="gap-2"
                onClick={() => void retryFromFeedback()}
                disabled={checkRetrying || isBuildRunning || isAutoRunningDetail}
                data-testid="feedback-retry-button"
              >
                <RotateCcw className="h-4 w-4" />
                <span>{checkRetrying ? "retrying..." : "다시 하기"}</span>
              </Button>
            </div>
          </Card>
        </div>
      )}

      {editOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="w-full max-w-2xl rounded-2xl">
            <CardHeader>
              <CardTitle>
                {selectedPane === "project_info"
                  ? "Edit Project Info"
                  : `Edit ${selectedPane.charAt(0).toUpperCase()}${selectedPane.slice(1)}`}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {selectedPane === "project_info" ? (
                <>
                  <Label>Name</Label>
                  <Input value={editName} onChange={(e) => setEditName(e.target.value)} />
                  <Label>Description</Label>
                  <Input value={editDescription} onChange={(e) => setEditDescription(e.target.value)} />
                  <Label>Spec</Label>
                  <Input value={editSpec} onChange={(e) => setEditSpec(e.target.value)} />
                  <Label>Goal</Label>
                  <Input data-testid="edit-goal" value={editGoal} onChange={(e) => setEditGoal(e.target.value)} />
                </>
              ) : selectedPane === "rules" ? (
                <>
                  <Label>Rules</Label>
                  <Textarea value={editRules} onChange={(e) => setEditRules(e.target.value)} rows={8} />
                </>
              ) : selectedPane === "constraints" ? (
                <>
                  <Label>Constraints</Label>
                  <Textarea
                    value={editConstraints}
                    onChange={(e) => setEditConstraints(e.target.value)}
                    rows={8}
                  />
                </>
              ) : (
                <>
                  <Label>Features</Label>
                  <Textarea value={editFeatures} onChange={(e) => setEditFeatures(e.target.value)} rows={8} />
                </>
              )}
              <div className="flex justify-end gap-2">
                <Button data-testid="edit-save" onClick={() => void saveEditor()}>
                  Save
                </Button>
                <Button variant="outline" onClick={() => setEditOpen(false)}>
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {autoModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="relative flex h-[70vh] max-h-[900px] w-full max-w-3xl flex-col rounded-2xl">
            <CardHeader>
              <CardTitle>auto_from_message</CardTitle>
            </CardHeader>
            <CardContent className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden">
              <div className="flex min-h-0 flex-1 flex-col space-y-3 rounded-xl border border-border p-3">
                <Label>요청 메시지</Label>
                <Textarea
                  value={autoModalInput}
                  onChange={(e) => setAutoModalInput(e.target.value)}
                  className="min-h-[260px] flex-1"
                  placeholder="요청 내용을 입력하세요"
                />
              </div>
              <div className="flex justify-end gap-2">
                <Button onClick={() => void runAutoFlowFromMessage()} disabled={autoRunning}>
                  요청하기
                </Button>
                <Button variant="outline" onClick={() => setAutoModalOpen(false)} disabled={autoRunning}>
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {requirementModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="relative flex h-[70vh] max-h-[900px] w-full max-w-3xl flex-col rounded-2xl">
            <CardHeader>
              <CardTitle>요구사항 추가</CardTitle>
            </CardHeader>
            <CardContent className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden">
              <div className="flex min-h-0 flex-1 flex-col space-y-3 rounded-xl border border-border p-3">
                <Label>입력 포맷: ## / - / &gt;</Label>
                <Textarea
                  value={requirementModalInput}
                  onChange={(e) => setRequirementModalInput(e.target.value)}
                  className="min-h-[260px] flex-1"
                  placeholder={"## 기능 이름\n- 기능(옵션)\n> 순서(옵션)\n\n## 다른 기능\n- 규칙"}
                />
              </div>
              <div className="flex justify-end gap-2">
                <Button onClick={() => void submitRequirementBlocks()} disabled={addInputApplying}>
                  저장
                </Button>
                <Button
                  variant="outline"
                  onClick={() => {
                    setRequirementModalOpen(false);
                    setRequirementModalInput("");
                  }}
                  disabled={addInputApplying}
                >
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {jobMessageModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="relative flex h-[70vh] max-h-[900px] w-full max-w-3xl flex-col rounded-2xl">
            <CardHeader>
              <CardTitle>메시지로 job.md 생성</CardTitle>
            </CardHeader>
            <CardContent className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden">
              <div className="flex min-h-0 flex-1 flex-col space-y-3 rounded-xl border border-border p-3">
                <Label>multiline 입력 (## / - / &gt;)</Label>
                <Textarea
                  value={jobMessageModalInput}
                  onChange={(e) => setJobMessageModalInput(e.target.value)}
                  className="min-h-[260px] flex-1"
                  placeholder={"## 기능 이름\n- 기능(옵션)\n> 순서(옵션)"}
                />
              </div>
              <div className="flex justify-end gap-2">
                <Button onClick={() => void submitMessageJobGenerate()} disabled={jobMessageGenerating}>
                  생성
                </Button>
                <Button
                  variant="outline"
                  onClick={() => {
                    setJobMessageModalOpen(false);
                    setJobMessageModalInput("");
                  }}
                  disabled={jobMessageGenerating}
                >
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {draftModalAction && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="w-full max-w-lg rounded-2xl">
            <CardHeader>
              <CardTitle className="capitalize">
                {draftModalAction === "add_draft" ? draftModalName : draftModalAction.replace("_", " ")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {draftModalAction === "add_draft" && (
                <>
                  {draftFormFields.length === 0 ? (
                    <>
                      <Label>add_code_draft payload (optional)</Label>
                      <Input
                        value={addDraftPayload}
                        onChange={(e) => setAddDraftPayload(e.target.value)}
                        placeholder="feature 메시지 입력"
                      />
                    </>
                  ) : (
                    <div className="max-h-[55vh] space-y-2 overflow-y-auto pr-1">
                      {draftFormFields.map((field, index) => (
                        <div key={`${field.key}-${index}`} className="space-y-1">
                          <Label>{field.key}</Label>
                          <Input
                            value={field.value}
                            onChange={(e) => updateDraftField(index, e.target.value)}
                            placeholder={field.key}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                </>
              )}
              <div className="flex justify-end gap-2">
                <Button
                  onClick={async () => {
                    const ok = await runAction(draftModalAction);
                    if (ok) setDraftModalAction(null);
                  }}
                >
                  Run
                </Button>
                <Button variant="outline" onClick={() => setDraftModalAction(null)}>
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {buildToast && (
        <div className="fixed bottom-4 right-4 z-[60] rounded-xl border border-border bg-white px-4 py-3 text-sm shadow-lg">
          {buildToast}
        </div>
      )}

      {templateModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
          <Card className="h-[78vh] w-full max-w-5xl rounded-2xl">
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <span>template assets ({templateModalType})</span>
                <button className="rounded p-1 text-muted-foreground hover:bg-muted" onClick={() => setTemplateModalOpen(false)}>
                  <X className="h-4 w-4" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="h-[calc(78vh-5.5rem)] overflow-hidden">
              {templateModalLoading ? (
                <div className="text-sm text-muted-foreground">loading...</div>
              ) : (
                <div className="grid h-full gap-3 md:grid-cols-[240px_1fr]">
                  <div className="space-y-3 overflow-y-auto rounded-xl border border-border p-3">
                    <div>
                      <button
                        className="mb-2 flex w-full items-center gap-2 text-left text-xs font-bold uppercase tracking-wide text-muted-foreground"
                        onClick={() => setTemplatePromptsOpen((prev) => !prev)}
                      >
                        <FolderOpen className="h-3.5 w-3.5" />
                        <span>PROMPTS</span>
                      </button>
                      {templatePromptsOpen && (
                        <div className="space-y-1">
                          {templateAssets.prompts.length === 0 && (
                            <div className="text-xs text-muted-foreground">no prompt files</div>
                          )}
                          {templateAssets.prompts.map((file) => (
                            <button
                              key={`prompt-list-${file.name}`}
                              className={`w-full rounded px-2 py-1 text-left text-xs ${
                                templateSelectedKey === `prompts:${file.name}`
                                  ? "bg-muted font-semibold text-foreground"
                                  : "text-muted-foreground hover:bg-muted/50"
                              }`}
                              onClick={() => selectTemplateAsset(`prompts:${file.name}`)}
                            >
                              {file.name}
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                    <div>
                      <button
                        className="mb-2 flex w-full items-center gap-2 text-left text-xs font-bold uppercase tracking-wide text-muted-foreground"
                        onClick={() => setTemplateTemplatesOpen((prev) => !prev)}
                      >
                        <FolderOpen className="h-3.5 w-3.5" />
                        <span>TEMPLATES</span>
                      </button>
                      {templateTemplatesOpen && (
                        <div className="space-y-1">
                          {templateAssets.templates.length === 0 && (
                            <div className="text-xs text-muted-foreground">no template files</div>
                          )}
                          {templateAssets.templates.map((file) => (
                            <button
                              key={`template-list-${file.name}`}
                              className={`w-full rounded px-2 py-1 text-left text-xs ${
                                templateSelectedKey === `templates:${file.name}`
                                  ? "bg-muted font-semibold text-foreground"
                                  : "text-muted-foreground hover:bg-muted/50"
                              }`}
                              onClick={() => selectTemplateAsset(`templates:${file.name}`)}
                            >
                              {file.name}
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                  <div ref={templateContentRef} className="h-full space-y-2 overflow-y-auto rounded-xl border border-border p-3">
                    <div className="flex items-center justify-between gap-2 text-xs font-semibold text-foreground">
                      <span>{templateSelectedFile ? templateSelectedFile.name : "select file"}</span>
                      {templateSelectedFile && (
                        <button
                          className="rounded p-1 text-muted-foreground hover:bg-muted"
                          onClick={() => setTemplateEditing((prev) => !prev)}
                          aria-label="edit-template-asset"
                        >
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                      )}
                    </div>
                    {templateEditing ? (
                      <div className="space-y-2">
                        <Textarea
                          value={templateEditorValue}
                          onChange={(e) => setTemplateEditorValue(e.target.value)}
                          rows={22}
                        />
                        <div className="flex justify-end gap-2">
                          <Button size="sm" onClick={() => void saveTemplateAsset()} disabled={templateSaving}>
                            Save
                          </Button>
                          <Button variant="outline" size="sm" onClick={() => setTemplateEditing(false)}>
                            Cancel
                          </Button>
                        </div>
                      </div>
                    ) : (
                      <pre className="max-h-[60vh] overflow-y-auto rounded bg-muted/30 p-2 text-xs">
                        {templateSelectedFile?.content || ""}
                      </pre>
                    )}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {isCreateOpen && (
        <div
          data-testid="create-project-modal"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4"
        >
          <Card className="w-full max-w-xl rounded-2xl">
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                Create Project
                <button
                  className="rounded p-1 text-muted-foreground hover:bg-muted"
                  onClick={() => {
                    setCreateOpen(false);
                    setCreateOpenLocal(false);
                  }}
                >
                  <X className="h-4 w-4" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 rounded-2xl border border-border p-4">
              <Label>New Project Name</Label>
              <Input
                data-testid="new-project-name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
              />
              <Label>Description</Label>
              <Input value={newDescription} onChange={(e) => setNewDescription(e.target.value)} />
              <Label>Path</Label>
              <div className="flex items-center gap-2">
                <Input
                  data-testid="new-project-path"
                  value={newPath}
                  onChange={(e) => setNewPath(e.target.value)}
                  placeholder="/home/tree/temp/orc-web-demo"
                />
                <Button
                  variant="outline"
                  size="icon"
                  type="button"
                  onClick={() => openBrowse("create")}
                  aria-label="open-create-browser"
                >
                  <FolderOpen className="h-4 w-4" />
                </Button>
              </div>
              <Label>Spec</Label>
              <Input
                value={newSpec}
                onChange={(e) => setNewSpec(e.target.value)}
                placeholder="react, zustand"
              />
              <div className="flex justify-end gap-2">
                <Button data-testid="create-project" onClick={() => void createProject()}>
                  Create Project
                </Button>
                <Button
                  variant="outline"
                  onClick={() => {
                    setCreateOpen(false);
                    setCreateOpenLocal(false);
                  }}
                >
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {loadOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="w-full max-w-xl rounded-2xl">
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                Load Project
                <button
                  className="rounded p-1 text-muted-foreground hover:bg-muted"
                  onClick={() => {
                    setLoadOpen(false);
                    setLoadPath("");
                  }}
                >
                  <X className="h-4 w-4" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <Label>Path</Label>
              <div className="flex items-center gap-2">
                <Input
                  value={loadPath}
                  onChange={(e) => setLoadPath(e.target.value)}
                  placeholder="/home/tree/project/existing-project"
                />
                <Button
                  variant="outline"
                  size="icon"
                  type="button"
                  onClick={() => openBrowse("load")}
                  aria-label="open-load-browser"
                >
                  <FolderOpen className="h-4 w-4" />
                </Button>
              </div>
              <div className="flex justify-end gap-2">
                <Button onClick={() => void loadProjectByPath(false)}>Load</Button>
                <Button
                  variant="outline"
                  onClick={() => {
                    setLoadOpen(false);
                    setLoadPath("");
                  }}
                >
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {browseOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="h-[90vh] w-full max-w-[600px] rounded-2xl">
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                File Explorer
                <button className="rounded p-1 text-muted-foreground hover:bg-muted" onClick={() => setBrowseOpen(false)}>
                  <X className="h-4 w-4" />
                </button>
              </CardTitle>
            </CardHeader>
            <CardContent className="flex h-[calc(90vh-88px)] flex-col space-y-3">
              <div className="flex items-center gap-2">
                <Input value={browsePath} onChange={(e) => setBrowsePath(e.target.value)} />
                <Button variant="outline" size="icon" onClick={() => void browseDirs(browsePath)} aria-label="browse-refresh">
                  <RefreshCw className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => {
                    if (browseParentPath) void browseDirs(browseParentPath);
                  }}
                  disabled={!browseParentPath}
                  aria-label="browse-up"
                >
                  <CornerUpLeft className="h-4 w-4" />
                </Button>
              </div>
              {browseError && <div className="text-sm text-red-600">{browseError}</div>}
              <div className="min-h-0 flex-1 overflow-y-auto rounded-xl border border-border">
                {browseLoading && <div className="p-3 text-sm text-muted-foreground">loading...</div>}
                {!browseLoading &&
                  browseEntries.filter((entry) => {
                    if (!browseShowHidden && entry.name.startsWith(".")) return false;
                    if (!browseKeyword.trim()) return true;
                    return entry.name.toLowerCase().includes(browseKeyword.toLowerCase());
                  }).length === 0 && (
                  <div className="p-3 text-sm text-muted-foreground">(empty)</div>
                )}
                {!browseLoading &&
                  browseEntries
                    .filter((entry) => {
                      if (!browseShowHidden && entry.name.startsWith(".")) return false;
                      if (!browseKeyword.trim()) return true;
                      return entry.name.toLowerCase().includes(browseKeyword.toLowerCase());
                    })
                    .map((entry) => (
                    <button
                      key={entry.path}
                      className="flex w-full items-center justify-between border-b border-border px-3 py-2 text-left text-sm hover:bg-muted/40"
                      onClick={() => {
                        applyBrowsePath(entry.path);
                        void browseDirs(entry.path);
                      }}
                    >
                      <span className="truncate">{entry.name}</span>
                      {entry.hasProjectMeta && <span className="text-xs text-muted-foreground">.project</span>}
                    </button>
                  ))}
              </div>
              <div className="flex items-center gap-2">
                <div className="flex flex-1 items-center gap-2 rounded-md border border-border px-2">
                  <Search className="h-4 w-4 text-muted-foreground" />
                  <Input
                    value={browseQuery}
                    onChange={(e) => setBrowseQuery(e.target.value)}
                    placeholder="folder name"
                    className="border-0 px-0 shadow-none focus-visible:ring-0"
                  />
                </div>
                <Button variant="outline" onClick={() => setBrowseKeyword(browseQuery)}>
                  Search
                </Button>
              </div>
              <div className="flex items-center justify-end gap-3">
                <label className="flex items-center gap-2 text-sm text-muted-foreground">
                  <input
                    type="checkbox"
                    checked={browseShowHidden}
                    onChange={(e) => setBrowseShowHidden(e.target.checked)}
                  />
                  hidden
                </label>
                <Button
                  variant="outline"
                  onClick={() => {
                    applyBrowsePath(browsePath);
                    setBrowseOpen(false);
                  }}
                >
                  Submit
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </main>
    </>
  );
}
