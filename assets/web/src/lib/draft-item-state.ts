export type DraftItemStatus = "wait" | "work" | "complete";

export function resolveDraftItemStatus(
  serverStatus: DraftItemStatus,
  options: { isRunningNow: boolean }
): DraftItemStatus {
  if (options.isRunningNow) return "work";
  return serverStatus;
}

export function draftItemStatusDotClass(status: DraftItemStatus): string {
  if (status === "work") return "bg-amber-500";
  if (status === "complete") return "bg-emerald-500";
  return "bg-red-500";
}
