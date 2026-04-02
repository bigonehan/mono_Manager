import test from "node:test";
import assert from "node:assert/strict";

import {
  assertDataTestId,
  collectDataTestIds,
  escapeForRegexLiteral,
  exactRegex,
} from "./playwright_safe_helpers.mjs";

test("escapeForRegexLiteral escapes regex metacharacters", () => {
  const escaped = escapeForRegexLiteral(String.raw`a+b(c)[d]{e}|f/g?h^i$j\k.l*`);
  assert.equal(
    escaped,
    String.raw`a\+b\(c\)\[d\]\{e\}\|f\/g\?h\^i\$j\\k\.l\*`,
  );
});

test("exactRegex matches only the exact string", () => {
  const regex = exactRegex("price ($10.00)?");
  assert.equal(regex.test("price ($10.00)?"), true);
  assert.equal(regex.test("price ($10.00)"), false);
});

test("collectDataTestIds returns only populated ids", async () => {
  const page = {
    evaluate: async (callback) =>
      callback.call(null, [
        { getAttribute: () => "header-title" },
        { getAttribute: () => "" },
        { getAttribute: () => null },
        { getAttribute: () => "save-button" },
      ]),
  };
  globalThis.document = {
    querySelectorAll() {
      return [
        { getAttribute: () => "header-title" },
        { getAttribute: () => "" },
        { getAttribute: () => null },
        { getAttribute: () => "save-button" },
      ];
    },
  };
  try {
    const testIds = await collectDataTestIds(page);
    assert.deepEqual(testIds, ["header-title", "save-button"]);
  } finally {
    delete globalThis.document;
  }
});

test("assertDataTestId throws with available ids", async () => {
  const page = {
    evaluate: async () => ["header-title", "save-button"],
  };
  await assert.rejects(
    () => assertDataTestId(page, "missing-id"),
    /Missing data-testid "missing-id"\. Available ids: header-title, save-button/,
  );
});

test("assertDataTestId returns the id when present", async () => {
  const page = {
    evaluate: async () => ["header-title", "save-button"],
  };
  await assert.doesNotReject(() => assertDataTestId(page, "save-button"));
});
