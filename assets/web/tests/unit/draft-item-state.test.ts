import test from "node:test";
import assert from "node:assert/strict";
import { draftItemStatusDotClass, resolveDraftItemStatus } from "../../src/lib/draft-item-state.ts";

test("status resolver: running now always shows work", () => {
  assert.equal(resolveDraftItemStatus("wait", { isRunningNow: true }), "work");
  assert.equal(resolveDraftItemStatus("complete", { isRunningNow: true }), "work");
});

test("status resolver: keeps server status when not running", () => {
  assert.equal(resolveDraftItemStatus("wait", { isRunningNow: false }), "wait");
  assert.equal(resolveDraftItemStatus("work", { isRunningNow: false }), "work");
  assert.equal(resolveDraftItemStatus("complete", { isRunningNow: false }), "complete");
});

test("status dot class mapping", () => {
  assert.equal(draftItemStatusDotClass("wait"), "bg-red-500");
  assert.equal(draftItemStatusDotClass("work"), "bg-amber-500");
  assert.equal(draftItemStatusDotClass("complete"), "bg-emerald-500");
});
