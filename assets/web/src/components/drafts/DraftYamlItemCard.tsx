import { Card, CardContent } from "@/components/ui/card";

export type DraftYamlItem = {
  name: string;
  status: "work" | "wait" | "complete";
};

function statusDotClass(status: DraftYamlItem["status"]): string {
  if (status === "work") return "bg-amber-500";
  if (status === "complete") return "bg-emerald-500";
  return "bg-red-500";
}

type Props = {
  item: DraftYamlItem;
  onClick: () => void;
};

export function DraftYamlItemCard({ item, onClick }: Props) {
  return (
    <button type="button" onClick={onClick} className="w-full text-left">
      <Card className="rounded-xl border border-border bg-white hover:bg-muted/30">
        <CardContent className="flex items-center justify-between px-3 py-2">
          <div className="truncate text-sm font-semibold text-foreground">{item.name}</div>
          <span className={`inline-block h-2.5 w-2.5 rounded-full ${statusDotClass(item.status)}`} />
        </CardContent>
      </Card>
    </button>
  );
}
