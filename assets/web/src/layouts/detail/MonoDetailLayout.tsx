import { Boxes, Settings } from "lucide-react";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";

export function MonoDetailLayout({ detail, selectedPane, setSelectedPane, openEditor }: DetailLayoutProps) {
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
        <div className="flex items-start justify-between gap-3">
          <div>
            <div className="text-4xl font-extrabold tracking-tight">{detail?.name ?? ""}</div>
            <div className="mt-1 text-xl text-muted-foreground">{detail?.description ?? ""}</div>
          </div>
          <span className="rounded-lg bg-muted px-3 py-1 text-xs font-bold uppercase">{detail?.state ?? "basic"}</span>
        </div>
        <div className="mt-6 flex flex-wrap gap-2">
          <span className="inline-flex rounded-xl border border-border bg-muted p-2 text-foreground">
            <Boxes className="h-5 w-5" />
          </span>
          {parseSpecTokens(detail?.spec ?? "").map((token) => (
            <span key={token} className="rounded-md border border-border px-2 py-1 text-sm font-medium">
              {token}
            </span>
          ))}
        </div>
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
