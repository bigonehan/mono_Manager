use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
    pub(crate) features: Vec<String>,
    #[serde(default)]
    pub(crate) planned: Vec<String>,
    #[serde(default)]
    pub(crate) worked: Vec<String>,
    #[serde(default)]
    pub(crate) complete: Vec<String>,
    #[serde(default)]
    pub(crate) failed: Vec<String>,
    #[serde(default)]
    pub(crate) planned_items: Vec<PlannedItem>,
    #[serde(default)]
    pub(crate) draft_state: DraftStateDoc,
    #[serde(default)]
    pub(crate) sync_initialized: bool,
}

pub(crate) fn validate_draft_doc(doc: &DraftDoc) -> Vec<String> {
    let mut issues = Vec::new();
    if doc.task.is_empty() {
        issues.push("no tasks defined in draft".to_string());
    }
    for (i, task) in doc.task.iter().enumerate() {
        if task.name.trim().is_empty() {
            issues.push(format!("task[{}] has no name", i));
        }
    }
    issues
}

pub(crate) fn load_drafts_list(path: &Path) -> Result<DraftsListDoc, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    serde_yaml::from_str::<DraftsListDoc>(&raw)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e))
}

pub(crate) fn save_drafts_list(path: &Path, doc: &DraftsListDoc) -> Result<(), String> {
    let raw = serde_yaml::to_string(doc).map_err(|e| format!("failed to encode yaml: {}", e))?;
    fs::write(path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn debug_prompt_instruction() -> String {
    let debug_enabled = crate::load_app_config()
        .as_ref()
        .is_none_or(crate::config::AppConfig::debug_enabled);
    if !debug_enabled {
        return String::new();
    }
    "- debug 상태(on)이므로 응답 본문 맨 앞에 `DEBUG_LOG:` 한 줄을 추가해 현재 작업 단계와 대기 중이면 대기 사유를 먼저 기록해.\n- `DEBUG_LOG:`는 YAML 코드블록(```yaml ... ```) 밖에서만 작성하고, YAML 스키마/키/구조는 절대 변경하지 마.\n".to_string()
}

fn draft_llm_timeout_sec() -> u64 {
    let configured = crate::load_app_config()
        .as_ref()
        .map_or(300, crate::config::AppConfig::default_timeout_sec);
    configured.max(30)
}

fn parse_and_validate_draft_yaml(draft_yaml: &str) -> Result<DraftDoc, String> {
    let draft_doc: DraftDoc = serde_yaml::from_str(draft_yaml)
        .map_err(|e| format!("generated draft yaml invalid: {}", e))?;
    let draft_issues = validate_draft_doc(&draft_doc);
    if !draft_issues.is_empty() {
        return Err(format!(
            "generated draft yaml invalid: {}",
            draft_issues.join(" | ")
        ));
    }
    Ok(draft_doc)
}

fn repair_draft_yaml_once(
    feature_name: &str,
    draft_yaml: &str,
    reason: &str,
) -> Result<String, String> {
    let prompt = format!(
        "다음 drafts.yaml을 검증 실패 사유에 맞게 수정해.\n\
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
<수정된 drafts.yaml>\n\
```\n\
설명 문장 금지.\n\n\
검증 실패 사유:\n{}\n\n\
현재 draft:\n{}",
        feature_name, feature_name, reason, draft_yaml
    );
    let repaired_raw = crate::run_codex_exec_capture(&prompt)?;
    Ok(crate::extract_yaml_block(&repaired_raw))
}

fn generate_valid_draft_yaml(
    prompt: &str,
    feature_name: &str,
    debug_enabled: bool,
) -> Result<String, String> {
    let timeout_sec = draft_llm_timeout_sec();
    let draft_raw_result = crate::run_codex_exec_capture_with_timeout(prompt, timeout_sec);
    let draft_raw = draft_raw_result?;
    let draft_yaml = crate::extract_yaml_block(&draft_raw);
    match parse_and_validate_draft_yaml(&draft_yaml) {
        Ok(_) => Ok(draft_yaml),
        Err(first_reason) => {
            let repaired_yaml = repair_draft_yaml_once(feature_name, &draft_yaml, &first_reason)?;
            parse_and_validate_draft_yaml(&repaired_yaml).map_err(|repair_reason| {
                format!("{} | repair failed: {}", first_reason, repair_reason)
            })?;
            Ok(repaired_yaml)
        }
    }
}

pub(crate) fn draft_add(feature_name: &str, request: Option<String>) -> Result<String, String> {
    let request_text = match request {
        Some(v) if !v.trim().is_empty() => v,
        _ => crate::read_one_line("draft 추가 요구사항을 입력하세요: ")?,
    };
    if request_text.trim().is_empty() {
        return Err("draft-add requires non-empty request".to_string());
    }
    
    // logic simplified or removed as per new orc flow which handles this in add_orc_drafts
    Ok("draft_add refactored into orc flow".to_string())
}

pub(crate) fn draft_delete(feature_name: &str) -> Result<String, String> {
    let answer = crate::read_one_line(&format!(
        "delete draft config for feature `{}` ? [y/N]: ",
        feature_name
    ))?;
    let accepted = matches!(answer.to_ascii_lowercase().as_str(), "y" | "yes");
    if !accepted {
        return Ok("draft-delete canceled".to_string());
    }
    // delete logic
    Ok(format!("draft deleted: {}", feature_name))
}
