import { Settings } from "lucide-react";
import type { DetailPane } from "@/store/orc-store";

const paneLabelClass = "mb-3 px-2 text-base font-bold uppercase tracking-wide text-foreground/80";

type Props = {
  rules: string[];
  constraints: string[];
  features: string[];
  selectedPane: DetailPane;
  setSelectedPane: (pane: DetailPane) => void;
  openEditor: () => void;
  sectionLabel?: string;
};

type FeatureGroup = {
  group: string;
  items: Array<{ key: string; description: string }>;
};

const SUFFIX_VERBS = ["create", "read", "update", "delete", "reply", "list", "add", "remove"];

function titleCase(value: string): string {
  if (!value) return "Misc";
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function inferFeatureGroup(key: string): string {
  const normalized = key.trim().toLowerCase();
  if (!normalized) return "Misc";
  for (const suffix of SUFFIX_VERBS) {
    if (normalized.endsWith(suffix) && normalized.length > suffix.length) {
      return normalized.slice(0, normalized.length - suffix.length);
    }
  }
  return "Misc";
}

function parseFeatureGroups(features: string[]): FeatureGroup[] {
  const map = new Map<string, Array<{ key: string; description: string }>>();
  for (const raw of features) {
    const text = raw.trim();
    if (!text) continue;
    const [keyPart, ...descParts] = text.split(":");
    const key = keyPart.trim();
    const description = descParts.join(":").trim();
    const group = titleCase(inferFeatureGroup(key));
    const current = map.get(group) ?? [];
    current.push({ key, description });
    map.set(group, current);
  }
  return [...map.entries()].map(([group, items]) => ({ group, items }));
}

function ListRows({ values }: { values: string[] }) {
  if (values.length === 0) {
    return <div className="text-sm text-muted-foreground">(empty)</div>;
  }
  return (
    <div className="space-y-1 text-sm text-foreground">
      {values.map((value) => (
        <div key={value}>- {value}</div>
      ))}
    </div>
  );
}

function FeatureRows({ values }: { values: string[] }) {
  const grouped = parseFeatureGroups(values);
  if (grouped.length === 0) {
    return <div className="text-sm text-muted-foreground">(empty)</div>;
  }
  return (
    <div className="space-y-4">
      {grouped.map((group) => (
        <div key={group.group} className="space-y-1">
          <div className="text-sm font-semibold text-foreground">{group.group}</div>
          <div className="space-y-1 text-sm text-foreground">
            {group.items.map((item) => (
              <div key={`${group.group}-${item.key}`}>
                - {item.key}
                {item.description ? `: ${item.description}` : ""}
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

export function DetailTabsPane({
  rules,
  constraints,
  features,
  selectedPane,
  setSelectedPane,
  openEditor,
  sectionLabel
}: Props) {
  const activePane: "rules" | "constraints" | "features" =
    selectedPane === "rules" || selectedPane === "constraints" || selectedPane === "features"
      ? selectedPane
      : "rules";

  return (
    <div>
      {sectionLabel && <div className={paneLabelClass}>{sectionLabel}</div>}
      <div className="relative">
        <div className="flex flex-wrap items-end justify-end gap-2 lg:absolute lg:right-3 lg:top-0 lg:-translate-y-full">
          {["rules", "constraints", "features"].map((tab) => (
            <button
              key={`detail-tab-${tab}`}
              type="button"
              onClick={() => setSelectedPane(tab as "rules" | "constraints" | "features")}
              className={`rounded-t-md border border-b-0 px-3 py-1 text-xs font-semibold uppercase tracking-wide ${
                activePane === tab
                  ? "border-border bg-white text-foreground"
                  : "border-border/70 bg-muted/20 text-muted-foreground"
              }`}
            >
              {tab}
            </button>
          ))}
          <button
            data-testid={`pane-edit-gear-${activePane}`}
            className="mb-1 rounded p-1 text-muted-foreground hover:bg-muted"
            onClick={openEditor}
            aria-label={`edit-pane-${activePane}`}
          >
            <Settings className="h-4 w-4" />
          </button>
        </div>
        <section data-testid="detail-pane-lists" className="overflow-hidden rounded-2xl border border-border bg-white text-sm lg:border-x lg:border-b lg:border-t-0">
        <div
          data-testid={`detail-pane-${activePane}`}
          className="h-[320px] min-h-[320px] overflow-y-auto bg-white p-3"
          onClick={() => setSelectedPane(activePane)}
        >
          {activePane === "rules" && <ListRows values={rules} />}
          {activePane === "constraints" && <ListRows values={constraints} />}
          {activePane === "features" && <FeatureRows values={features} />}
        </div>
        </section>
      </div>
    </div>
  );
}
