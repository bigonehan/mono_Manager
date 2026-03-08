import { Clapperboard, Settings } from "lucide-react";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";
import { DetailTabsPane } from "@/layouts/detail/DetailTabsPane";

export function MovieDetailLayout({ detail, showProjectInfo = true, selectedPane, setSelectedPane, openEditor }: DetailLayoutProps) {
  return (
    <div className="space-y-4">
      {showProjectInfo && (
      <section
        data-testid="detail-pane-project"
        onClick={() => setSelectedPane("project_info")}
        className="relative border-b border-border pb-5 pt-5 text-sm"
      >
        {selectedPane === "project_info" && (
          <button
            data-testid="pane-edit-gear"
            className="absolute right-2 top-2 rounded p-1 text-muted-foreground hover:bg-muted"
            onClick={(e) => {
              e.stopPropagation();
              openEditor();
            }}
            aria-label="edit-pane"
          >
            <Settings className="h-4 w-4" />
          </button>
        )}
        <div className="flex items-center gap-2 text-muted-foreground">
          <Clapperboard className="h-5 w-5" />
          <span className="text-xs uppercase">video detail layout</span>
        </div>
        <div className="mt-2 text-4xl font-black">{detail?.name ?? ""}</div>
        <div className="mt-1 text-xl text-muted-foreground">{detail?.description ?? ""}</div>
        <div className="mt-6 flex flex-wrap gap-2">
          {parseSpecTokens(detail?.spec ?? "").map((token) => (
            <span key={token} className="rounded-xl bg-muted px-3 py-1 text-sm font-semibold">
              {token}
            </span>
          ))}
        </div>
        <div className="mt-4 text-sm">goal: {detail?.goal ?? ""}</div>
      </section>
      )}
      <DetailTabsPane
        rules={detail?.rules ?? []}
        constraints={detail?.constraints ?? []}
        features={detail?.features ?? []}
        selectedPane={selectedPane}
        setSelectedPane={setSelectedPane}
        openEditor={openEditor}
      />
    </div>
  );
}
