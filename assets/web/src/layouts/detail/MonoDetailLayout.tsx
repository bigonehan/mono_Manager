import { Boxes, Settings } from "lucide-react";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";
import { DetailTabsPane } from "@/layouts/detail/DetailTabsPane";

const sectionLabelClass = "mt-8 mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";

function getMonorepoDomainsHardcoded(_projectPath: string): string[] {
  return ["accounts", "catalog", "orders", "payment", "shared"];
}

export function MonoDetailLayout({ detail, showProjectInfo = true, selectedPane, setSelectedPane, openEditor }: DetailLayoutProps) {
  const domains = getMonorepoDomainsHardcoded(detail?.path ?? "");
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
      )}
      <DetailTabsPane
        rules={detail?.rules ?? []}
        constraints={detail?.constraints ?? []}
        features={detail?.features ?? []}
        selectedPane={selectedPane}
        setSelectedPane={setSelectedPane}
        openEditor={openEditor}
      />
      <div>
        <div className={sectionLabelClass}>domains</div>
        <section className="rounded-2xl border border-border bg-white p-4 text-sm">
          <div className="flex flex-wrap gap-2">
            {domains.map((domain) => (
              <span key={domain} className="rounded-md border border-border px-2 py-1 text-xs font-semibold">
                {domain}
              </span>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
