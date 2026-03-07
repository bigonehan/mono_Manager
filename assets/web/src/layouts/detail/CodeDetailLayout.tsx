import { Folder, ListChecks, Settings, ShieldAlert, Sparkles } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";

const sectionLabelClass =
  "mt-8 mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";
const paneShellClass = "rounded-2xl border border-border/70 bg-white";

export function CodeDetailLayout({
  detail,
  selectedPane,
  setSelectedPane,
  selectedDomain,
  setSelectedDomain,
  openEditor,
  memoDraft,
  updateMemo,
  flushMemo,
  memoSaving
}: DetailLayoutProps) {
  return (
    <>
      <div>
        <div
          data-testid="detail-pane-project"
          onClick={() => setSelectedPane("project_info")}
          className="relative px-2 py-1 text-sm"
        >
          {selectedPane === "project_info" && (
            <button
              data-testid="pane-edit-gear"
              className="absolute right-2 top-0 rounded p-1 text-muted-foreground hover:bg-muted"
              onClick={(e) => {
                e.stopPropagation();
                openEditor();
              }}
              aria-label="edit-pane"
            >
              <Settings className="h-4 w-4" />
            </button>
          )}
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0">
              <div data-testid="detail-project-name" className="text-5xl font-extrabold tracking-tight text-foreground">
                {detail?.name ?? ""}
              </div>
              <div className="my-3 text-sm text-muted-foreground">{detail?.description ?? ""}</div>
              <div className="mt-2 flex items-center justify-between gap-3">
                <div className="flex flex-wrap gap-2">
                  {(parseSpecTokens(detail?.spec ?? "").length > 0
                    ? parseSpecTokens(detail?.spec ?? "")
                    : ["(empty)"]
                  ).map((token) => (
                    <span
                      key={token}
                      className="rounded-full border border-border bg-white px-2 py-1 text-xs text-foreground/80"
                    >
                      {token}
                    </span>
                  ))}
                </div>
                <div className="flex max-w-[45%] items-center gap-2 text-sm text-muted-foreground">
                  <Folder className="h-4 w-4 shrink-0" />
                  <span className="truncate text-right">{detail?.path ?? ""}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <MemoPane memoDraft={memoDraft} updateMemo={updateMemo} flushMemo={flushMemo} memoSaving={memoSaving} />
      <ThreePaneLists
        detail={detail}
        selectedPane={selectedPane}
        setSelectedPane={setSelectedPane}
        openEditor={openEditor}
      />
      <DomainsPane
        detail={detail}
        selectedDomain={selectedDomain}
        setSelectedDomain={setSelectedDomain}
      />
    </>
  );
}

type ListProps = DetailLayoutProps;

function ThreePaneLists({ detail, selectedPane, setSelectedPane, openEditor }: ListProps) {
  return (
    <div>
      <div className={sectionLabelClass}>detail</div>
      <section className={`p-2 text-sm ${paneShellClass}`}>
      <div className="grid gap-0 md:grid-cols-3 md:divide-x md:divide-border">
      <section
        data-testid="detail-pane-rules"
        onClick={() => setSelectedPane("rules")}
        className={`relative p-4 text-sm ${selectedPane === "rules" ? "bg-muted/15" : ""}`}
      >
        {selectedPane === "rules" && (
          <button
            data-testid="pane-edit-gear-rules"
            className="absolute right-2 top-2 rounded p-1 text-muted-foreground hover:bg-muted"
            onClick={(e) => {
              e.stopPropagation();
              openEditor();
            }}
            aria-label="edit-pane-rules"
          >
            <Settings className="h-4 w-4" />
          </button>
        )}
        <div className="mb-2 flex items-center gap-2 text-sm font-bold uppercase tracking-wide text-foreground">
          <ListChecks className="h-3.5 w-3.5" />
          <span>rules</span>
        </div>
        {(detail?.rules ?? []).map((v) => (
          <div key={`rule-${v}`}>- {v}</div>
        ))}
      </section>
      <section
        data-testid="detail-pane-constraints"
        onClick={() => setSelectedPane("constraints")}
        className={`relative p-4 text-sm ${selectedPane === "constraints" ? "bg-muted/15" : ""}`}
      >
        {selectedPane === "constraints" && (
          <button
            data-testid="pane-edit-gear-constraints"
            className="absolute right-2 top-2 rounded p-1 text-muted-foreground hover:bg-muted"
            onClick={(e) => {
              e.stopPropagation();
              openEditor();
            }}
            aria-label="edit-pane-constraints"
          >
            <Settings className="h-4 w-4" />
          </button>
        )}
        <div className="mb-2 flex items-center gap-2 text-sm font-bold uppercase tracking-wide text-foreground">
          <ShieldAlert className="h-3.5 w-3.5" />
          <span>constraints</span>
        </div>
        {(detail?.constraints ?? []).map((v) => (
          <div key={`constraint-${v}`}>- {v}</div>
        ))}
      </section>
      <section
        data-testid="detail-pane-features"
        onClick={() => setSelectedPane("features")}
        className={`relative p-4 text-sm ${selectedPane === "features" ? "bg-muted/15" : ""}`}
      >
        {selectedPane === "features" && (
          <button
            data-testid="pane-edit-gear-features"
            className="absolute right-2 top-2 rounded p-1 text-muted-foreground hover:bg-muted"
            onClick={(e) => {
              e.stopPropagation();
              openEditor();
            }}
            aria-label="edit-pane-features"
          >
            <Settings className="h-4 w-4" />
          </button>
        )}
        <div className="mb-2 flex items-center gap-2 text-sm font-bold uppercase tracking-wide text-foreground">
          <Sparkles className="h-3.5 w-3.5" />
          <span>features</span>
        </div>
        {(detail?.features ?? []).map((v) => (
          <div key={`feature-${v}`}>- {v}</div>
        ))}
      </section>
      </div>
      </section>
    </div>
  );
}

function DomainsPane({
  detail,
  selectedDomain,
  setSelectedDomain
}: {
  detail: DetailLayoutProps["detail"];
  selectedDomain: string;
  setSelectedDomain: (domain: string) => void;
}) {
  const domains = detail?.domains ?? [];
  const selected = domains.find((domain) => domain.name === selectedDomain) ?? domains[0] ?? null;

  return (
    <div>
      <div className={sectionLabelClass}>domains</div>
      <section data-testid="detail-pane-domains" className={`p-2 text-sm ${paneShellClass}`}>
      <div className="grid gap-0 md:grid-cols-[220px_1fr] md:divide-x md:divide-border">
        <div className="p-2">
          <div className="flex flex-wrap gap-2">
            {domains.length === 0 && <span className="text-xs text-muted-foreground">(none)</span>}
            {domains.map((domain) => (
              <span
                key={domain.name}
                className={`inline-flex cursor-pointer rounded-md border px-2 py-1 text-xs ${
                  selected?.name === domain.name
                    ? "border-primary text-foreground font-semibold"
                    : "border-border text-muted-foreground"
                }`}
                onClick={() => setSelectedDomain(domain.name)}
              >
                {domain.name}
              </span>
            ))}
          </div>
        </div>
        <div className="p-2">
          {!selected && <div className="text-sm text-muted-foreground">선택된 domain이 없습니다.</div>}
          {selected && (
            <div className="space-y-3">
              <div>
                <div className="text-sm text-foreground">{selected.description || "(empty)"}</div>
              </div>
              <div>
                <div className="flex flex-wrap gap-2">
                  {selected.features.length === 0 && (
                    <span className="text-sm text-muted-foreground">(empty)</span>
                  )}
                  {selected.features.map((feature) => (
                    <span key={feature} className="px-2 py-1 text-xs text-foreground">
                      {feature}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
      </section>
    </div>
  );
}

function MemoPane({
  memoDraft,
  updateMemo,
  flushMemo,
  memoSaving
}: Pick<DetailLayoutProps, "memoDraft" | "updateMemo" | "flushMemo" | "memoSaving">) {
  return (
    <div>
      <div className={sectionLabelClass}>memo</div>
      <section className={`p-3 text-sm ${paneShellClass}`}>
        <Textarea
          value={memoDraft}
          onChange={(e) => updateMemo(e.target.value)}
          onBlur={flushMemo}
          rows={9}
          className="min-h-[190px] resize-y bg-white"
          placeholder="memo"
        />
        <div className="mt-2 flex justify-end text-xs text-muted-foreground">
          {memoSaving ? "saving..." : "saved"}
        </div>
      </section>
    </div>
  );
}
