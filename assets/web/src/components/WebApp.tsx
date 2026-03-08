import { useEffect, useMemo, useRef, useState } from "react";
import {
  Ban,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clapperboard,
  Code2,
  CornerUpLeft,
  FilePlus2,
  FlaskConical,
  FolderOpen,
  GraduationCap,
  Hammer,
  Menu,
  LayoutGrid,
  List,
  NotebookPen,
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
import { DraftYamlItemModal } from "@/components/drafts/DraftYamlItemModal";
import { useOrcStore, type Detail, type Project } from "@/store/orc-store";
import { DetailLayoutProvider } from "@/layouts/detail";

const sectionLabelClass = "mt-4 mb-2 px-2 text-base font-bold uppercase tracking-wide text-foreground/80 lg:mt-8 lg:mb-3";
const projectContainerItemClass =
  "project-container-item relative rounded-xl border border-border bg-card p-3 text-left text-sm hover:bg-muted/40";
const projectContainerItemMinimalClass =
  "project-container-item-minimal relative rounded-xl border border-border bg-card p-3 text-left text-sm hover:bg-muted/40";
type DraftModalAction = "add_draft" | "impl_draft" | "check_code";
type BrowseTarget = "create" | "load";
type BrowseEntry = { name: string; path: string; hasProjectMeta: boolean };
type ProjectItemViewMode = "card" | "minimal";
type ProfileType = "code" | "mono" | "write" | "video";
type DraftFormField = { key: string; value: string };
type TemplateAssetFile = { name: string; path: string; content: string };

function stateLabel(state?: Project["state"]): string {
  if (state === "build") return "build";
  if (state === "run") return "run";
  if (state === "review") return "review";
  if (state === "init") return "init";
  if (state === "work") return "work";
  if (state === "wait") return "wait";
  return "basic";
}

function stateClass(state?: Project["state"]): string {
  if (state === "build") return "border-amber-500/70 bg-amber-100 text-amber-800";
  if (state === "run") return "border-emerald-500/50 bg-emerald-500/10 text-emerald-700";
  if (state === "review") return "border-emerald-500/70 bg-emerald-100 text-emerald-800";
  if (state === "init") return "border-sky-500/50 bg-sky-500/10 text-sky-700";
  if (state === "work") return "border-orange-500/60 bg-orange-100 text-orange-800";
  if (state === "wait") return "border-zinc-400/70 bg-zinc-100 text-zinc-700";
  return "border-zinc-400/70 bg-zinc-100 text-zinc-700";
}

function projectTypeLabel(type?: Project["project_type"]): string {
  if (type === "mono") return "monorepo";
  if (type === "movie") return "video";
  if (type === "story") return "write";
  return "code";
}

function profileTypeFromProjectType(type?: Project["project_type"]): ProfileType {
  if (type === "mono") return "mono";
  if (type === "movie") return "video";
  if (type === "story") return "write";
  return "code";
}

function ProjectTypeIcon({ type }: { type: Project["project_type"] }) {
  if (type === "story") return <NotebookPen className="h-5 w-5 text-muted-foreground" />;
  if (type === "movie") return <Clapperboard className="h-5 w-5 text-muted-foreground" />;
  if (type === "mono") return <Shapes className="h-5 w-5 text-muted-foreground" />;
  return <Code2 className="h-5 w-5 text-muted-foreground" />;
}

function parseLines(input: string): string[] {
  return input
    .split("\n")
    .map((v) => v.trim())
    .filter((v) => v.length > 0);
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
  const [draftModalAction, setDraftModalAction] = useState<DraftModalAction | null>(null);
  const [formAddInputOpen, setFormAddInputOpen] = useState(false);
  const [formRawInput, setFormRawInput] = useState("");
  const [formAiMessage, setFormAiMessage] = useState("");
  const [formAiGenerating, setFormAiGenerating] = useState(false);
  const [formAiDone, setFormAiDone] = useState(false);
  const [addInputStatus, setAddInputStatus] = useState("");
  const [addInputApplying, setAddInputApplying] = useState(false);
  const [autoModalOpen, setAutoModalOpen] = useState(false);
  const [autoModalInput, setAutoModalInput] = useState("");
  const [autoRunning, setAutoRunning] = useState(false);
  const [selectedInputTitle, setSelectedInputTitle] = useState("");
  const [selectedDraftYamlItem, setSelectedDraftYamlItem] = useState<{
    name: string;
    draft: Record<string, unknown>;
  } | null>(null);
  const [draftsViewMode, setDraftsViewMode] = useState<"items" | "input" | "drafts">("items");
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
  const lastSavedMemoRef = useRef("");
  const codeSectionRef = useRef<HTMLDivElement | null>(null);
  const monorepoSectionRef = useRef<HTMLDivElement | null>(null);
  const videoSectionRef = useRef<HTMLDivElement | null>(null);
  const writeSectionRef = useRef<HTMLDivElement | null>(null);
  const templateContentRef = useRef<HTMLDivElement | null>(null);
  const rawInputSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
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
    setActiveRunProjectIds
  } = useOrcStore();
  const isCreateOpen = createOpen || createOpenLocal;

  const selectedProject = useMemo(
    () => projects.find((p) => p.id === selectedId) ?? null,
    [projects, selectedId]
  );
  useEffect(() => {
    return () => {
      if (rawInputSaveTimerRef.current) {
        clearTimeout(rawInputSaveTimerRef.current);
        rawInputSaveTimerRef.current = null;
      }
    };
  }, []);
  const groupedProjects = useMemo(
    () => ({
      code: projects.filter((v) => v.project_type === "code"),
      monorepo: projects.filter((v) => v.project_type === "mono"),
      video: projects.filter((v) => v.project_type === "movie"),
      write: projects.filter((v) => v.project_type === "story")
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
    if (activeRunProjectIds.includes(project.id)) return "run";
    return project.state;
  }
  const templateSelectedFile = useMemo(() => {
    const selectedPrompt = templateAssets.prompts.find((file) => `prompts:${file.name}` === templateSelectedKey);
    if (selectedPrompt) return selectedPrompt;
    const selectedTemplate = templateAssets.templates.find((file) => `templates:${file.name}` === templateSelectedKey);
    return selectedTemplate ?? null;
  }, [templateAssets, templateSelectedKey]);

  async function loadProjects() {
    const res = await fetch(apiUrl("/api/projects"));
    const data = await res.json();
    const next: Project[] = data.projects ?? [];
    setProjects(next);
    setActiveRunProjectIds((prev) => {
      const nextRunIds = next.filter((project) => project.state === "run").map((project) => project.id);
      const merged = new Set([...prev, ...nextRunIds]);
      for (const id of [...merged]) {
        if (!next.some((project) => project.id === id)) {
          merged.delete(id);
        }
      }
      for (const project of next) {
        if (project.state !== "run" && merged.has(project.id) && !prev.includes(project.id)) {
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
            {visualState === "run" && <FlaskConical className="h-3.5 w-3.5 text-muted-foreground" />}
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

  function scrollToProjectSection(section: "code" | "monorepo" | "video" | "write") {
    const ref =
      section === "code"
        ? codeSectionRef
        : section === "monorepo"
          ? monorepoSectionRef
          : section === "video"
            ? videoSectionRef
            : writeSectionRef;
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
    if (detail.state === "run") {
      setActiveRunProjectIds((prev) => [...new Set([...prev, detail.id])]);
      return;
    }
    setActiveRunProjectIds((prev) => prev.filter((id) => id !== detail.id));
  }, [detail?.id, detail?.state]);

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
    const titles = (detail?.inputItems ?? []).map((item) => item.title);
    if (titles.length === 0) {
      setSelectedInputTitle("");
      return;
    }
    if (!selectedInputTitle || !titles.includes(selectedInputTitle)) {
      setSelectedInputTitle(titles[0]);
    }
  }, [detail?.id, detail?.inputItems, selectedInputTitle]);

  useEffect(() => {
    if (!detail?.id) return;
    if (detail.state !== "build") return;
    const timer = setInterval(() => {
      void pollBuildStatus(detail.id);
    }, 1200);
    return () => clearInterval(timer);
  }, [detail?.id, detail?.state]);

  useEffect(() => {
    if (activeRunProjectIds.length === 0) return;
    const timer = setInterval(() => {
      void loadProjects();
    }, 2000);
    return () => clearInterval(timer);
  }, [activeRunProjectIds.length]);

  useEffect(() => {
    if (!selectedDraftYamlItem) return;
    const exists = (detail?.draftsYamlItems ?? []).some((item) => item.name === selectedDraftYamlItem.name);
    if (!exists) {
      setSelectedDraftYamlItem(null);
    }
  }, [detail?.draftsYamlItems, selectedDraftYamlItem]);

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
    if (bulkDeleteIds.length === 0) return;
    let deleted = 0;
    for (const id of bulkDeleteIds) {
      const res = await fetch(apiUrl("/api/project-delete"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ id })
      });
      if (res.ok) deleted += 1;
    }
    setDetail(null);
    setSelectedId("");
    setBulkDeleteIds([]);
    setBulkDeleteMode(false);
    await loadProjects();
    pushLog(`project deleted: ${deleted}`);
  }

  async function saveRawInputMd(nextRaw: string) {
    if (!detail) return;
    const res = await fetch(apiUrl("/api/input-md-raw"), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        id: detail.id,
        raw: nextRaw
      })
    });
    const data = await res.json();
    if (!res.ok) {
      pushLog(`raw input save failed: ${String(data.error ?? "unknown error")}`);
      return;
    }
    setDetail(data.detail);
  }

  function handleRawInputChange(nextRaw: string) {
    setFormRawInput(nextRaw);
    if (rawInputSaveTimerRef.current) {
      clearTimeout(rawInputSaveTimerRef.current);
    }
    rawInputSaveTimerRef.current = setTimeout(() => {
      void saveRawInputMd(nextRaw);
    }, 250);
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
    setDetail((prev) => (prev ? { ...prev, state: "build", current_job: "starting" } : prev));
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
              current_job: String(data.current_job ?? "")
            }
          : project
      )
    );
    setDetail((prev) =>
      prev && prev.id === id
        ? {
            ...prev,
            state: nextState,
            current_job: String(data.current_job ?? "")
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

  async function runAction(action: DraftModalAction): Promise<boolean> {
    if (!detail) return false;
    const isImpl = action === "impl_draft";
    if (isImpl) setRunningImplDraft(true);
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
      if (isImpl) setRunningImplDraft(false);
    }
  }

  async function runQuickAction(action: "check_code" | "retry_incomplete" | "finalize_complete") {
    if (!detail) return;
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

  async function generateInputMdWithAi() {
    if (!detail) return;
    const message = formAiMessage.trim();
    if (!message) {
      pushLog("input generate failed: message is empty");
      return;
    }
    setFormAiGenerating(true);
    setFormAiDone(false);
    try {
      const res = await fetch(apiUrl("/api/input-md-generate"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          message
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`input generate failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      setDetail(data.detail);
      setFormRawInput(String(data.detail?.inputMdRaw ?? ""));
      setFormAiDone(true);
      setTimeout(() => setFormAiDone(false), 1800);
      pushLog(String(data.output ?? "input.md generated"));
    } finally {
      setFormAiGenerating(false);
    }
  }

  async function runAutoFlowFromMessage() {
    if (!detail) return;
    const message = autoModalInput.trim();
    if (!message) {
      pushLog("auto run failed: message is empty");
      return;
    }
    setAutoRunning(true);
    try {
      const res = await fetch(apiUrl("/api/auto-run"), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          id: detail.id,
          message
        })
      });
      const data = await res.json();
      if (!res.ok) {
        pushLog(`auto run failed: ${String(data.error ?? "unknown error")}`);
        return;
      }
      setDetail(data.detail);
      setFormRawInput(String(data.detail?.inputMdRaw ?? ""));
      setAutoModalOpen(false);
      setAutoModalInput("");
      await loadProjects();
      pushLog(String(data.output ?? "auto completed"));
    } finally {
      setAutoRunning(false);
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
        state: data.running ? "run" : "basic",
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

  const isBuildRunning = detail?.state === "build";
  const isReviewState = detail?.state === "review";
  const isAiBusy = formAiGenerating || autoRunning;
  const inputItemRows = detail?.inputItems ?? [];
  const draftsYamlCards = detail?.draftsYamlItems ?? [];
  const hasGreenDraft = draftsYamlCards.some((item) => item.status === "complete");

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
          <button className="text-sm font-semibold text-muted-foreground hover:text-foreground" onClick={() => scrollToProjectSection("code")}>
            code
          </button>
          <button className="text-sm font-semibold text-muted-foreground hover:text-foreground" onClick={() => scrollToProjectSection("monorepo")}>
            monorepo
          </button>
          <button className="text-sm font-semibold text-muted-foreground hover:text-foreground" onClick={() => scrollToProjectSection("video")}>
            video
          </button>
          <button className="text-sm font-semibold text-muted-foreground hover:text-foreground" onClick={() => scrollToProjectSection("write")}>
            write
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
          <div ref={videoSectionRef}>
          <Card className="project-container-pane rounded-2xl">
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <Clapperboard className="h-4 w-4" />
                <span>Video</span>
              </CardTitle>
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={() => openCreateFor("movie")} aria-label="create-video-project">
                  <Plus className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => openLoadFor("movie")} aria-label="load-video-project">
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void openTemplateAssetsModal("video")} aria-label="open-video-template-assets">
                  <Settings className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void loadProjects()} aria-label="refresh-video-projects">
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
                    aria-label="toggle-delete-mode-video"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className={projectItemsContainerClass}>
                {groupedProjects.video.map((p) => renderProjectItemByMode(p))}
              </div>
              {groupedProjects.video.length === 0 && <div className="text-xs text-muted-foreground">no video projects</div>}
            </CardContent>
          </Card>
          </div>
          <div ref={writeSectionRef}>
          <Card className="project-container-pane rounded-2xl">
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <NotebookPen className="h-4 w-4" />
                <span>Write</span>
              </CardTitle>
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={() => openCreateFor("story")} aria-label="create-write-project">
                  <Plus className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => openLoadFor("story")} aria-label="load-write-project">
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void openTemplateAssetsModal("write")} aria-label="open-write-template-assets">
                  <Settings className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void loadProjects()} aria-label="refresh-write-projects">
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
                    aria-label="toggle-delete-mode-write"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className={projectItemsContainerClass}>
                {groupedProjects.write.map((p) => renderProjectItemByMode(p))}
              </div>
              {groupedProjects.write.length === 0 && <div className="text-xs text-muted-foreground">no write projects</div>}
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
                  <span className={`rounded-full border px-2 py-1 text-[11px] uppercase tracking-wide ${stateClass(detail?.state)}`}>
                    {stateLabel(detail?.state)}
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
                {detail?.state === "run" && detail.dev_server_url && (
                  <a
                    href={detail.dev_server_url}
                    target="_blank"
                    rel="noreferrer"
                    className="text-xs font-medium text-blue-600 underline underline-offset-2 hover:text-blue-700"
                  >
                    {detail.dev_server_url}
                  </a>
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
                  disabled={addInputApplying || isAiBusy}
                  aria-label="auto_from_message"
                >
                  <GraduationCap className="h-4 w-4" />
                  <span>auto</span>
                </Button>
                <Button
                  variant="outline"
                  className="h-10 shrink-0 gap-2 px-3 text-sm font-semibold"
                  onClick={() => {
                    setFormRawInput(detail?.inputMdRaw ?? "");
                    setFormAiMessage("");
                    setFormAiDone(false);
                    setAddInputStatus("");
                    setFormAddInputOpen(true);
                  }}
                  disabled={addInputApplying || isBuildRunning || isAiBusy}
                  aria-label="form_add_input"
                >
                  <FilePlus2 className="h-4 w-4" />
                  <span>add</span>
                </Button>
                <Button
                  variant="outline"
                  className="h-10 shrink-0 gap-2 px-3 text-sm font-semibold"
                  onClick={() => {
                    setSelectedPane("project_info");
                    openEditor();
                  }}
                  disabled={addInputApplying || isBuildRunning || isAiBusy}
                  aria-label="modify_project_info"
                >
                  <Pencil className="h-4 w-4" />
                  <span>modify</span>
                </Button>
                <Button
                  variant="outline"
                  className={`h-10 shrink-0 gap-2 px-3 text-sm font-semibold ${
                    isBuildRunning ? "border-red-600 bg-red-600 text-white hover:bg-red-700 hover:text-white" : ""
                  }`}
                  onClick={() => void (isBuildRunning ? stopBuildJob() : startBuildJob())}
                  disabled={addInputApplying || isAiBusy}
                  aria-label="build_parallel"
                >
                  {isBuildRunning ? <Ban className="h-4 w-4" /> : <Hammer className="h-4 w-4" />}
                  <span>{isBuildRunning ? "stop" : "build"}</span>
                </Button>
                <Button
                  variant="outline"
                  className={`h-10 shrink-0 gap-2 px-3 text-sm font-semibold ${
                    detail?.state === "run" ? "border-red-600 bg-red-600 text-white hover:bg-red-700 hover:text-white" : ""
                  }`}
                  onClick={() => void runDevServer()}
                  aria-label="run_project_test"
                >
                  {detail?.state === "run" ? <Ban className="h-4 w-4" /> : <FlaskConical className="h-4 w-4" />}
                  <span>{detail?.state === "run" ? "stop" : "test"}</span>
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
                    <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      value={sidebarSearch}
                      onChange={(e) => setSidebarSearch(e.target.value)}
                      placeholder="search folders..."
                      className="h-9 rounded-xl bg-white pl-8 text-xs"
                      aria-label="detail-sidebar-search-mobile"
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
                <Card className="rounded-2xl bg-white">
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
          <div className={`grid gap-4 lg:grid-cols-[220px_1fr] ${addInputApplying ? "blur-sm" : ""}`}>
          <div className="hidden pt-4 lg:block">
            <div className="mb-1">
              <div className="relative">
                <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={sidebarSearch}
                  onChange={(e) => setSidebarSearch(e.target.value)}
                  placeholder="search folders..."
                  className="h-9 rounded-xl bg-white pl-8 text-xs"
                  aria-label="detail-sidebar-search"
                />
              </div>
            </div>
            <Card className="rounded-2xl bg-white">
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
          <div className="space-y-4 pt-4">
            <DetailLayoutProvider
              detail={detail}
              showProjectInfo={false}
              selectedProject={selectedProject}
              selectedPane={selectedPane}
              setSelectedPane={setSelectedPane}
              selectedDomain={selectedDomain}
              setSelectedDomain={setSelectedDomain}
              openEditor={openEditor}
              memoDraft={memoDraft}
              updateMemo={updateMemoRealtime}
              flushMemo={flushMemo}
              memoSaving={memoSaving}
            />
            <div>
              <div className={sectionLabelClass}>drafts</div>
              <div className="relative">
                <div className="flex flex-wrap items-end justify-end gap-2 lg:absolute lg:right-3 lg:top-0 lg:-translate-y-full">
                  {[
                    { key: "items", label: "items" },
                    { key: "input", label: "input.md" },
                    { key: "drafts", label: "drafts.yaml" }
                  ].map((tab) => (
                    <button
                      key={`draft-tab-${tab.key}`}
                      type="button"
                      onClick={() => setDraftsViewMode(tab.key as "items" | "input" | "drafts")}
                      className={`rounded-t-md border border-b-0 px-3 py-1 text-xs font-semibold uppercase tracking-wide ${
                        draftsViewMode === tab.key
                          ? "border-border bg-white text-foreground"
                          : "border-border/70 bg-muted/20 text-muted-foreground"
                      }`}
                    >
                      {tab.label}
                    </button>
                  ))}
                </div>
              <Card className={`rounded-2xl border border-border lg:border-x lg:border-b lg:border-t-0 ${runningImplDraft ? "bg-amber-50" : "bg-white"}`}>
                <CardContent className="pt-6">
                  <div className="rounded-b-xl border-x border-b border-border bg-white p-3">
                    {draftsViewMode === "items" ? (
                      <div className="grid gap-3 md:grid-cols-[220px_1fr]">
                        <div className="max-h-64 space-y-1 overflow-y-auto rounded-xl border border-border bg-white p-2">
                          {inputItemRows.length === 0 && (
                            <div className="text-xs text-muted-foreground">no input.md headings</div>
                          )}
                          {inputItemRows.map((item) => (
                            <button
                              key={`input-title-${item.title}`}
                              className={`w-full rounded px-2 py-1 text-left text-xs ${
                                selectedInputTitle === item.title
                                  ? "bg-muted font-semibold text-foreground"
                                  : "text-muted-foreground hover:bg-muted/50"
                              }`}
                              onClick={() => setSelectedInputTitle(item.title)}
                            >
                              {item.title}
                            </button>
                          ))}
                        </div>
                        <div className="max-h-64 overflow-y-auto rounded-xl border border-border bg-white p-3 text-xs">
                          {(() => {
                            const selected = inputItemRows.find((item) => item.title === selectedInputTitle);
                            if (!selected) {
                              return <div className="text-muted-foreground">input.md 항목을 선택하세요.</div>;
                            }
                            return (
                              <div className="space-y-2">
                                <div>
                                  <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">title</div>
                                  <div className="mt-1 text-sm text-foreground">{selected.title}</div>
                                </div>
                                <div>
                                  <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">rule</div>
                                  <div className="mt-1 text-sm text-foreground">{selected.rule || "-"}</div>
                                </div>
                                <div>
                                  <div className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">step</div>
                                  <div className="mt-1 text-sm text-foreground">{selected.step || "-"}</div>
                                </div>
                              </div>
                            );
                          })()}
                        </div>
                      </div>
                    ) : draftsViewMode === "input" ? (
                      <pre className="max-h-64 overflow-y-auto rounded-xl border border-border bg-white p-3 text-xs leading-relaxed text-foreground/80">
                        {detail?.inputMdRaw?.trim() || "# input.md not found"}
                      </pre>
                    ) : (
                      <div className="max-h-64 overflow-y-auto rounded-xl border border-border bg-white p-2">
                        {draftsYamlCards.length === 0 && (
                          <div className="px-1 py-2 text-xs text-muted-foreground">no drafts.yaml items</div>
                        )}
                        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
                          {draftsYamlCards.map((item) => (
                            <DraftYamlItemCard
                              key={`drafts-yaml-item-${item.name}`}
                              item={item}
                              onClick={() => setSelectedDraftYamlItem({ name: item.name, draft: item.draft })}
                            />
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                  <div className="mt-3 flex justify-end gap-2">
                    <Button
                      variant="outline"
                      className="h-9 gap-2 px-3 text-sm font-semibold"
                      onClick={() => void runQuickAction("check_code")}
                      disabled={!isReviewState || addInputApplying || isBuildRunning}
                      aria-label="check_code_review"
                    >
                      <CheckCircle2 className="h-4 w-4" />
                      <span>check</span>
                    </Button>
                    <Button
                      variant="outline"
                      className="h-9 gap-2 px-3 text-sm font-semibold"
                      onClick={() => void runQuickAction("retry_incomplete")}
                      disabled={!isReviewState || addInputApplying || isBuildRunning}
                      aria-label="retry_red_items"
                    >
                      <RotateCcw className="h-4 w-4" />
                      <span>retry red</span>
                    </Button>
                    <Button
                      variant="outline"
                      className="h-9 gap-2 px-3 text-sm font-semibold"
                      onClick={() => void runQuickAction("finalize_complete")}
                      disabled={!isReviewState || !hasGreenDraft || addInputApplying || isBuildRunning}
                      aria-label="finalize_green_items"
                    >
                      <CheckCircle2 className="h-4 w-4" />
                      <span>complete</span>
                    </Button>
                  </div>
                </CardContent>
              </Card>
              </div>
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
                input.md 반영중...
              </div>
            </div>
          )}
        </div>
        </div>
      )}

      <DraftYamlItemModal
        item={selectedDraftYamlItem}
        onClose={() => {
          setSelectedDraftYamlItem(null);
        }}
      />

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

      {formAddInputOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="relative flex h-[80vh] max-h-[1100px] w-full max-w-4xl flex-col rounded-2xl">
            <CardHeader>
              <CardTitle>form_add_input</CardTitle>
            </CardHeader>
            <CardContent className={`flex min-h-0 flex-1 flex-col gap-3 overflow-hidden ${formAiGenerating ? "blur-sm" : ""}`}>
              <div className="flex min-h-0 flex-1 flex-col space-y-3 overflow-hidden rounded-xl border border-border p-3">
                <Label>raw input.md (# / - / &gt;)</Label>
                <Textarea
                  value={formRawInput}
                  onChange={(e) => handleRawInputChange(e.target.value)}
                  className="min-h-[220px] flex-1"
                  placeholder={"# title\n- rule > step"}
                />
                <div className="flex items-center gap-2">
                  <Input
                    value={formAiMessage}
                    onChange={(e) => setFormAiMessage(e.target.value)}
                    placeholder="메시지로 input.md 생성"
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        void generateInputMdWithAi();
                      }
                    }}
                  />
                  <Button
                    type="button"
                    variant="outline"
                    className="gap-2"
                    onClick={() => void generateInputMdWithAi()}
                    disabled={formAiGenerating}
                    aria-label="generate-input-md-with-ai"
                  >
                    <span>ai</span>
                    <Sparkles className="h-4 w-4" />
                  </Button>
                </div>
                <div className="flex items-center justify-between text-xs text-muted-foreground">
                  <span>입력 즉시 input.md로 저장됩니다.</span>
                  {formAiDone && <span className="font-semibold text-emerald-600">input.md 생성 완료</span>}
                </div>
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  onClick={async () => {
                    if (!detail) return;
                    setFormAddInputOpen(false);
                    setAddInputApplying(true);
                    setAddInputStatus("input.md 반영중...");
                    if (rawInputSaveTimerRef.current) {
                      clearTimeout(rawInputSaveTimerRef.current);
                      rawInputSaveTimerRef.current = null;
                    }
                    try {
                      const res = await fetch(apiUrl("/api/input-md-raw"), {
                        method: "POST",
                        headers: { "content-type": "application/json" },
                        body: JSON.stringify({
                          id: detail.id,
                          raw: formRawInput,
                          apply: true
                        })
                      });
                      const data = await res.json();
                      if (!res.ok) {
                        pushLog(`raw input apply failed: ${String(data.error ?? "unknown error")}`);
                        setAddInputStatus("실패");
                        return;
                      }
                      if (Array.isArray(data.stages)) {
                        for (const line of data.stages) {
                          pushLog(`[form_add_input] ${String(line)}`);
                        }
                      }
                      setDetail(data.detail);
                      setAddInputStatus("완료");
                      await loadProjects();
                    } finally {
                      setAddInputApplying(false);
                    }
                  }}
                >
                  확인
                </Button>
                <Button
                  variant="outline"
                  disabled={formAiGenerating}
                  onClick={() => {
                    if (rawInputSaveTimerRef.current) {
                      clearTimeout(rawInputSaveTimerRef.current);
                      rawInputSaveTimerRef.current = null;
                    }
                    setFormAddInputOpen(false);
                  }}
                >
                  Cancel
                </Button>
              </div>
            </CardContent>
            {formAiGenerating && (
              <div className="absolute inset-0 z-10 flex items-center justify-center rounded-2xl bg-white/45">
                <div className="rounded-xl border border-border bg-white px-4 py-2 text-sm font-semibold text-foreground shadow">
                  AI 작업중...
                </div>
              </div>
            )}
          </Card>
        </div>
      )}

      {autoModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
          <Card className="relative flex h-[70vh] max-h-[900px] w-full max-w-3xl flex-col rounded-2xl">
            <CardHeader>
              <CardTitle>auto_from_message</CardTitle>
            </CardHeader>
            <CardContent className={`flex min-h-0 flex-1 flex-col gap-3 overflow-hidden ${autoRunning ? "blur-sm" : ""}`}>
              <div className="flex min-h-0 flex-1 flex-col space-y-3 rounded-xl border border-border p-3">
                <Label>요청 메시지</Label>
                <Textarea
                  value={autoModalInput}
                  onChange={(e) => setAutoModalInput(e.target.value)}
                  className="min-h-[260px] flex-1"
                  placeholder="요청 내용을 입력하세요"
                />
                <div className="flex justify-end">
                  <Button type="button" variant="outline" onClick={() => void runAutoFlowFromMessage()} disabled={autoRunning}>
                    요청하기
                  </Button>
                </div>
              </div>
              <div className="flex justify-end gap-2">
                <Button onClick={() => void runAutoFlowFromMessage()} disabled={autoRunning}>
                  확인
                </Button>
                <Button variant="outline" onClick={() => setAutoModalOpen(false)} disabled={autoRunning}>
                  Cancel
                </Button>
              </div>
            </CardContent>
            {autoRunning && (
              <div className="absolute inset-0 z-10 flex items-center justify-center rounded-2xl bg-white/45">
                <div className="rounded-xl border border-border bg-white px-4 py-2 text-sm font-semibold text-foreground shadow">
                  AI 자동 작업중...
                </div>
              </div>
            )}
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
