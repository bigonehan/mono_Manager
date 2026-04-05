import test from "node:test";
import assert from "node:assert/strict";
import { parseJobProcessSnapshot } from "../../src/lib/job-process-parser.ts";

test("job process parser: extracts locked sections and checklist state", () => {
  const parsed = parseJobProcessSnapshot(`
# input
- inspect web ui
- keep manager session

# output
- reflect internal process

# keep
- worker split only

# add
- process alignment panel

# forbid
- manager direct impl

# task
## verify
- [ ] symptom reproduced
- [x] symptom cleared

# problems
- [ ] mismatch still unknown
- [x] worker created

# check evidence
- [ ] qa report pending
- [x] plan fixed
`);

  assert.deepEqual(parsed.locks.map((section) => section.key), ["input", "output", "keep", "add", "forbid"]);
  assert.deepEqual(parsed.locks[0].items, ["inspect web ui", "keep manager session"]);
  assert.deepEqual(parsed.locks[3].items, ["process alignment panel"]);
  assert.deepEqual(parsed.verify, [
    { checked: false, text: "symptom reproduced" },
    { checked: true, text: "symptom cleared" }
  ]);
  assert.deepEqual(parsed.problems, [
    { checked: false, text: "mismatch still unknown" },
    { checked: true, text: "worker created" }
  ]);
  assert.deepEqual(parsed.evidence, [
    { checked: false, text: "qa report pending" },
    { checked: true, text: "plan fixed" }
  ]);
});
