const REGEX_META = /[\\^$.*+?()[\]{}|/]/g;

export function escapeForRegexLiteral(value) {
  return String(value).replace(REGEX_META, "\\$&");
}

export function exactRegex(value, flags = "") {
  return new RegExp(`^${escapeForRegexLiteral(value)}$`, flags);
}

export async function collectDataTestIds(page) {
  return page.evaluate(() =>
    Array.from(document.querySelectorAll("[data-testid]"))
      .map((element) => element.getAttribute("data-testid"))
      .filter((value) => typeof value === "string" && value.length > 0),
  );
}

export async function assertDataTestId(page, testId) {
  const testIds = await collectDataTestIds(page);
  if (!testIds.includes(testId)) {
    throw new Error(
      `Missing data-testid "${testId}". Available ids: ${testIds.join(", ") || "(none)"}`,
    );
  }
  return testId;
}
