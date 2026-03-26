import { Boxes, Pencil, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { DetailLayoutProps } from "@/layouts/detail/types";
import { parseSpecTokens } from "@/layouts/detail/types";
import { DetailTabsPane } from "@/layouts/detail/DetailTabsPane";

const sectionLabelClass = "mt-8 mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";

export function MonoDetailLayout({
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
  editName,
  editDescription,
  editSpec,
  editGoal,
  setEditName,
  setEditDescription,
  setEditSpec,
  setEditGoal,
  saveProjectInfo,
  projectInfoSaving,
  editRules,
  editConstraints,
  editFeatures,
  setEditRules,
  setEditConstraints,
  setEditFeatures,
  saveListPane,
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
        <div className={sectionLabelClass}>domains</div>
        <section data-testid="detail-pane-domains" className="rounded-2xl border border-border bg-white p-2 text-sm">
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
                          <div key={`${selected.name}-${title}-${description}`} className="rounded-md border border-border/70 px-2 py-1 text-xs">
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
