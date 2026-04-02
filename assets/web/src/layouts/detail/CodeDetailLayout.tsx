import { Folder, Pencil, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";
import { DetailTabsPane } from "@/layouts/detail/DetailTabsPane";

const sectionLabelClass =
  "mt-8 mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";
const paneShellClass = "rounded-2xl border border-border/70 bg-white";
const domainHeaderClass = "mt-8 mb-3 flex items-center justify-between gap-4";
const domainTitleClass = "text-[2.1rem] font-black uppercase tracking-[0.08em] text-slate-700";
const domainActionBoxClass = "inline-flex items-center gap-1 border border-border/70 bg-white px-3 py-2 shadow-sm";
const domainPanelClass =
  "overflow-hidden rounded-[2rem] border border-stone-200/80 bg-white/95 p-4 shadow-[0_18px_40px_rgba(15,23,42,0.06)]";
const domainRailClass = "flex min-h-[128px] flex-wrap content-start gap-4 rounded-[1.6rem] border border-stone-200/80 bg-stone-50/60 p-4";
const domainChipClass =
  "inline-flex items-center rounded-[1.1rem] border px-5 py-3 text-[1.05rem] font-medium transition";
const domainFeatureCardClass = "rounded-2xl border border-border/70 bg-stone-50/70 px-3 py-2 text-xs";

export function CodeDetailLayout({
  detail,
  showProjectInfo = true,
  selectedPane,
  setSelectedPane,
  selectedDomain,
  setSelectedDomain,
  refreshDomainFeatures,
  openDomainEditor,
  domainLoading = false,
  domainError = "",
  openEditor,
  actionsDisabled,
  memoDraft,
  updateMemo,
  flushMemo,
  memoSaving,
  editName,
  editDescription,
  editSpec,
  editGoal,
  editArchitecture,
  setEditName,
  setEditDescription,
  setEditSpec,
  setEditGoal,
  setEditArchitecture,
  saveProjectInfo,
  projectInfoSaving,
  editRules,
  editConstraints,
  editFeatures,
  setEditRules,
  setEditConstraints,
  setEditFeatures,
  saveListPane,
  domainSaving = false,
  listSaving
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
            <div className="min-w-0 space-y-2">
              <Input value={editName} onChange={(e) => setEditName(e.target.value)} disabled={actionsDisabled || projectInfoSaving} />
              <Input
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
                disabled={actionsDisabled || projectInfoSaving}
              />
              <Input
                value={editSpec}
                onChange={(e) => setEditSpec(e.target.value)}
                disabled={actionsDisabled || projectInfoSaving}
              />
              <Input
                value={editGoal}
                onChange={(e) => setEditGoal(e.target.value)}
                disabled={actionsDisabled || projectInfoSaving}
              />
              <Input
                value={editArchitecture}
                onChange={(e) => setEditArchitecture(e.target.value)}
                placeholder="architecture skill id"
                disabled={actionsDisabled || projectInfoSaving}
              />
              <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                {(parseSpecTokens(editSpec ?? "").length > 0
                  ? parseSpecTokens(editSpec ?? "")
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
              <div className="flex items-center gap-3">
                <Button
                  type="button"
                  size="sm"
                  onClick={() => void saveProjectInfo()}
                  disabled={actionsDisabled || projectInfoSaving}
                >
                  {projectInfoSaving ? "저장중..." : "저장"}
                </Button>
              </div>
              <div className="flex max-w-[45%] items-center gap-2 text-sm text-muted-foreground">
                <Folder className="h-4 w-4 shrink-0" />
                <span className="truncate text-right">{detail?.path ?? ""}</span>
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
        editRules={editRules}
        editConstraints={editConstraints}
        editFeatures={editFeatures}
        setEditRules={setEditRules}
        setEditConstraints={setEditConstraints}
        setEditFeatures={setEditFeatures}
        saveListPane={saveListPane}
        listSaving={listSaving}
      />
      <DomainsPane
        detail={detail}
        selectedDomain={selectedDomain}
        setSelectedDomain={setSelectedDomain}
        refreshDomainFeatures={refreshDomainFeatures}
        openDomainEditor={openDomainEditor}
        domainLoading={domainLoading}
        domainError={domainError}
        domainSaving={domainSaving}
      />
    </>
  );
}

function DomainsPane({
  detail,
  selectedDomain,
  setSelectedDomain,
  refreshDomainFeatures,
  openDomainEditor,
  domainLoading,
  domainError,
  domainSaving
}: {
  detail: DetailLayoutProps["detail"];
  selectedDomain: string;
  setSelectedDomain: (domain: string) => void;
  refreshDomainFeatures: (domain?: string) => Promise<boolean>;
  openDomainEditor: () => void;
  domainLoading: boolean;
  domainError: string;
  domainSaving: boolean;
}) {
  const domains = detail?.domains ?? [];
  const selected = domains.find((domain) => domain.name === selectedDomain) ?? domains[0] ?? null;
  const onDomainClick = (domainName: string) => {
    setSelectedDomain(domainName);
    void refreshDomainFeatures(domainName);
  };

  return (
    <div>
      <div data-testid="detail-pane-domains-header" className={domainHeaderClass}>
        <div className={domainTitleClass}>domains</div>
        <div className={domainActionBoxClass}>
          <button
            type="button"
            className="rounded-md p-2 text-slate-600 transition hover:bg-stone-100"
            onClick={() => openDomainEditor()}
            aria-label="domains-edit"
            data-testid="domains-edit"
            disabled={!selected || domainSaving}
          >
            <Pencil className="h-5 w-5" />
          </button>
          <button
            type="button"
            className="rounded-md p-2 text-slate-600 transition hover:bg-stone-100"
            onClick={() => void refreshDomainFeatures(selected?.name)}
            aria-label="domains-refresh"
            data-testid="domains-refresh"
            disabled={domainLoading}
          >
            <RefreshCw className={`h-5 w-5 ${domainLoading ? "animate-spin" : ""}`} />
          </button>
        </div>
      </div>
      <section data-testid="detail-pane-domains" className={`text-sm ${domainPanelClass}`}>
      <div className="grid gap-4 md:grid-cols-[minmax(260px,320px)_1fr] md:items-start">
        <div className={domainRailClass}>
          {domains.length === 0 && <span className="text-xs text-muted-foreground">(none)</span>}
          {domains.map((domain) => (
            <button
              type="button"
              key={domain.name}
              className={`${domainChipClass} ${
                selected?.name === domain.name
                  ? "border-emerald-500 bg-emerald-50 text-slate-900 shadow-sm"
                  : "border-stone-300 bg-white text-slate-600 hover:border-stone-400 hover:text-slate-800"
              }`}
              onClick={() => onDomainClick(domain.name)}
            >
              {domain.name}
            </button>
          ))}
        </div>
        <div className="min-h-[128px] border-l border-stone-200/80 pl-5">
          {!selected && <div className="text-sm text-muted-foreground">선택된 domain이 없습니다.</div>}
          {selected && (
            <div className="space-y-4">
              <div className="text-sm text-foreground">{selected.description || "(empty)"}</div>
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
                      <div key={`${selected.name}-${title}-${description}`} className={domainFeatureCardClass}>
                        <span className="font-semibold text-foreground">{title || feature.trim()}</span>
                        <span className="text-muted-foreground">: {description}</span>
                      </div>
                    );
                  })}
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
