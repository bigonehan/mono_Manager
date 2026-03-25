import { NotebookPen, Pencil } from "lucide-react";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";
import { DetailTabsPane } from "@/layouts/detail/DetailTabsPane";

export function WriteDetailLayout({
  detail,
  showProjectInfo = true,
  selectedPane,
  setSelectedPane,
  openEditor,
  actionsDisabled
}: DetailLayoutProps) {
  return (
    <div className="space-y-4">
      {showProjectInfo && (
      <section
        data-testid="detail-pane-project"
        onClick={() => setSelectedPane("project_info")}
        className="relative border-b border-border bg-card/80 pb-12 pt-5 text-sm"
      >
        {selectedPane === "project_info" && (
          <button
            data-testid="pane-edit-pencil"
            className="absolute bottom-2 right-2 inline-flex h-9 w-9 items-center justify-center rounded-full bg-emerald-600 text-white shadow-sm hover:bg-emerald-700"
            onClick={(e) => {
              e.stopPropagation();
              openEditor();
            }}
            disabled={actionsDisabled}
            aria-label="edit-pane"
          >
            <Pencil className="h-4 w-4" />
          </button>
        )}
        <div className="text-3xl font-black">{detail?.name ?? ""}</div>
        <div className="mt-2 text-lg text-muted-foreground">{detail?.description ?? ""}</div>
        <div className="mt-5 flex flex-wrap items-center gap-2">
          <NotebookPen className="h-5 w-5" />
          {parseSpecTokens(detail?.spec ?? "").map((token) => (
            <span key={token} className="rounded-full border border-border px-3 py-1 text-sm font-semibold">
              {token}
            </span>
          ))}
        </div>
        <div className="mt-4 text-sm text-muted-foreground">goal: {detail?.goal ?? ""}</div>
      </section>
      )}
      <DetailTabsPane
        rules={detail?.rules ?? []}
        constraints={detail?.constraints ?? []}
        features={detail?.features ?? []}
        selectedPane={selectedPane}
        setSelectedPane={setSelectedPane}
        openEditor={openEditor}
        actionsDisabled={actionsDisabled}
      />
    </div>
  );
}
