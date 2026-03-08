import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import YAML from "yaml";
import { CodeDraftItem } from "@/components/drafts/code_draft_item";

export type DraftYamlDetailItem = {
  name: string;
  draft: Record<string, unknown>;
};

type Props = {
  item: DraftYamlDetailItem | null;
  onClose: () => void;
};

export function DraftYamlItemModal({ item, onClose }: Props) {
  if (!item) return null;
  const yamlText = YAML.stringify(item.draft ?? {});
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4">
      <Card className="w-full max-w-3xl rounded-2xl">
        <CardHeader>
          <CardTitle>{item.name}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <CodeDraftItem yamlText={yamlText} />
          <div className="flex justify-end">
            <Button onClick={onClose}>확인</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
