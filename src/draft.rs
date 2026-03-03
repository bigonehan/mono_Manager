use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftTask {
    pub(crate) name: String,
    #[serde(default, rename = "type")]
    pub(crate) task_type: String,
    #[serde(default)]
    pub(crate) domain: Vec<String>,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
    #[serde(default)]
    pub(crate) scope: Vec<String>,
    #[serde(default)]
    pub(crate) rule: Vec<String>,
    #[serde(default)]
    pub(crate) step: Vec<String>,
    #[serde(default)]
    pub(crate) touches: Vec<String>,
    #[serde(default)]
    pub(crate) contracts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftFeatures {
    #[serde(default)]
    pub(crate) domain: Vec<String>,
    #[serde(default)]
    pub(crate) flow: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct DraftDoc {
    #[serde(default)]
    pub(crate) rule: Vec<String>,
    #[serde(default)]
    pub(crate) features: DraftFeatures,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
    #[serde(default)]
    pub(crate) task: Vec<DraftTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct PlannedItem {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct DraftStateDoc {
    #[serde(default)]
    pub(crate) generated: Vec<String>,
    #[serde(default)]
    pub(crate) pending: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct DraftsListDoc {
    #[serde(default)]
    pub(crate) domains: Vec<String>,
    #[serde(default)]
    pub(crate) flows: Vec<String>,
    #[serde(default)]
    #[serde(alias = "feature")]
    pub(crate) features: Vec<String>,
    #[serde(default)]
    pub(crate) planned: Vec<String>,
    #[serde(default)]
    pub(crate) planned_items: Vec<PlannedItem>,
    #[serde(default)]
    pub(crate) draft_state: DraftStateDoc,
    #[serde(default)]
    pub(crate) sync_initialized: bool,
}

fn calc_failure_report_path(feature_name: &str) -> Result<PathBuf, String> {
    let feature_dir = crate::ui::action_resolve_feature_draft_path(feature_name)
        .parent()
        .ok_or_else(|| "failed to resolve feature dir".to_string())?
        .to_path_buf();
    Ok(feature_dir.join("failure.md"))
}

fn action_write_draft_failure_report(feature_name: &str, reason: &str) -> Result<(), String> {
    let path = calc_failure_report_path(feature_name)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let content = format!(
        "# draft create failure\n\n- feature: `{}`\n- reason: {}\n",
        feature_name, reason
    );
    fs::write(&path, content).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn action_clear_draft_failure_report(feature_name: &str) -> Result<(), String> {
    let path = calc_failure_report_path(feature_name)?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("failed to delete {}: {}", path.display(), e))?;
    }
    Ok(())
}

fn calc_debug_prompt_instruction() -> String {
    let debug_enabled = crate::action_load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    if !debug_enabled {
        return String::new();
    }
    "- debug 상태(on)이므로 응답 본문 맨 앞에 `DEBUG_LOG:` 한 줄을 추가해 현재 작업 단계와 대기 중이면 대기 사유를 먼저 기록해.\n- `DEBUG_LOG:`는 YAML 코드블록(```yaml ... ```) 밖에서만 작성하고, YAML 스키마/키/구조는 절대 변경하지 마.\n".to_string()
}

fn calc_draft_llm_timeout_sec() -> u64 {
    let configured = crate::action_load_app_config()
        .as_ref()
        .map_or(300, crate::config::AppConfig::default_timeout_sec);
    configured.max(30)
}

fn action_append_draft_runtime_log(debug_enabled: bool, feature_name: &str, stage: &str, detail: &str) {
    if !debug_enabled {
        return;
    }
    let runtime_dir = Path::new(".project").join("runtime");
    if fs::create_dir_all(&runtime_dir).is_err() {
        return;
    }
    let path = runtime_dir.join(format!("{}.log", feature_name));
    let mut file = match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "[{}] {} | {}", crate::calc_now_unix(), stage, detail);
}

fn action_parse_and_validate_draft_yaml(draft_yaml: &str) -> Result<DraftDoc, String> {
    let draft_doc: DraftDoc = serde_yaml::from_str(draft_yaml)
        .map_err(|e| format!("generated draft yaml invalid: {}", e))?;
    let draft_issues = crate::action_validate_draft_doc(&draft_doc);
    if !draft_issues.is_empty() {
        return Err(format!(
            "generated draft yaml invalid: {}",
            draft_issues.join(" | ")
        ));
    }
    Ok(draft_doc)
}

fn action_repair_draft_yaml_once(
    feature_name: &str,
    draft_yaml: &str,
    reason: &str,
) -> Result<String, String> {
    let prompt = format!(
        "다음 draft.yaml을 검증 실패 사유에 맞게 수정해.\n\
지시:\n\
- YAML 스키마는 반드시 유지: 최상위는 `rule`, `features`, `task`만 허용.\n\
- `task`는 리스트 형식으로 유지.\n\
- YAML 중복 키를 절대 만들지 마(특히 `rule`/`contracts` 중복 금지).\n\
- `task` 객체 키는 `name,type,domain,depends_on,scope,rule,step,touches,contracts`만 사용.\n\
- `rule`은 자동 검증 가능한 식(`==`, `!=`, `>=`, `<=`, `matches`, `contains`, `exists`)만 사용.\n\
- `contracts` 항목은 문자열 리스트로, 각 항목은 `key=value` 또는 `key: value` 형식만 사용.\n\
- `contract`(단수) 키는 사용 금지, 반드시 `contracts`(복수)만 사용.\n\
- `step`, `rule`, `contracts`의 문자열은 YAML 파싱 오류 방지를 위해 반드시 따옴표로 감싸.\n\
- `FEATURE_NAME`은 `{}`를 사용.\n\
출력 형식:\n\
FEATURE_NAME: {}\n\
```yaml\n\
<수정된 draft.yaml>\n\
```\n\
설명 문장 금지.\n\n\
검증 실패 사유:\n{}\n\n\
현재 draft:\n{}",
        feature_name, feature_name, reason, draft_yaml
    );
    let repaired_raw = crate::action_run_codex_exec_capture(&prompt)?;
    Ok(crate::calc_extract_yaml_block(&repaired_raw))
}

fn action_generate_valid_draft_yaml(
    prompt: &str,
    feature_name: &str,
    debug_enabled: bool,
) -> Result<String, String> {
    let timeout_sec = calc_draft_llm_timeout_sec();
    action_append_draft_runtime_log(
        debug_enabled,
        feature_name,
        "시작/프롬프트 전송",
        &format!("timeout={}s", timeout_sec),
    );
    let watchdog_stop = Arc::new(AtomicBool::new(false));
    let watchdog = if debug_enabled {
        let stop = Arc::clone(&watchdog_stop);
        let feature = feature_name.to_string();
        Some(thread::spawn(move || {
            let mut elapsed = 0u64;
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(15));
                elapsed += 15;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                action_append_draft_runtime_log(
                    true,
                    &feature,
                    "무응답 보호",
                    &format!("LLM 응답 대기 중 ({}s 경과)", elapsed),
                );
            }
        }))
    } else {
        None
    };
    let draft_raw_result = crate::action_run_codex_exec_capture_with_timeout(prompt, timeout_sec);
    watchdog_stop.store(true, Ordering::Relaxed);
    if let Some(handle) = watchdog {
        let _ = handle.join();
    }
    let draft_raw = draft_raw_result?;
    action_append_draft_runtime_log(
        debug_enabled,
        feature_name,
        "LLM 응답 수신",
        "초안 응답을 수신했습니다.",
    );
    let draft_yaml = crate::calc_extract_yaml_block(&draft_raw);
    action_append_draft_runtime_log(
        debug_enabled,
        feature_name,
        "검증 단계",
        "draft yaml 파싱/검증을 시작합니다.",
    );
    match action_parse_and_validate_draft_yaml(&draft_yaml) {
        Ok(_) => Ok(draft_yaml),
        Err(first_reason) => {
            let repaired_yaml = action_repair_draft_yaml_once(feature_name, &draft_yaml, &first_reason)?;
            action_append_draft_runtime_log(
                debug_enabled,
                feature_name,
                "검증 단계",
                "초기 검증 실패로 1회 자동 보정 후 재검증합니다.",
            );
            action_parse_and_validate_draft_yaml(&repaired_yaml).map_err(|repair_reason| {
                format!(
                    "{} | repair failed: {}",
                    first_reason, repair_reason
                )
            })?;
            Ok(repaired_yaml)
        }
    }
}

fn action_build_draft_prompt(
    doc: &DraftsListDoc,
    feature: &str,
    project_info: &str,
    project_rules: &[String],
    debug_instruction: &str,
) -> String {
    let feature_request = doc
        .planned_items
        .iter()
        .find(|item| item.name == feature)
        .map(|item| item.value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| feature.to_string());
    format!(
        "너는 rust-orc 프로젝트의 draft 작성기다.\nproject info:\n{}\n\nproject rules:\n- {}\n\n입력 기능 key:\n- {}\n입력 기능 설명:\n- {}\n\n지시:\n- `draft.yaml`은 템플릿(`assets/code/templates/draft.yaml`)을 대상 폴더에 먼저 복사한 뒤, 주석/예시를 지우고 값만 수정해.\n- 규칙은 `$plan-drafts-code`, `$rule-naming` 스킬을 사용해.\n- FEATURE_NAME은 반드시 입력 기능 key와 동일하게 출력해.\n- YAML 중복 키를 절대 만들지 마(특히 `rule`/`contracts`).\n- `task` 키는 `name,type,domain,depends_on,scope,rule,step,touches,contracts`만 허용.\n- `rule`은 자동 검증 가능한 식(`==`, `!=`, `>=`, `<=`, `matches`, `contains`, `exists`)으로만 작성해.\n- `contracts`는 `key=value` 또는 `key: value` 형식으로만 작성하고 `contract` 키는 금지.\n{}출력 형식:\nFEATURE_NAME: <snake_case>\n```yaml\n<draft.yaml 본문>\n```\n설명 문장 금지.",
        project_info,
        project_rules.join("\n- "),
        feature,
        feature_request,
        debug_instruction,
    )
}

fn action_write_generated_draft(
    feature_name: &str,
    draft_yaml: &str,
    debug_enabled: bool,
) -> Result<(), String> {
    let draft_path = crate::ui::action_apply_draft_create_update_delete(
        crate::ui::DraftCommand::Create,
        feature_name,
        None,
    )?;
    fs::write(&draft_path, draft_yaml)
        .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
    action_append_draft_runtime_log(
        debug_enabled,
        feature_name,
        "파일 반영 단계",
        "draft.yaml 쓰기를 완료했습니다.",
    );
    let _ = action_clear_draft_failure_report(feature_name);
    Ok(())
}

fn action_second_pass_check(
    feature_name: &str,
    draft_yaml: &str,
    known_features: &HashSet<String>,
) -> Result<(), String> {
    let doc = action_parse_and_validate_draft_yaml(draft_yaml)?;
    for dep in &doc.depends_on {
        if !known_features.contains(dep) {
            return Err(format!(
                "depends_on references unknown feature `{}` in {}",
                dep, feature_name
            ));
        }
    }
    for task in &doc.task {
        let mut seen = HashSet::new();
        for scope in &task.scope {
            let trimmed = scope.trim();
            if trimmed.is_empty() {
                return Err(format!("empty scope found in task `{}`", task.name));
            }
            if !trimmed.contains('/') && !trimmed.contains('.') {
                return Err(format!(
                    "scope `{}` in task `{}` looks non-file path",
                    trimmed, task.name
                ));
            }
            if !seen.insert(trimmed.to_string()) {
                return Err(format!("duplicated scope `{}` in task `{}`", trimmed, task.name));
            }
        }
    }
    Ok(())
}

pub(crate) fn draft_create() -> Result<String, String> {
    let _ = crate::action_sync_project_tasks_list_from_project_md(Path::new("."))?;
    let project_root = Path::new(".");
    let path = crate::action_resolve_drafts_list_path(project_root)?;
    let preflight_msg = crate::action_preflight_draft_create(&path)?;
    let mut doc = crate::action_load_drafts_list(&path)?;
    crate::action_sync_draft_state_doc(project_root, &mut doc);
    crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc)?;
    let project_md_path = crate::action_resolve_project_md_path_for_flow();
    let project_md = fs::read_to_string(&project_md_path)
        .map_err(|e| format!("failed to read {}: {}", project_md_path.display(), e))?;
    let project_info = crate::calc_extract_project_info(&project_md);
    let project_rules = crate::calc_extract_project_rules(&project_md);
    let debug_instruction = calc_debug_prompt_instruction();
    let debug_enabled = crate::action_load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    let retry_on_fail = crate::action_load_app_config()
        .as_ref()
        .is_some_and(crate::config::AppConfig::draft_retry_on_fail_enabled);
    let mut created = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut attempt_targets = doc.planned.clone();
    let max_attempt = if retry_on_fail { 2 } else { 1 };
    for attempt in 1..=max_attempt {
        if attempt_targets.is_empty() {
            break;
        }
        let mut handles = Vec::new();
        for feature in &attempt_targets {
            let feature_name = feature.clone();
            let prompt = action_build_draft_prompt(
                &doc,
                &feature_name,
                &project_info,
                &project_rules,
                &debug_instruction,
            );
            handles.push(thread::spawn(move || {
                let result = action_generate_valid_draft_yaml(&prompt, &feature_name, debug_enabled);
                (feature_name, result)
            }));
        }

        let mut generated: Vec<(String, String)> = Vec::new();
        let mut next_failures: Vec<(String, String)> = Vec::new();
        for handle in handles {
            let joined = handle
                .join()
                .map_err(|_| "draft generation worker panicked".to_string())?;
            let (feature, result) = joined;
            if let Err(e) = result {
                action_append_draft_runtime_log(
                    debug_enabled,
                    &feature,
                    "완료/실패",
                    &format!("실패(attempt {}): {}", attempt, e),
                );
                let _ = action_write_draft_failure_report(&feature, &e);
                next_failures.push((feature, e.clone()));
                if !retry_on_fail {
                    crate::action_sync_draft_state_doc(project_root, &mut doc);
                    let _ =
                        crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc);
                    return Err(format!("create-draft failed at `{}`: {}", next_failures[0].0, e));
                }
            } else {
                generated.push((feature, result.unwrap_or_default()));
            }
        }

        let known_features: HashSet<String> = doc.planned.iter().cloned().collect();
        for (feature, draft_yaml) in generated {
            match action_second_pass_check(&feature, &draft_yaml, &known_features)
                .and_then(|_| action_write_generated_draft(&feature, &draft_yaml, debug_enabled))
            {
                Ok(_) => {
                    action_append_draft_runtime_log(debug_enabled, &feature, "완료/실패", "완료");
                    created.push(feature);
                }
                Err(e) => {
                    action_append_draft_runtime_log(
                        debug_enabled,
                        &feature,
                        "완료/실패",
                        &format!("실패(attempt {}): {}", attempt, e),
                    );
                    let _ = action_write_draft_failure_report(&feature, &e);
                    next_failures.push((feature, e));
                }
            }
            crate::action_sync_draft_state_doc(project_root, &mut doc);
            let _ = crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc);
        }
        if next_failures.is_empty() {
            failures.clear();
            break;
        }
        failures = next_failures;
        if attempt < max_attempt {
            crate::action_sync_draft_state_doc(project_root, &mut doc);
            let _ = crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc);
            attempt_targets = doc.draft_state.pending.clone();
        }
    }
    crate::action_sync_draft_state_doc(project_root, &mut doc);
    crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc)?;
    if !failures.is_empty() {
        let pending_names: Vec<String> = failures.into_iter().map(|(name, _)| name).collect();
        return Err(format!(
            "create-draft retry exhausted; pending: {}",
            pending_names.join(", ")
        ));
    }
    created.sort();
    created.dedup();
    let check_msg = crate::action_run_check_code_after_draft_changes(&created, "create-draft")?;
    Ok(format!(
        "{}; draft-create completed with llm: {} item(s) from drafts_list.yaml.planned | {}",
        preflight_msg,
        created.len(),
        check_msg,
    ))
}

pub(crate) fn draft_add(feature_name: &str, request: Option<String>) -> Result<String, String> {
    let request_text = match request {
        Some(v) if !v.trim().is_empty() => v,
        _ => crate::action_read_one_line("draft 추가 요구사항을 입력하세요: ")?,
    };
    if request_text.trim().is_empty() {
        return Err("draft-add requires non-empty request".to_string());
    }
    let project_md = fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let project_info = crate::calc_extract_project_info(&project_md);
    let project_rules = crate::calc_extract_project_rules(&project_md);
    let debug_instruction = calc_debug_prompt_instruction();
    let prompt = format!(
        "너는 rust-orc 프로젝트의 draft 작성기다.\nproject info:\n{}\n\nproject rules:\n- {}\n\n입력 기능명:\n- {}\n요구사항:\n- {}\n\n지시:\n- `draft.yaml`은 템플릿(`assets/code/templates/draft.yaml`)을 대상 폴더에 먼저 복사한 뒤, 주석/예시를 지우고 값만 수정해.\n- 규칙은 `$plan-drafts-code`, `$rule-naming` 스킬을 사용해.\n- YAML 중복 키를 절대 만들지 마(특히 `rule`/`contracts`).\n- `task` 키는 `name,type,domain,depends_on,scope,rule,step,touches,contracts`만 허용.\n- `contracts`는 `key=value` 또는 `key: value` 형식으로만 작성하고 `contract` 키는 금지.\n{}출력 형식:\nFEATURE_NAME: <snake_case>\n```yaml\n<draft.yaml 본문>\n```\n설명 문장 금지.",
        project_info,
        project_rules.join("\n- "),
        feature_name,
        request_text,
        debug_instruction
    );
    let generated_name = feature_name.to_string();
    let debug_enabled = crate::action_load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    let draft_yaml = action_generate_valid_draft_yaml(&prompt, &generated_name, debug_enabled)?;
    crate::add_feature_to_planned(&generated_name)?;
    let draft_path = crate::ui::action_apply_draft_create_update_delete(
        crate::ui::DraftCommand::Create,
        &generated_name,
        None,
    )?;
    fs::write(&draft_path, &draft_yaml)
        .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
    let check_msg =
        crate::action_run_check_code_after_draft_changes(&[generated_name.clone()], "add-draft")?;
    Ok(format!(
        "draft-add completed with llm: planned+file updated for {} ({}) | {}",
        generated_name,
        draft_path.display(),
        check_msg
    ))
}

pub(crate) fn draft_delete(feature_name: &str) -> Result<String, String> {
    let answer = crate::action_read_one_line(&format!(
        "delete `.project/feature/{}/draft.yaml` ? [y/N]: ",
        feature_name
    ))?;
    let accepted = matches!(answer.to_ascii_lowercase().as_str(), "y" | "yes");
    if !accepted {
        return Ok("draft-delete canceled".to_string());
    }
    let path = crate::ui::action_apply_draft_create_update_delete(
        crate::ui::DraftCommand::Delete,
        feature_name,
        None,
    )?;
    Ok(format!("draft deleted: {}", path.display()))
}
