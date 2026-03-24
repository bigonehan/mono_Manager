import test from "node:test";
import assert from "node:assert/strict";
import { parseRequirementBlocks } from "../../src/lib/requirement-parser.ts";

test("parser: ## only", () => {
  const parsed = parseRequirementBlocks("## feature_a");
  assert.equal(parsed.length, 1);
  assert.deepEqual(parsed[0], { title: "feature_a", rules: [], steps: [] });
});

test("parser: ## + -", () => {
  const parsed = parseRequirementBlocks(["## feature_a", "- rule_a"].join("\n"));
  assert.equal(parsed.length, 1);
  assert.deepEqual(parsed[0], { title: "feature_a", rules: ["rule_a"], steps: [] });
});

test("parser: ## + - + >", () => {
  const parsed = parseRequirementBlocks(["## feature_a", "- rule_a", "> step_a"].join("\n"));
  assert.equal(parsed.length, 1);
  assert.deepEqual(parsed[0], { title: "feature_a", rules: ["rule_a"], steps: ["step_a"] });
});

test("parser: ## contains - and > in one header keeps full title", () => {
  const parsed = parseRequirementBlocks("## feature_a - rule_a > step_a");
  assert.equal(parsed.length, 1);
  assert.deepEqual(parsed[0], {
    title: "feature_a - rule_a > step_a",
    rules: [],
    steps: []
  });
});

test("parser: mixed multi-block input", () => {
  const parsed = parseRequirementBlocks(
    [
      "## a",
      "- ar1",
      "> as1",
      "## b",
      "- br1",
      "- br2",
      "> bs1",
      "2. bs2"
    ].join("\n")
  );
  assert.equal(parsed.length, 2);
  assert.deepEqual(parsed[0], { title: "a", rules: ["ar1"], steps: ["as1"] });
  assert.deepEqual(parsed[1], { title: "b", rules: ["br1", "br2"], steps: ["bs1", "bs2"] });
});
