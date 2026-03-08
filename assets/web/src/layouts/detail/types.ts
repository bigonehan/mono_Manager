import type { Detail, DetailPane, Project } from "@/store/orc-store";

export type DetailLayoutType = "code" | "write" | "mono" | "movie";

export type DetailLayoutProps = {
  detail: Detail | null;
  showProjectInfo?: boolean;
  selectedPane: DetailPane;
  setSelectedPane: (pane: DetailPane) => void;
  selectedDomain: string;
  setSelectedDomain: (domain: string) => void;
  openEditor: () => void;
  memoDraft: string;
  updateMemo: (value: string) => void;
  flushMemo: () => void;
  memoSaving: boolean;
};

export function parseSpecTokens(input: string): string[] {
  return input
    .split(",")
    .map((v) => v.trim())
    .filter((v) => v.length > 0);
}

export function resolveDetailLayoutType(
  detail: Detail | null,
  selectedProject: Project | null
): DetailLayoutType {
  const projectType = detail?.project_type ?? selectedProject?.project_type ?? "code";
  if (projectType === "mono") return "mono";
  if (projectType === "movie") return "movie";
  if (projectType === "story") return "write";
  return "code";
}
