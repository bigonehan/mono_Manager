import { Folder, Pencil, RefreshCw } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";
import { DetailTabsPane } from "@/layouts/detail/DetailTabsPane";

const sectionLabelClass =
  "mt-8 mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";
const paneShellClass = "rounded-2xl border border-border/70 bg-white";

export function CodeDetailLayout({
  detail,
  showProjectInfo = true,
  selectedPane,
  setSelectedPane,
  selectedDomain,
  setSelectedDomain,
  refreshDomainFeatures,
  domainLoading = false,
  domainError = "",
  openEditor,
  actionsDisabled,
  memoDraft,
  updateMemo,
  flushMemo,
  memoSaving
}: DetailLayoutProps) {
  return (
    <>
      {showProjectInfo && (
      <div>
        <div
          data-testid="detail-pane-project"
          onClick={() => setSelectedPane("project_info")}
          className="relative border-b border-border px-2 pb-12 pt-1 text-sm"
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
      )}
      <MemoPane
        memoDraft={memoDraft}
        updateMemo={updateMemo}
        flushMemo={flushMemo}
        memoSaving={memoSaving}
        actionsDisabled={actionsDisabled}
      />
      <DetailTabsPane
        rules={detail?.rules ?? []}
        constraints={detail?.constraints ?? []}
        features={detail?.features ?? []}
        selectedPane={selectedPane}
        setSelectedPane={setSelectedPane}
        openEditor={openEditor}
        actionsDisabled={actionsDisabled}
      />
      <DomainsPane
        detail={detail}
        selectedDomain={selectedDomain}
        setSelectedDomain={setSelectedDomain}
        refreshDomainFeatures={refreshDomainFeatures}
        domainLoading={domainLoading}
        domainError={domainError}
      />
    </>
  );
}

function DomainsPane({
  detail,
  selectedDomain,
  setSelectedDomain,
  refreshDomainFeatures,
  domainLoading,
  domainError
}: {
  detail: DetailLayoutProps["detail"];
  selectedDomain: string;
  setSelectedDomain: (domain: string) => void;
  refreshDomainFeatures: (domain?: string) => Promise<boolean>;
  domainLoading: boolean;
  domainError: string;
}) {
  const domains = detail?.domains ?? [];
  const selected = domains.find((domain) => domain.name === selectedDomain) ?? domains[0] ?? null;
  const onDomainClick = (domainName: string) => {
    setSelectedDomain(domainName);
    void refreshDomainFeatures(domainName);
  };

  return (
    <div>
      <div className={sectionLabelClass}>domains</div>
      <section data-testid="detail-pane-domains" className={`p-2 text-sm ${paneShellClass}`}>
      <div className="grid gap-0 md:grid-cols-[220px_1fr] md:divide-x md:divide-border">
        <div className="p-2">
          <div className="mb-2 flex items-center justify-end">
            <button
              type="button"
              className="rounded p-1 text-muted-foreground hover:bg-muted"
              onClick={() => void refreshDomainFeatures(selected?.name)}
              aria-label="domains-refresh"
              data-testid="domains-refresh"
              disabled={domainLoading}
            >
              <RefreshCw className={`h-4 w-4 ${domainLoading ? "animate-spin" : ""}`} />
            </button>
          </div>
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
                onClick={() => onDomainClick(domain.name)}
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
                <div className="space-y-2">
                  {domainLoading && (
                    <span className="text-sm text-muted-foreground">기능 분석 중...</span>
                  )}
                  {!domainLoading && domainError && (
                    <div className="rounded-md border border-red-300 bg-red-50 px-2 py-1 text-xs text-red-700">
                      {domainError}
                    </div>
                  )}
                  {!domainLoading && !domainError && selected.features.length === 0 && (
                    <span className="text-sm text-muted-foreground">소스에서 추출된 기능이 없습니다.</span>
                  )}
                  {!domainLoading &&
                    !domainError &&
                    selected.features.map((feature) => {
                      const [name, ...rest] = feature.split(":");
                      const title = name.trim();
                      const description = rest.join(":").trim() || "소스에서 추출된 기능";
                      return (
                        <div key={`${selected.name}-${title}-${description}`} className="rounded-md border border-border/70 px-2 py-1 text-xs">
                          <span className="font-semibold text-foreground">{title || feature.trim()}</span>
                          <span className="text-muted-foreground">: {description}</span>
                        </div>
                      );
                    })}
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
  memoSaving,
  actionsDisabled
}: Pick<DetailLayoutProps, "memoDraft" | "updateMemo" | "flushMemo" | "memoSaving" | "actionsDisabled">) {
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
          disabled={actionsDisabled}
        />
        <div className="mt-2 flex justify-end text-xs text-muted-foreground">
          {memoSaving ? "saving..." : "saved"}
        </div>
      </section>
    </div>
  );
}
