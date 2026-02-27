use std::path::Path;

use serde::Deserialize;

pub(crate) fn add_plan(request_input: Option<String>) -> Result<String, String> {
    let tasks_list_path = crate::action_resolve_drafts_list_path(Path::new("."))?;
    let mut doc = crate::action_load_drafts_list(&tasks_list_path)?;
    if !doc.planned.is_empty() {
        return Ok("add-plan skipped: drafts_list.yaml.planned already exists".to_string());
    }
    let project_md = std::fs::read_to_string(crate::PROJECT_MD_PATH)
        .map_err(|e| format!("failed to read {}: {}", crate::PROJECT_MD_PATH, e))?;
    let project_info = crate::calc_extract_project_info(&project_md);
    let project_rules = crate::calc_extract_project_rules(&project_md);
    let request_hint = request_input.unwrap_or_default();
    let features_text = if doc.features.is_empty() {
        "- (none)".to_string()
    } else {
        doc.features
            .iter()
            .map(|v| format!("- {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prompt = format!(
        "너는 프로젝트의 초기 개발 계획을 잡는 planner다.\n\
project info:\n{}\n\n\
project rules:\n- {}\n\n\
already features:\n{}\n\n\
user hint:\n{}\n\n\
`.project/drafts_list.yaml`의 planned에 넣을 key 목록만 생성해.\n\
YAML/Markdown 참조 파일이 있으면 먼저 읽고 값을 채워야 할 헤더/속성을 정리한 뒤 형식에 맞게 추가해.\n\
규칙은 `$plan-drafts-code` 스킬을 사용해.\n\
출력은 YAML만:\n\
planned:\n\
  - <snake_case key>",
        project_info,
        project_rules.join("\n- "),
        features_text,
        request_hint
    );
    let raw = crate::action_run_codex_exec_capture(&prompt)?;
    let yaml = crate::calc_extract_yaml_block(&raw);
    #[derive(Debug, Deserialize)]
    struct AddPlanDoc {
        #[serde(default)]
        planned: Vec<String>,
    }
    let parsed: AddPlanDoc =
        serde_yaml::from_str(&yaml).map_err(|e| format!("add-plan yaml parse failed: {}", e))?;
    let mut next_planned = Vec::new();
    for item in parsed.planned {
        let key = crate::calc_feature_name_snake_like(&item);
        if !crate::calc_is_valid_snake_feature_key(&key)
            || doc.features.iter().any(|v| v == &key)
            || next_planned.iter().any(|v| v == &key)
        {
            continue;
        }
        next_planned.push(key);
    }
    if next_planned.is_empty() {
        return Err("add-plan produced empty planned list".to_string());
    }
    doc.planned = next_planned;
    crate::action_save_drafts_list(&tasks_list_path, &doc)?;
    Ok(format!(
        "add-plan completed: {} item(s) added to drafts_list.yaml.planned",
        doc.planned.len()
    ))
}
