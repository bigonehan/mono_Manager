import unittest

from scripts.run_playwright_qa import build_command


class RunPlaywrightQaWrapperTest(unittest.TestCase):
    def test_builds_rc_command(self) -> None:
        self.assertEqual(
            build_command(["--web-root", "assets/web", "--", "node", "qa-check.mjs"]),
            [
                "cargo",
                "run",
                "--quiet",
                "--bin",
                "rc",
                "--",
                "run-playwright-qa",
                "--web-root",
                "assets/web",
                "--",
                "node",
                "qa-check.mjs",
            ],
        )


if __name__ == "__main__":
    unittest.main()
