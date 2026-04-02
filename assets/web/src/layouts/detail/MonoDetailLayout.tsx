import { Boxes, Pencil, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";
import { DetailTabsPane } from "@/layouts/detail/DetailTabsPane";

const domainHeaderClass = "mt-8 mb-3 flex items-center justify-between gap-4";
const domainTitleClass = "text-[2.1rem] font-black uppercase tracking-[0.08em] text-slate-700";
const domainActionBoxClass = "inline-flex items-center gap-1 border border-border/70 bg-white px-3 py-2 shadow-sm";
const domainPanelClass =
  "overflow-hidden rounded-[2rem] border border-stone-200/80 bg-white/95 p-4 shadow-[0_18px_40px_rgba(15,23,42,0.06)]";
const domainRailClass = "flex min-h-[128px] flex-wrap content-start gap-4 rounded-[1.6rem] border border-stone-200/80 bg-stone-50/60 p-4";
const domainChipClass =
  "inline-flex items-center rounded-[1.1rem] border px-5 py-3 text-[1.05rem] font-medium transition";
const domainFeatureCardClass = "rounded-2xl border border-border/70 bg-stone-50/70 px-3 py-2 text-xs";

export function MonoDetailLayout({
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
  const domains = detail?.domains ?? [];
  const selected = domains.find((domain) => domain.name === selectedDomain) ?? domains[0] ?? null;
  const onDomainClick = (domainName: string) => {
    setSelectedDomain(domainName);
    void refreshDomainFeatures(domainName);
  };
  return (
    <div className="space-y-4">
      {showProjectInfo && (
      <section
        data-testid="detail-pane-project"
        onClick={() => setSelectedPane("project_info")}
        className="relative border-b border-border pb-12 pt-5 text-sm"
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
        <div className="flex items-start justify-between gap-3">
          <div>
            <Input
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              className="mb-3 text-4xl font-extrabold tracking-tight"
              disabled={actionsDisabled || projectInfoSaving}
            />
            <Input
              value={editDescription}
              onChange={(e) => setEditDescription(e.target.value)}
              className="mt-1"
              disabled={actionsDisabled || projectInfoSaving}
            />
            <Input
              value={editSpec}
              onChange={(e) => setEditSpec(e.target.value)}
              className="mt-1"
              disabled={actionsDisabled || projectInfoSaving}
            />
            <Input
              value={editGoal}
              onChange={(e) => setEditGoal(e.target.value)}
              className="mt-1"
              disabled={actionsDisabled || projectInfoSaving}
            />
            <Input
              value={editArchitecture}
              onChange={(e) => setEditArchitecture(e.target.value)}
              className="mt-1"
              placeholder="architecture skill id"
              disabled={actionsDisabled || projectInfoSaving}
            />
            <Button
              type="button"
              size="sm"
              className="mt-3"
              onClick={() => void saveProjectInfo()}
              disabled={actionsDisabled || projectInfoSaving}
            >
              {projectInfoSaving ? "저장중..." : "저장"}
            </Button>
          </div>
          <span className="rounded-lg bg-muted px-3 py-1 text-xs font-bold uppercase">{detail?.state ?? "wait"}</span>
        </div>
        <div className="mt-6 flex flex-wrap gap-2">
          <span className="inline-flex rounded-xl border border-border bg-muted p-2 text-foreground">
            <Boxes className="h-5 w-5" />
          </span>
          {(parseSpecTokens(editSpec).length > 0
            ? parseSpecTokens(editSpec)
            : ["(empty)"]
          ).map((token) => (
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
                  data-testid={`mono-domain-chip-${domain.name}`}
                  className={`${domainChipClass} ${
                    selected?.name === domain.name
                      ? "border-emerald-500 bg-emerald-50 text-slate-900 shadow-sm"
                      : domain.is_active
                        ? "border-amber-400 bg-amber-50 text-amber-900"
                        : "border-stone-300 bg-white text-slate-600 hover:border-stone-400 hover:text-slate-800"
                  }`}
                  onClick={() => onDomainClick(domain.name)}
                >
                  <span>{domain.name}</span>
                  {domain.is_active && (
                    <span
                      data-testid={`mono-domain-active-${domain.name}`}
                      className="ml-2 rounded-full bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-amber-800"
                    >
                      active
                    </span>
                  )}
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
    </div>
  );
}
