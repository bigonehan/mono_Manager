import { Clapperboard, Settings } from "lucide-react";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";

export function MovieDetailLayout({ detail, selectedPane, setSelectedPane, openEditor }: DetailLayoutProps) {
  return (
    <div className="space-y-4">
      <section
        data-testid="detail-pane-project"
        onClick={() => setSelectedPane("project_info")}
        className={`relative rounded-2xl border p-5 text-sm ${
          selectedPane === "project_info" ? "border-primary" : "border-border"
        }`}
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
          <span className="text-xs uppercase">movie detail layout</span>
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
      <div className="grid gap-4 lg:grid-cols-3">
        {[
          { key: "rules", title: "rules", values: detail?.rules ?? [] },
          { key: "constraints", title: "constraints", values: detail?.constraints ?? [] },
          { key: "features", title: "features", values: detail?.features ?? [] }
        ].map((pane) => (
          <section
            key={pane.key}
            data-testid={`detail-pane-${pane.key}`}
            onClick={() => setSelectedPane(pane.key as "rules" | "constraints" | "features")}
            className={`relative rounded-2xl border p-4 text-sm ${
              selectedPane === pane.key ? "border-primary" : "border-border"
            }`}
          >
            {selectedPane === pane.key && (
              <button
                data-testid={`pane-edit-gear-${pane.key}`}
                className="absolute right-2 top-2 rounded p-1 text-muted-foreground hover:bg-muted"
                onClick={(e) => {
                  e.stopPropagation();
                  openEditor();
                }}
                aria-label={`edit-pane-${pane.key}`}
              >
                <Settings className="h-4 w-4" />
              </button>
            )}
            <div className="mb-2 font-semibold">{pane.title}</div>
            {pane.values.map((v) => (
              <div key={`${pane.key}-${v}`}>- {v}</div>
            ))}
          </section>
        ))}
      </div>
    </div>
  );
}
