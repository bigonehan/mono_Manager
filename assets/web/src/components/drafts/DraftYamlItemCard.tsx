import { Card, CardContent } from "@/components/ui/card";
import { draftItemStatusDotClass } from "@/lib/draft-item-state";

export type DraftYamlItem = {
  name: string;
  status: "work" | "wait" | "complete";
};

type Props = {
  item: DraftYamlItem;
  onClick: () => void;
  selected?: boolean;
};

export function DraftYamlItemCard({ item, onClick, selected = false }: Props) {
  return (
    <button type="button" onClick={onClick} className="w-full text-left" data-testid={`draft-item-card-${item.name}`}>
      <Card className={`rounded-xl border bg-white hover:bg-muted/30 ${selected ? "border-primary" : "border-border"}`}>
        <CardContent className="flex items-center justify-between px-3 py-2">
          <div className="truncate text-sm font-semibold text-foreground">{item.name}</div>
          <span
            className={`inline-block h-2.5 w-2.5 rounded-full ${draftItemStatusDotClass(item.status)}`}
            data-testid={`draft-item-status-dot-${item.name}`}
          />
        </CardContent>
      </Card>
    </button>
  );
}
