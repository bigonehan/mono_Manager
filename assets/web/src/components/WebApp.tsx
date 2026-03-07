import { useEffect, useMemo, useRef, useState } from "react";
import {
  Clapperboard,
  Code2,
  CornerUpLeft,
  FilePlus2,
  FolderOpen,
  Hammer,
  NotebookPen,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Shapes,
  ShieldCheck,
  Trash2,
  X
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { useOrcStore, type Project } from "@/store/orc-store";
import { DetailLayoutProvider } from "@/layouts/detail";

const sectionLabelClass = "mt-8 mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";
const projectContainerItemClass =
  "project-container-item relative rounded-xl border border-border bg-card p-3 text-left text-sm hover:bg-muted/40";
type DraftModalAction = "add_draft" | "impl_draft" | "check_code";
type BrowseTarget = "create" | "load";
type BrowseEntry = { name: string; path: string; hasProjectMeta: boolean };

function stateLabel(state?: Project["state"]): string {
  if (state === "init") return "init";
  if (state === "work") return "work";
  if (state === "wait") return "wait";
  return "basic";
}

function stateClass(state?: Project["state"]): string {
  if (state === "init") return "border-sky-500/50 bg-sky-500/10 text-sky-700";
  if (state === "work") return "border-amber-500/50 bg-amber-500/10 text-amber-700";
  if (state === "wait") return "border-border text-muted-foreground";
  return "border-emerald-500/50 bg-emerald-500/10 text-emerald-700";
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
  const lastSavedMemoRef = useRef("");
  const codeSectionRef = useRef<HTMLDivElement | null>(null);
  const monorepoSectionRef = useRef<HTMLDivElement | null>(null);
  const videoSectionRef = useRef<HTMLDivElement | null>(null);
  const writeSectionRef = useRef<HTMLDivElement | null>(null);
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
    setTab,
    setProjects,
    setSelectedId,
    setDetail,
    setSelectedPane,
    pushLog,
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
    setEditFeatures
  } = useOrcStore();
  const isCreateOpen = createOpen || createOpenLocal;

  const selectedProject = useMemo(
    () => projects.find((p) => p.id === selectedId) ?? null,
    [projects, selectedId]
  );
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

  async function loadProjects() {
    const res = await fetch(apiUrl("/api/projects"));
    const data = await res.json();
    const next: Project[] = data.projects ?? [];
    setProjects(next);
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

  function renderProjectContainerItem(p: Project) {
    return (
      <div
        key={p.id}
        data-testid={`project-item-${p.id}`}
        className={`${projectContainerItemClass} ${selectedProject?.id === p.id ? "border-primary bg-secondary/70" : ""}`}
        onClick={() => {
          void markSelected(p.id);
        }}
        onDoubleClick={() => {
          void markSelected(p.id);
          setTab("detail");
        }}
      >
        <div className="mb-1 flex items-center gap-2 pr-16">
          <ProjectTypeIcon type={p.project_type} />
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
        <div className="text-xs text-muted-foreground">{compactPath(p.path)}</div>
        <div className="mt-1 text-[11px] text-muted-foreground">{p.description}</div>
        <div className="mt-2">
          <span className={`rounded-full border px-2 py-1 text-[11px] uppercase tracking-wide ${stateClass(p.state)}`}>
            {stateLabel(p.state)}
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
        <div className="space-y-4">
          <div ref={codeSectionRef}>
            <Card className="project-container-pane rounded-2xl">
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>Code</CardTitle>
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
                <Button variant="outline" size="sm" onClick={() => void loadProjects()} aria-label="refresh-projects">
                  <RefreshCw className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-5">
                {groupedProjects.code.map((p) => renderProjectContainerItem(p))}
              </div>
              {groupedProjects.code.length === 0 && <div className="text-xs text-muted-foreground">no code projects</div>}
            </CardContent>
          </Card>
          </div>
          <div ref={monorepoSectionRef}>
          <Card className="project-container-pane rounded-2xl">
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>Monorepo</CardTitle>
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={() => openCreateFor("mono")} aria-label="create-monorepo-project">
                  <Plus className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => openLoadFor("mono")} aria-label="load-monorepo-project">
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void loadProjects()} aria-label="refresh-monorepo-projects">
                  <RefreshCw className="h-4 w-4" />
                </Button>
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
                  <div className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-5">
                    {sidebarMonorepoGroups.app.map((p) => renderProjectContainerItem(p))}
                    {sidebarMonorepoGroups.app.length === 0 && <div className="text-xs text-muted-foreground">no app packages</div>}
                  </div>
                </div>
                <div>
                  <div className="mb-2 text-xs font-bold uppercase tracking-wide text-muted-foreground">feature</div>
                  <div className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-5">
                    {sidebarMonorepoGroups.feature.map((p) => renderProjectContainerItem(p))}
                    {sidebarMonorepoGroups.feature.length === 0 && <div className="text-xs text-muted-foreground">no feature packages</div>}
                  </div>
                </div>
                <div>
                  <div className="mb-2 text-xs font-bold uppercase tracking-wide text-muted-foreground">templates</div>
                  <div className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-5">
                    {sidebarMonorepoGroups.template.map((p) => renderProjectContainerItem(p))}
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
              <CardTitle>Video</CardTitle>
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={() => openCreateFor("movie")} aria-label="create-video-project">
                  <Plus className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => openLoadFor("movie")} aria-label="load-video-project">
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void loadProjects()} aria-label="refresh-video-projects">
                  <RefreshCw className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-5">
                {groupedProjects.video.map((p) => renderProjectContainerItem(p))}
              </div>
              {groupedProjects.video.length === 0 && <div className="text-xs text-muted-foreground">no video projects</div>}
            </CardContent>
          </Card>
          </div>
          <div ref={writeSectionRef}>
          <Card className="project-container-pane rounded-2xl">
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>Write</CardTitle>
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={() => openCreateFor("story")} aria-label="create-write-project">
                  <Plus className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => openLoadFor("story")} aria-label="load-write-project">
                  <FolderOpen className="h-4 w-4" />
                </Button>
                <Button variant="outline" size="sm" onClick={() => void loadProjects()} aria-label="refresh-write-projects">
                  <RefreshCw className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="grid grid-cols-1 gap-2 md:grid-cols-2 xl:grid-cols-5">
                {groupedProjects.write.map((p) => renderProjectContainerItem(p))}
              </div>
              {groupedProjects.write.length === 0 && <div className="text-xs text-muted-foreground">no write projects</div>}
            </CardContent>
          </Card>
          </div>
        </div>
      ) : (
        <div className="grid gap-4 lg:grid-cols-[220px_1fr]">
          <div>
            <div className={sectionLabelClass}>projects</div>
            <Card className="rounded-2xl bg-white">
              <CardContent className="space-y-2 pt-6">
                {selectedProject?.project_type === "mono" ? (
                  <div className="space-y-3">
                    <div>
                      <div className="px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">app</div>
                      <div className="mt-1 space-y-1">
                        {sidebarMonorepoGroups.app.map((p) => (
                          <button
                            key={`detail-sidebar-app-${p.id}`}
                            className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                              selectedProject?.id === p.id ? "bg-muted font-semibold text-foreground" : "text-muted-foreground hover:bg-muted/50"
                            }`}
                            onClick={() => {
                              void markSelected(p.id);
                            }}
                          >
                            {p.name}
                          </button>
                        ))}
                      </div>
                    </div>
                    <div>
                      <div className="px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">feature</div>
                      <div className="mt-1 space-y-1">
                        {sidebarMonorepoGroups.feature.map((p) => (
                          <button
                            key={`detail-sidebar-feature-${p.id}`}
                            className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                              selectedProject?.id === p.id ? "bg-muted font-semibold text-foreground" : "text-muted-foreground hover:bg-muted/50"
                            }`}
                            onClick={() => {
                              void markSelected(p.id);
                            }}
                          >
                            {p.name}
                          </button>
                        ))}
                      </div>
                    </div>
                    <div>
                      <div className="px-1 text-[11px] font-bold uppercase tracking-wide text-muted-foreground">templates</div>
                      <div className="mt-1 space-y-1">
                        {sidebarMonorepoGroups.template.map((p) => (
                          <button
                            key={`detail-sidebar-template-${p.id}`}
                            className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                              selectedProject?.id === p.id ? "bg-muted font-semibold text-foreground" : "text-muted-foreground hover:bg-muted/50"
                            }`}
                            onClick={() => {
                              void markSelected(p.id);
                            }}
                          >
                            {p.name}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                ) : (
                  projects.map((p) => (
                    <button
                      key={`detail-sidebar-${p.id}`}
                      className={`w-full rounded-lg px-3 py-2 text-left text-sm ${
                        selectedProject?.id === p.id ? "bg-muted font-semibold text-foreground" : "text-muted-foreground hover:bg-muted/50"
                      }`}
                      onClick={() => {
                        void markSelected(p.id);
                      }}
                    >
                      {p.name}
                    </button>
                  ))
                )}
              </CardContent>
            </Card>
          </div>
          <div className="space-y-4">
            <DetailLayoutProvider
              detail={detail}
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
              <Card className={`rounded-2xl ${runningImplDraft ? "bg-amber-50" : "bg-white"}`}>
                <CardContent className="pt-6">
                  <div className="flex items-center gap-2">
                    <Button variant="outline" size="icon" onClick={() => setDraftModalAction("add_draft")} aria-label="add_code_draft">
                      <FilePlus2 className="h-4 w-4" />
                    </Button>
                    <Button variant="outline" size="icon" onClick={() => setDraftModalAction("impl_draft")} aria-label="impl_code_draft">
                      <Hammer className="h-4 w-4" />
                    </Button>
                    <Button variant="outline" size="icon" onClick={() => setDraftModalAction("check_code")} aria-label="check_code_draft">
                      <ShieldCheck className="h-4 w-4" />
                    </Button>
                  </div>
                </CardContent>
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
                <Button variant="outline" onClick={() => setEditOpen(false)}>
                  Cancel
                </Button>
                <Button data-testid="edit-save" onClick={() => void saveEditor()}>
                  Save
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
              <CardTitle className="capitalize">{draftModalAction.replace("_", " ")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {draftModalAction === "add_draft" && (
                <>
                  <Label>add_code_draft payload (optional)</Label>
                  <Input
                    value={addDraftPayload}
                    onChange={(e) => setAddDraftPayload(e.target.value)}
                    placeholder="feature 메시지 입력"
                  />
                </>
              )}
              <div className="flex justify-end gap-2">
                <Button variant="outline" onClick={() => setDraftModalAction(null)}>
                  Cancel
                </Button>
                <Button
                  onClick={async () => {
                    const ok = await runAction(draftModalAction);
                    if (ok) setDraftModalAction(null);
                  }}
                >
                  Run
                </Button>
              </div>
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
                <Button
                  variant="outline"
                  onClick={() => {
                    setCreateOpen(false);
                    setCreateOpenLocal(false);
                  }}
                >
                  Cancel
                </Button>
                <Button data-testid="create-project" onClick={() => void createProject()}>
                  Create Project
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
                <Button
                  variant="outline"
                  onClick={() => {
                    setLoadOpen(false);
                    setLoadPath("");
                  }}
                >
                  Cancel
                </Button>
                <Button onClick={() => void loadProjectByPath(false)}>Load</Button>
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
