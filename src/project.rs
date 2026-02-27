use std::env;
use std::fs;
use std::path::PathBuf;

pub(crate) fn auto_mode(project_name: Option<&str>) -> Result<String, String> {
    let registry = crate::action_load_registry(&crate::action_registry_path())?;
    let target = if let Some(name) = project_name {
        registry.projects.iter().find(|p| p.name == name)
    } else {
        registry.projects.iter().find(|p| p.selected)
    }
    .ok_or_else(|| "auto mode requires a selected project".to_string())?;

    let pane_id = crate::tmux::action_current_pane_id().map_err(|_| {
        "auto mode warning: tmux pane is not active. open tmux and retry.".to_string()
    })?;
    crate::tmux::action_rename_pane(&pane_id, "plan")?;

    let project_root = PathBuf::from(&target.path);
    let project_md_path = project_root.join(".project").join("project.md");
    let project_info = fs::read_to_string(&project_md_path).unwrap_or_else(|_| {
        format!(
            "# info\n- name: {}\n- description: {}\n- path: {}",
            target.name, target.description, target.path
        )
    });
    let features = crate::action_collect_project_features(&project_root)?;
    let features_text = if features.is_empty() {
        "- (none)".to_string()
    } else {
        features
            .iter()
            .map(|f| format!("- {}", f))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let auto_prompt = format!(
        "You are in auto mode for project `{}`.\n\
1) Use web search to find similar apps/services for this project context.\n\
2) Propose missing features and pick high-impact items.\n\
3) Create/update drafts under `.project/feature/*/draft.yaml`.\n\
4) Implement all selected features in this repository with minimal safe changes.\n\
5) Run project tests/lint required by project rules.\n\
If a YAML/Markdown file is referenced, read it first, identify headers/properties to fill, then append in the required format.\n\
Output a short action log at the end.\n\n\
Current project info:\n{}\n\nCurrent feature list:\n{}",
        target.name, project_info, features_text
    );
    let _ = crate::action_run_codex_exec_capture_in_dir(&project_root, &auto_prompt)?;

    let _ = crate::action_run_command_in_dir(&project_root, "cargo", &["test"], "cargo test")?;
    let _ = crate::action_run_command_in_dir(
        &project_root,
        "jj",
        &["commit", "-m", "auto mode: feature completion after passing tests"],
        "jj commit",
    )?;
    Ok(format!(
        "auto mode completed: project={} pane={} tests=passed committed=yes",
        target.name, pane_id
    ))
}

pub(crate) fn create_project(
    name: &str,
    path: Option<&str>,
    description: &str,
) -> Result<String, String> {
    let target = path
        .map(PathBuf::from)
        .unwrap_or_else(crate::calc_default_project_path);

    crate::action_ensure_project_dir(&target)?;

    let existing = crate::calc_is_existing_project(&target);
    if !existing {
        fs::create_dir_all(target.join(".project"))
            .map_err(|e| format!("failed to create .project: {}", e))?;
    }
    let registry_path = crate::action_registry_path();
    let registry = crate::action_load_registry(&registry_path)?;
    let upserted = crate::action_upsert_project(&registry, name, &target, description);
    let selected = crate::calc_select_only(&upserted, name);
    crate::action_save_registry(&registry_path, &selected)?;

    let project_md_path = target.join(crate::PROJECT_MD_PATH);
    let mut create_project_plan_msg = String::new();
    let defer_project_plan = env::var("ORC_DEFER_PROJECT_PLAN")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false);
    if !project_md_path.exists() && !defer_project_plan {
        create_project_plan_msg = crate::action_generate_project_plan(
            &target,
            name,
            description,
            "",
            "",
            &[],
            "",
            None,
        )?;
    }

    if existing {
        if create_project_plan_msg.is_empty() {
            Ok(format!("loaded existing project: {} ({})", name, target.display()))
        } else {
            Ok(format!(
                "loaded existing project: {} ({}) | {}",
                name,
                target.display(),
                create_project_plan_msg
            ))
        }
    } else if create_project_plan_msg.is_empty() {
        Ok(format!("created project: {} ({})", name, target.display()))
    } else {
        Ok(format!(
            "created project: {} ({}) | {}",
            name,
            target.display(),
            create_project_plan_msg
        ))
    }
}

pub(crate) fn select_project(name: &str) -> Result<String, String> {
    let registry_path = crate::action_registry_path();
    let registry = crate::action_load_registry(&registry_path)?;
    let exists = registry.projects.iter().any(|p| p.name == name);
    if !exists {
        return Err(format!("project not found: {}", name));
    }
    let updated = crate::calc_select_only(&registry, name);
    crate::action_save_registry(&registry_path, &updated)?;
    Ok(format!("selected project: {}", name))
}

pub(crate) fn delete_project(name: &str) -> Result<String, String> {
    let registry_path = crate::action_registry_path();
    let registry = crate::action_load_registry(&registry_path)?;
    let updated = crate::action_delete_project(&registry, name);
    crate::action_save_registry(&registry_path, &updated)?;
    Ok(format!("deleted project: {}", name))
}
