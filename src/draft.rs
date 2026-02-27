use std::fs;
use std::path::Path;

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
    let mut created = Vec::new();
    let mut next_planned = Vec::new();
    let planned_snapshot = doc.planned.clone();
    for feature in &planned_snapshot {
        let feature_request = doc
            .planned_items
            .iter()
            .find(|item| item.name == *feature)
            .map(|item| item.value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| feature.clone());
        let result: Result<(), String> = (|| {
            let prompt = format!(
                "너는 rust-orc 프로젝트의 draft 작성기다.\nproject info:\n{}\n\nproject rules:\n- {}\n\n입력 기능 key:\n- {}\n입력 기능 설명:\n- {}\n\n지시:\n- `draft.yaml`은 템플릿(`/home/tree/ai/skills/plan-drafts/references/draft.yaml`)을 대상 폴더에 먼저 복사한 뒤, 주석/예시를 지우고 값만 수정해.\n- 규칙은 `$plan-drafts-code`, `$feature-name-prompt-rules` 스킬을 사용해.\n- FEATURE_NAME은 반드시 입력 기능 key와 동일하게 출력해.\n출력 형식:\nFEATURE_NAME: <snake_case>\n```yaml\n<draft.yaml 본문>\n```\n설명 문장 금지.",
                project_info,
                project_rules.join("\n- "),
                feature,
                feature_request,
            );
            let draft_raw = crate::action_run_codex_exec_capture(&prompt)?;
            let feature_name = feature.clone();
            let draft_yaml = crate::calc_extract_yaml_block(&draft_raw);
            let draft_doc: crate::DraftDoc = serde_yaml::from_str(&draft_yaml)
                .map_err(|e| format!("generated draft yaml invalid: {}", e))?;
            let draft_issues = crate::action_validate_draft_doc(&draft_doc);
            if !draft_issues.is_empty() {
                return Err(format!(
                    "generated draft yaml invalid: {}",
                    draft_issues.join(" | ")
                ));
            }
            let draft_path = crate::ui::action_apply_draft_create_update_delete(
                crate::ui::DraftCommand::Create,
                &feature_name,
                None,
            )?;
            fs::write(&draft_path, &draft_yaml)
                .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
            let task_path = crate::ui::action_resolve_feature_draft_path(&feature_name)
                .parent()
                .ok_or_else(|| "failed to resolve feature dir".to_string())?
                .join("task.yaml");
            fs::write(&task_path, &draft_yaml)
                .map_err(|e| format!("failed to write {}: {}", task_path.display(), e))?;
            if !next_planned.iter().any(|v| v == &feature_name)
                && !doc.features.iter().any(|v| v == &feature_name)
            {
                next_planned.push(feature_name.clone());
            }
            created.push(feature_name);
            Ok(())
        })();
        if let Err(e) = result {
            doc.planned = next_planned;
            crate::action_sync_draft_state_doc(project_root, &mut doc);
            let _ = crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc);
            return Err(e);
        }
        doc.planned = next_planned.clone();
        crate::action_sync_draft_state_doc(project_root, &mut doc);
        let _ = crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc);
    }
    doc.planned = next_planned;
    crate::action_sync_draft_state_doc(project_root, &mut doc);
    crate::action_save_drafts_list_primary_with_legacy_mirror(project_root, &doc)?;
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
    let prompt = format!(
        "너는 rust-orc 프로젝트의 draft 작성기다.\nproject info:\n{}\n\nproject rules:\n- {}\n\n입력 기능명:\n- {}\n요구사항:\n- {}\n\n지시:\n- `draft.yaml`은 템플릿(`/home/tree/ai/skills/plan-drafts/references/draft.yaml`)을 대상 폴더에 먼저 복사한 뒤, 주석/예시를 지우고 값만 수정해.\n- 규칙은 `$plan-drafts-code`, `$feature-name-prompt-rules` 스킬을 사용해.\n출력 형식:\nFEATURE_NAME: <snake_case>\n```yaml\n<draft.yaml 본문>\n```\n설명 문장 금지.",
        project_info,
        project_rules.join("\n- "),
        feature_name,
        request_text
    );
    let draft_raw = crate::action_run_codex_exec_capture(&prompt)?;
    let generated_name = crate::calc_extract_feature_name(&draft_raw, feature_name);
    let draft_yaml = crate::calc_extract_yaml_block(&draft_raw);
    let draft_doc: crate::DraftDoc = serde_yaml::from_str(&draft_yaml)
        .map_err(|e| format!("generated draft yaml invalid: {}", e))?;
    let draft_issues = crate::action_validate_draft_doc(&draft_doc);
    if !draft_issues.is_empty() {
        return Err(format!(
            "generated draft yaml invalid: {}",
            draft_issues.join(" | ")
        ));
    }
    crate::add_feature_to_planned(&generated_name)?;
    let draft_path = crate::ui::action_apply_draft_create_update_delete(
        crate::ui::DraftCommand::Create,
        &generated_name,
        None,
    )?;
    fs::write(&draft_path, &draft_yaml)
        .map_err(|e| format!("failed to write {}: {}", draft_path.display(), e))?;
    let task_path = crate::ui::action_resolve_feature_draft_path(&generated_name)
        .parent()
        .ok_or_else(|| "failed to resolve feature dir".to_string())?
        .join("task.yaml");
    fs::write(&task_path, &draft_yaml)
        .map_err(|e| format!("failed to write {}: {}", task_path.display(), e))?;
    let check_msg =
        crate::action_run_check_code_after_draft_changes(&[generated_name.clone()], "add-draft")?;
    Ok(format!(
        "draft-add completed with llm: planned+file updated for {} ({}) | {}",
        generated_name,
        task_path.display(),
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
