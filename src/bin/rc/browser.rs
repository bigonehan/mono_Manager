#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadMode {
    On,
    Off,
}

impl HeadMode {
    pub fn as_agent_browser_flag(self) -> &'static str {
        match self {
            Self::On => " --headed",
            Self::Off => "",
        }
    }
}

pub fn install_command(agent_browser_command: &str) -> String {
    format!(
        "if command -v {agent_browser_command} >/dev/null 2>&1; then {agent_browser_command} install; \
         else npm install -g agent-browser && {agent_browser_command} install; fi"
    )
}

pub fn open_command(agent_browser_command: &str, url: &str, head_mode: HeadMode) -> String {
    format!(
        "{agent_browser_command} open {url}{}",
        head_mode.as_agent_browser_flag()
    )
}

pub fn wait_for_url_command(url: &str) -> String {
    format!(
        concat!(
            "python3 - \"{url}\" <<'PY'\n",
            "import sys\n",
            "import time\n",
            "import urllib.request\n",
            "\n",
            "url = sys.argv[1]\n",
            "last = None\n",
            "for _ in range(30):\n",
            "    try:\n",
            "        with urllib.request.urlopen(url, timeout=2) as response:\n",
            "            if response.status < 500:\n",
            "                raise SystemExit(0)\n",
            "    except Exception as exc:\n",
            "        last = exc\n",
            "        time.sleep(1)\n",
            "\n",
            "print(\"server not ready: \" + url + \": \" + str(last), file=sys.stderr)\n",
            "raise SystemExit(1)\n",
            "PY"
        ),
        url = url
    )
}

pub fn click_command(agent_browser_command: &str, label: &str) -> String {
    format!("{agent_browser_command} find role button click --name \"{label}\"")
}

pub fn click_selector_command(agent_browser_command: &str, selector: &str) -> String {
    format!("{agent_browser_command} click \"{selector}\"")
}

pub fn fill_command(agent_browser_command: &str, selector: &str, value: &str) -> String {
    format!("{agent_browser_command} fill \"{selector}\" \"{value}\"")
}

pub fn keyboard_type_command(agent_browser_command: &str, value: &str) -> String {
    format!("{agent_browser_command} keyboard type \"{value}\"")
}

pub fn wait_for_selector_command(agent_browser_command: &str, selector: &str) -> String {
    format!("{agent_browser_command} wait \"{selector}\"")
}

pub fn sleep_command(seconds: u32) -> String {
    format!("sleep {seconds}")
}

pub fn snapshot_command(agent_browser_command: &str) -> String {
    format!("{agent_browser_command} snapshot -i")
}

pub fn screenshot_and_close_command(agent_browser_command: &str) -> String {
    format!(
        "{agent_browser_command} screenshot rc-web.png && printf 'Screenshot saved: rc-web.png\\n' && {agent_browser_command} close"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headed_flag_is_added() {
        assert!(
            open_command("agent-browser", "http://127.0.0.1:3000", HeadMode::On)
                .contains("--headed")
        );
        assert!(
            !open_command("agent-browser", "http://127.0.0.1:3000", HeadMode::Off)
                .contains("--headed")
        );
        assert!(
            !open_command("agent-browser", "http://127.0.0.1:3000", HeadMode::On)
                .contains("snapshot -i")
        );
    }

    #[test]
    fn install_command_skips_system_dependency_flag() {
        let command = install_command("agent-browser");
        assert!(!command.contains("--with-deps"));
        assert!(command.contains("agent-browser install"));
    }

    #[test]
    fn helper_commands_use_supported_locators() {
        assert_eq!(
            click_selector_command(
                "agent-browser",
                ".auth-card:nth-of-type(2) button[type='submit']"
            ),
            "agent-browser click \".auth-card:nth-of-type(2) button[type='submit']\""
        );
        assert_eq!(
            fill_command(
                "agent-browser",
                ".auth-card:nth-of-type(1) input[type='password']",
                "demo-pass"
            ),
            "agent-browser fill \".auth-card:nth-of-type(1) input[type='password']\" \"demo-pass\""
        );
        assert_eq!(
            wait_for_selector_command(
                "agent-browser",
                ".auth-card:nth-of-type(2) input[placeholder='name']"
            ),
            "agent-browser wait \".auth-card:nth-of-type(2) input[placeholder='name']\""
        );
        assert_eq!(sleep_command(1), "sleep 1");
        assert_eq!(
            snapshot_command("agent-browser"),
            "agent-browser snapshot -i"
        );
        assert!(wait_for_url_command("http://127.0.0.1:3000")
            .contains("python3 - \"http://127.0.0.1:3000\" <<'PY'"));
    }
}
