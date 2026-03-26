import { Pencil } from "lucide-react";
import type { DetailPane } from "@/store/orc-store";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

const paneLabelClass = "mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";

type Props = {
  rules: string[];
  constraints: string[];
  features: string[];
  selectedPane: DetailPane;
  setSelectedPane: (pane: DetailPane) => void;
  openEditor: () => void;
  actionsDisabled: boolean;
  sectionLabel?: string;
  editRules: string;
  editConstraints: string;
  editFeatures: string;
  setEditRules: (value: string) => void;
  setEditConstraints: (value: string) => void;
  setEditFeatures: (value: string) => void;
  saveListPane: (pane: "rules" | "constraints" | "features") => Promise<void>;
  listSaving: boolean;
  projectInfoSaving?: boolean;
}

export function DetailTabsPane({
  rules,
  constraints,
  features,
  selectedPane,
  setSelectedPane,
  openEditor,
  actionsDisabled,
  sectionLabel,
  editRules,
  editConstraints,
  editFeatures,
  setEditRules,
  setEditConstraints,
  setEditFeatures,
  saveListPane,
  listSaving
}: Props) {
  const activePane: "rules" | "constraints" | "features" =
    selectedPane === "rules" || selectedPane === "constraints" || selectedPane === "features"
      ? selectedPane
      : "rules";
  const isSaving = actionsDisabled || listSaving;
  const activeListValue = activePane === "rules" ? editRules : activePane === "constraints" ? editConstraints : editFeatures;
  const onChangeActiveList = (next: string) => {
    if (activePane === "rules") setEditRules(next);
    if (activePane === "constraints") setEditConstraints(next);
    if (activePane === "features") setEditFeatures(next);
  };
  return (
    <div className="mt-4">
      {sectionLabel && <div className={paneLabelClass}>{sectionLabel}</div>}
      <div className="relative">
        <div className="mb-2 flex flex-wrap items-end justify-end gap-2 px-2">
          {["rules", "constraints", "features"].map((tab) => (
            <button
              key={`detail-tab-${tab}`}
              type="button"
              onClick={() => setSelectedPane(tab as "rules" | "constraints" | "features")}
              disabled={actionsDisabled}
              className={`rounded-t-md border border-b-0 px-3 py-1 text-xs font-semibold uppercase tracking-wide ${
                activePane === tab
                  ? "border-border bg-white text-foreground"
                  : "border-border/70 bg-muted/20 text-muted-foreground"
              }`}
            >
              {tab}
            </button>
          ))}
        </div>
        <section data-testid="detail-pane-lists" className="relative overflow-hidden rounded-2xl border border-border bg-white text-sm">
        <div
          data-testid={`detail-pane-${activePane}`}
          className="h-[320px] min-h-[320px] overflow-hidden bg-white p-3 pb-14"
          onClick={() => setSelectedPane(activePane)}
        >
          <Textarea
            value={activeListValue}
            onChange={(e) => onChangeActiveList(e.target.value)}
            rows={12}
            className="h-full resize-none border-0 bg-transparent p-0 text-sm focus-visible:ring-0"
            aria-label={`${activePane}-editor`}
            disabled={isSaving}
            placeholder={
              activePane === "rules"
                ? "rules"
                : activePane === "constraints"
                  ? "constraints"
                  : "features"
            }
          />
        </div>
        <div className="absolute right-3 bottom-3 flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => void saveListPane(activePane)}
            disabled={actionsDisabled || listSaving}
          >
            {listSaving ? "saving..." : "save"}
          </Button>
          <button
            data-testid={`pane-edit-pencil-${activePane}`}
            type="button"
            className="inline-flex h-9 w-9 items-center justify-center rounded-full bg-emerald-600 text-white shadow-sm hover:bg-emerald-700 disabled:opacity-60"
            onClick={openEditor}
            disabled={actionsDisabled}
            aria-label={`edit-pane-${activePane}`}
          >
            <Pencil className="h-4 w-4" />
          </button>
        </div>
        </section>
      </div>
    </div>
  );
}
