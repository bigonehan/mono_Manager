import YAML from "yaml";
import { useState } from "react";

type Props = {
  yamlText: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function toText(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return "";
}

function toTextList(value: unknown): string[] {
  if (!Array.isArray(value)) {
    const single = toText(value);
    return single ? [single] : [];
  }
  return value.map(toText).filter((item) => item.length > 0);
}

function PaneLabel({ title }: { title: string }) {
  return (
    <div className="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">{title}</div>
  );
}

function PaneBody({ items, numbered = false }: { items: string[]; numbered?: boolean }) {
  return (
    <section className="h-28 overflow-y-auto rounded-xl border border-border bg-white p-3">
      {items.length === 0 && <div className="text-sm text-muted-foreground">(empty)</div>}
      {items.length > 0 && (
        <div className="space-y-1 text-sm text-foreground">
          {items.map((item, index) => (
            <div key={`item-${index}`}>
              {numbered ? `${index + 1}. ${item}` : item}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function DraftItemPane({ value }: { value: Record<string, unknown> }) {
  const name = toText(value.name) || "-";
  const type = toText(value.type) || "-";
  const domains = toTextList(value.domain);
  const dependsOn = toTextList(value.depends_on);
  const scope = toTextList(value.scope);
  const steps = toTextList(value.step);
  const tasks = toTextList(value.tasks);
  const constraints = toTextList(value.constraints);
  const checks = toTextList(value.check);

  const topFields = [
    { label: "name", value: name },
    { label: "type", value: type },
    { label: "domain", value: domains.length > 0 ? domains.join(", ") : "-" }
  ];
  const tabItems = [
    { key: "step", label: "step", items: steps, numbered: true },
    { key: "tasks", label: "tasks", items: tasks, numbered: false },
    { key: "constraints", label: "constraints", items: constraints, numbered: false },
    { key: "check", label: "check", items: checks, numbered: false }
  ] as const;
  const [activeTab, setActiveTab] = useState<(typeof tabItems)[number]["key"]>("step");
  const active = tabItems.find((tab) => tab.key === activeTab) ?? tabItems[0];

  return (
    <div className="space-y-4">
      <div className="grid gap-3 md:grid-cols-3">
        {topFields.map((field) => (
          <div key={field.label}>
            <PaneLabel title={field.label} />
            <div className="rounded-lg border border-border bg-white p-2">
              <div className="text-sm font-semibold text-foreground">{field.value}</div>
            </div>
          </div>
        ))}
      </div>

      <div className="space-y-3 border-t border-border pt-3">
        <div>
          <PaneLabel title="depends_on" />
          <PaneBody items={dependsOn} />
        </div>
        <div>
          <PaneLabel title="scope" />
          <PaneBody items={scope} />
        </div>
      </div>

      <div className="space-y-3 border-t border-border pt-3">
        <div className="flex items-end justify-end gap-2">
          {tabItems.map((tab) => (
            <button
              key={`tab-${tab.key}`}
              type="button"
              onClick={() => setActiveTab(tab.key)}
              className={`rounded-t-md border border-b-0 px-3 py-1 text-xs font-semibold uppercase tracking-wide ${
                activeTab === tab.key
                  ? "border-border bg-white text-foreground"
                  : "border-border/70 bg-muted/20 text-muted-foreground"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div>
          <PaneLabel title={active.label} />
          <section className="h-56 overflow-y-auto rounded-b-xl border-x border-b border-border bg-white p-3">
            {active.items.length === 0 && <div className="text-sm text-muted-foreground">(empty)</div>}
            {active.items.length > 0 && (
              <div className="space-y-1 text-sm text-foreground">
                {active.items.map((item, index) => (
                  <div key={`active-item-${index}`}>
                    {active.numbered ? `${index + 1}. ${item}` : item}
                  </div>
                ))}
              </div>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}

function CodeDraftItemView({ parsed }: { parsed: unknown }) {
  if (Array.isArray(parsed)) {
    const first = parsed.find(isRecord);
    if (!first) {
      return <div className="text-sm text-muted-foreground">(empty)</div>;
    }
    return <DraftItemPane value={first} />;
  }
  if (isRecord(parsed)) {
    return <DraftItemPane value={parsed} />;
  }
  return <div className="text-sm text-muted-foreground">(empty)</div>;
}

export function CodeDraftItem({ yamlText }: Props) {
  let parsed: unknown;
  try {
    parsed = YAML.parse(yamlText);
  } catch (error) {
    return (
      <div className="rounded-xl border border-red-300 bg-red-50 p-3 text-sm text-red-700">
        yaml parse error: {String(error)}
      </div>
    );
  }

  return (
    <div className="h-[72vh] min-h-[560px] overflow-y-auto rounded-xl border border-border bg-white p-3">
      <CodeDraftItemView parsed={parsed} />
    </div>
  );
}
