import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ProjectState = "wait" | "work" | "complete" | "auto";

export type Project = {
  id: string;
  name: string;
  path: string;
  description: string;
  selected: boolean;
  project_type: "code" | "mono";
  state?: ProjectState;
  current_job?: string;
  is_dev_running?: boolean;
  is_build_running?: boolean;
};

export type Detail = {
  id: string;
  name: string;
  description: string;
  path: string;
  memo: string;
  project_type: "code" | "mono";
  spec: string;
  goal: string;
  rules: string[];
  constraints: string[];
  features: string[];
  domains: Array<{ name: string; description: string; features: string[]; is_active?: boolean }>;
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
  draftItems?: Array<Record<string, unknown>>;
  draftsYamlItems?: Array<{
    name: string;
    status: "work" | "wait" | "complete";
    draft: Record<string, unknown>;
  }>;
  checkSubject?: string;
  checkSteps?: Array<{
    subject: string;
    text: string;
    source: string;
  }>;
  feedbackMdRaw?: string;
  hasFeedbackMd?: boolean;
  screenshots?: Array<{
    name: string;
    path: string;
    url: string;
    modifiedAt: string;
  }>;
};

export type AppTab = "project" | "detail";
export type DetailPane = "project_info" | "rules" | "constraints" | "features";

type OrcStore = {
  tab: AppTab;
  projects: Project[];
  selectedId: string;
  detail: Detail | null;
  selectedPane: DetailPane;
  logs: string[];

  newName: string;
  newDescription: string;
  newPath: string;
  newSpec: string;
  addDraftPayload: string;
  createOpen: boolean;

  editOpen: boolean;
  selectedDomain: string;
  editName: string;
  editDescription: string;
  editSpec: string;
  editGoal: string;
  editRules: string;
  editConstraints: string;
  editFeatures: string;
  activeRunProjectIds: string[];
  activeAutoProjectIds: string[];

  setTab: (v: AppTab) => void;
  setProjects: (v: Project[] | ((prev: Project[]) => Project[])) => void;
  setSelectedId: (v: string) => void;
  setDetail: (v: Detail | null | ((prev: Detail | null) => Detail | null)) => void;
  setSelectedPane: (v: DetailPane) => void;
  pushLog: (line: string) => void;
  setLogs: (lines: string[]) => void;

  setNewName: (v: string) => void;
  setNewDescription: (v: string) => void;
  setNewPath: (v: string) => void;
  setNewSpec: (v: string) => void;
  resetNewProjectForm: () => void;
  setAddDraftPayload: (v: string) => void;
  setCreateOpen: (v: boolean) => void;

  setEditOpen: (v: boolean) => void;
  setSelectedDomain: (v: string) => void;
  setEditName: (v: string) => void;
  setEditDescription: (v: string) => void;
  setEditSpec: (v: string) => void;
  setEditGoal: (v: string) => void;
  setEditRules: (v: string) => void;
  setEditConstraints: (v: string) => void;
  setEditFeatures: (v: string) => void;
  setActiveRunProjectIds: (v: string[] | ((prev: string[]) => string[])) => void;
  setActiveAutoProjectIds: (v: string[] | ((prev: string[]) => string[])) => void;
};

export const useOrcStore = create<OrcStore>()(
  persist(
    (set) => ({
      tab: "project",
      projects: [],
      selectedId: "",
      detail: null,
      selectedPane: "project_info",
      logs: [],

      newName: "",
      newDescription: "",
      newPath: "",
      newSpec: "",
      addDraftPayload: "",
      createOpen: false,

      editOpen: false,
      selectedDomain: "",
      editName: "",
      editDescription: "",
      editSpec: "",
      editGoal: "",
      editRules: "",
      editConstraints: "",
      editFeatures: "",
      activeRunProjectIds: [],
      activeAutoProjectIds: [],

      setTab: (v) => set({ tab: v }),
      setProjects: (v) =>
        set((state) => ({
          projects: typeof v === "function" ? (v as (prev: Project[]) => Project[])(state.projects) : v
        })),
      setSelectedId: (v) => set({ selectedId: v }),
      setDetail: (v) =>
        set((state) => ({
          detail: typeof v === "function" ? (v as (prev: Detail | null) => Detail | null)(state.detail) : v
        })),
      setSelectedPane: (v) => set({ selectedPane: v }),
      pushLog: (line) => set((s) => ({ logs: [line, ...s.logs].slice(0, 80) })),
      setLogs: (lines) => set({ logs: lines.slice(0, 200) }),

      setNewName: (v) => set({ newName: v }),
      setNewDescription: (v) => set({ newDescription: v }),
      setNewPath: (v) => set({ newPath: v }),
      setNewSpec: (v) => set({ newSpec: v }),
      resetNewProjectForm: () => set({ newName: "", newDescription: "", newPath: "", newSpec: "" }),
      setAddDraftPayload: (v) => set({ addDraftPayload: v }),
      setCreateOpen: (v) => set({ createOpen: v }),

      setEditOpen: (v) => set({ editOpen: v }),
      setSelectedDomain: (v) => set({ selectedDomain: v }),
      setEditName: (v) => set({ editName: v }),
      setEditDescription: (v) => set({ editDescription: v }),
      setEditSpec: (v) => set({ editSpec: v }),
      setEditGoal: (v) => set({ editGoal: v }),
      setEditRules: (v) => set({ editRules: v }),
      setEditConstraints: (v) => set({ editConstraints: v }),
      setEditFeatures: (v) => set({ editFeatures: v }),
      setActiveRunProjectIds: (v) =>
        set((state) => ({
          activeRunProjectIds:
            typeof v === "function" ? (v as (prev: string[]) => string[])(state.activeRunProjectIds) : v
        })),
      setActiveAutoProjectIds: (v) =>
        set((state) => ({
          activeAutoProjectIds:
            typeof v === "function" ? (v as (prev: string[]) => string[])(state.activeAutoProjectIds) : v
        }))
    }),
    {
      name: "orc-web-store",
      partialize: (state) => ({
        activeRunProjectIds: state.activeRunProjectIds,
        activeAutoProjectIds: state.activeAutoProjectIds
      })
    }
  )
);
