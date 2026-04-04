#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerificationMode {
    UnitOnly,
    FixtureOnly,
    RealEquivalent,
    RealRuntime,
}

impl VerificationMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::UnitOnly => "unit_only",
            Self::FixtureOnly => "fixture_only",
            Self::RealEquivalent => "real_equivalent",
            Self::RealRuntime => "real_runtime",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidenceRecord {
    pub(crate) checked: bool,
    pub(crate) detail: String,
    pub(crate) data_source: Option<String>,
    pub(crate) execution: Option<String>,
    pub(crate) artifact: Option<String>,
    pub(crate) raw: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HardGateInput {
    pub(crate) verify_targets: Vec<String>,
    pub(crate) problems: Vec<String>,
    pub(crate) check_section_names: Vec<String>,
    pub(crate) check_evidence_lines: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GateEvaluation {
    pub(crate) mode: Option<VerificationMode>,
    pub(crate) failures: Vec<String>,
}

impl GateEvaluation {
    pub(crate) fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

pub(crate) fn parse_evidence_records(lines: &[String]) -> Result<Vec<EvidenceRecord>, String> {
    let mut records = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        let (checked, rest) = if let Some(rest) = trimmed.strip_prefix("- [x]") {
            (true, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("- [ ]") {
            (false, rest.trim())
        } else {
            continue;
        };
        let mut detail = rest.to_string();
        let mut data_source = None;
        let mut execution = None;
        let mut artifact = None;
        let parts = rest.split('|').map(str::trim).collect::<Vec<_>>();
        if let Some(first) = parts.first() {
            detail = (*first).to_string();
        }
        for part in parts.iter().skip(1) {
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim().to_ascii_lowercase();
                let value = value.trim().to_string();
                match key.as_str() {
                    "data_source" => data_source = Some(value),
                    "execution" => execution = Some(value),
                    "artifact" => artifact = Some(value),
                    _ => {}
                }
            }
        }
        records.push(EvidenceRecord {
            checked,
            detail,
            data_source,
            execution,
            artifact,
            raw: trimmed.to_string(),
        });
    }
    if records.is_empty() {
        return Err("missing job.md check evidence entries".to_string());
    }
    Ok(records)
}

pub(crate) fn evaluate_hard_gates(input: &HardGateInput) -> Result<GateEvaluation, String> {
    let records = parse_evidence_records(&input.check_evidence_lines)?;
    let mut evaluation = GateEvaluation {
        mode: Some(classify_verification_mode(&records)),
        failures: Vec::new(),
    };

    let has_ui = input
        .check_section_names
        .iter()
        .any(|name| name.eq_ignore_ascii_case("ui_checklist"));
    let has_reentry = input
        .check_section_names
        .iter()
        .any(|name| name.eq_ignore_ascii_case("reentry_checklist"));
    let has_persistence = input
        .check_section_names
        .iter()
        .any(|name| name.eq_ignore_ascii_case("persistence_checklist"));
    let requires_reentry = has_reentry
        || has_persistence
        || input.verify_targets.iter().any(|item| contains_reentry_signal(item))
        || input.problems.iter().any(|item| contains_reentry_signal(item));

    if has_ui {
        let has_real_ui_evidence = records.iter().any(|record| {
            record.checked
                && record.execution.as_deref() == Some("browser")
                && matches!(
                    record.data_source.as_deref(),
                    Some("real") | Some("real-equivalent")
                )
                && has_nonempty_artifact(record.artifact.as_deref())
                && !record_uses_fixture(record)
        });
        if !has_real_ui_evidence {
            evaluation.failures.push(
                "ui_checklist requires browser evidence with data_source=real|real-equivalent and artifact=<path>; fixture/bootstrap evidence does not count".to_string(),
            );
        }
    }

    if requires_reentry {
        let has_reentry_evidence = records.iter().any(|record| {
            record.checked
                && matches!(
                    record.data_source.as_deref(),
                    Some("real") | Some("real-equivalent")
                )
                && contains_reentry_signal(&record.raw)
        });
        if !has_reentry_evidence {
            evaluation.failures.push(
                "re-entry/persistence verification requires checked evidence that mentions persist/read/load/reload/reopen with data_source=real|real-equivalent".to_string(),
            );
        }
    }

    Ok(evaluation)
}

pub(crate) fn classify_verification_mode(records: &[EvidenceRecord]) -> VerificationMode {
    let any_real_runtime = records.iter().any(|record| {
        record.checked
            && record.execution.as_deref() == Some("browser")
            && record.data_source.as_deref() == Some("real")
            && has_nonempty_artifact(record.artifact.as_deref())
            && !record_uses_fixture(record)
    });
    if any_real_runtime {
        return VerificationMode::RealRuntime;
    }

    let any_real_equivalent = records.iter().any(|record| {
        record.checked
            && matches!(
                record.data_source.as_deref(),
                Some("real") | Some("real-equivalent")
            )
            && contains_reentry_signal(&record.raw)
    });
    if any_real_equivalent {
        return VerificationMode::RealEquivalent;
    }

    let any_fixture = records
        .iter()
        .any(|record| record.checked && record_uses_fixture(record));
    if any_fixture {
        return VerificationMode::FixtureOnly;
    }

    VerificationMode::UnitOnly
}

fn has_nonempty_artifact(value: Option<&str>) -> bool {
    value.is_some_and(|item| {
        let trimmed = item.trim();
        !trimmed.is_empty() && trimmed != "none" && trimmed != "missing"
    })
}

fn contains_reentry_signal(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    [
        "reload",
        "reopen",
        "restart",
        "re-entry",
        "reentry",
        "persist",
        "read/load",
        "load",
        "read",
        "재실행",
        "다시 열",
        "사라짐",
        "유지",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn record_uses_fixture(record: &EvidenceRecord) -> bool {
    record
        .data_source
        .as_deref()
        .is_some_and(|value| matches!(value, "fixture" | "mock"))
        || {
            let lowered = record.raw.to_ascii_lowercase();
            [
                "fixture",
                "mock",
                "bootstrap",
                "__preset_e2e_state__",
                "in-memory",
                "preset.bootstrap",
            ]
            .iter()
            .any(|needle| lowered.contains(needle))
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_evidence_key_value_fields() {
        let records = parse_evidence_records(&[String::from(
            "- [x] ui -> rendered : ok | data_source=real | execution=browser | artifact=.project/screenshot/ui.png",
        )])
        .expect("parse");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].data_source.as_deref(), Some("real"));
        assert_eq!(records[0].execution.as_deref(), Some("browser"));
        assert_eq!(records[0].artifact.as_deref(), Some(".project/screenshot/ui.png"));
    }

    #[test]
    fn ui_gate_rejects_fixture_browser_evidence() {
        let result = evaluate_hard_gates(&HardGateInput {
            verify_targets: vec!["ui".to_string()],
            problems: Vec::new(),
            check_section_names: vec!["ui_checklist".to_string()],
            check_evidence_lines: vec![String::from(
                "- [x] ui -> rendered : ok | data_source=fixture | execution=browser | artifact=.project/screenshot/ui.png",
            )],
        })
        .expect("gate");
        assert!(!result.passed());
        assert_eq!(result.mode, Some(VerificationMode::FixtureOnly));
    }

    #[test]
    fn reentry_gate_requires_real_persistence_evidence() {
        let result = evaluate_hard_gates(&HardGateInput {
            verify_targets: vec!["prefix group 사라짐 방지".to_string()],
            problems: Vec::new(),
            check_section_names: vec!["reentry_checklist".to_string()],
            check_evidence_lines: vec![String::from(
                "- [x] prefix group -> kept : unit ok | data_source=fixture | execution=unit | artifact=none",
            )],
        })
        .expect("gate");
        assert!(!result.passed());
    }
}
