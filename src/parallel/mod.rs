use crate::{append_failure_log, build_task_prompt, check_and_improve_drafts_before_parallel, collect_parallel_feature_tasks, default_model_bin, initialize_parallel_workspace_if_empty, load_app_config, move_finished_features_to_clear, preflight_parallel_build, print_parallel_modal, promote_planned_to_features, read_project_info, resolve_task_template_path, write_parallel_feedback, model_supports_dangerous_flag, config, ui};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

fn update_task_status(
    statuses: &[(String, ui::TaskRuntimeState)],
    target: &str,
    state: ui::TaskRuntimeState,
) -> Vec<(String, ui::TaskRuntimeState)> {
    statuses
        .iter()
        .map(|(name, old)| {
            if name == target {
                (name.clone(), state)
            } else {
                (name.clone(), *old)
            }
        })
        .collect()
}

async fn run_one_parallel_task(
    semaphore: Arc<Semaphore>,
    model_bin: String,
    task_name: String,
    prompt: String,
    timeout_sec: u64,
    dangerous_bypass: bool,
    debug_enabled: bool,
) -> Result<String, String> {
    let _permit = semaphore
        .acquire_owned()
        .await
        .map_err(|e| format!("failed to acquire semaphore: {}", e))?;
    append_task_runtime_log(
        debug_enabled,
        &task_name,
        "시작/프롬프트 전송",
        "codex exec 호출을 시작했습니다.",
    );
    let mut cmd = tokio::process::Command::new(&model_bin);
    cmd.arg("exec");
    if dangerous_bypass && model_supports_dangerous_flag(&model_bin) {
        cmd.arg("--dangerously-bypass-approvals-and-sandbox");
    }
    cmd.arg(prompt);
    let run_fut = cmd.status();
    let status = tokio::time::timeout(Duration::from_secs(timeout_sec), run_fut)
        .await
        .map_err(|_| {
            append_task_runtime_log(
                debug_enabled,
                &task_name,
                "완료/실패",
                &format!("timeout ({timeout_sec}s)"),
            );
            format!("timeout ({timeout_sec}s) for {task_name}")
        })?
        .map_err(|e| {
            append_task_runtime_log(
                debug_enabled,
                &task_name,
                "완료/실패",
                &format!("프로세스 실행 실패: {}", e),
            );
            format!("failed to run command for {task_name}: {}", e)
        })?;
    append_task_runtime_log(
        debug_enabled,
        &task_name,
        "LLM 응답 수신",
        &format!("codex exec 종료 code={:?}", status.code()),
    );
    append_task_runtime_log(
        debug_enabled,
        &task_name,
        "검증 단계",
        "종료 코드 기반 성공/실패 판정을 진행합니다.",
    );
    if status.success() {
        append_task_runtime_log(
            debug_enabled,
            &task_name,
            "파일 반영 단계",
            "codex 작업 결과를 워크스페이스에 반영 완료로 간주합니다.",
        );
        append_task_runtime_log(
            debug_enabled,
            &task_name,
            "완료/실패",
            "완료",
        );
        Ok(task_name)
    } else {
        append_task_runtime_log(
            debug_enabled,
            &task_name,
            "완료/실패",
            &format!("실패 code={:?}", status.code()),
        );
        Err(format!(
            "{} failed with exit code {:?}",
            task_name,
            status.code()
        ))
    }
}

fn append_task_runtime_log(
    debug_enabled: bool,
    task_name: &str,
    stage: &str,
    detail: &str,
) {
    if !debug_enabled {
        return;
    }
    let runtime_dir = Path::new(".project").join("runtime");
    if fs::create_dir_all(&runtime_dir).is_err() {
        return;
    }
    let log_path = runtime_dir.join(format!("{}.log", task_name));
    let mut file = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = writeln!(file, "[{}] {} | {}", crate::now_unix(), stage, detail);
}

pub async fn run_parallel_build_code() -> Result<String, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    if let Some(init_msg) = initialize_parallel_workspace_if_empty(&cwd)? {
        println!("{}", init_msg);
    }

    let app_conf = load_app_config();
    let max_parallel = app_conf.as_ref().map_or(10, config::AppConfig::default_max_parallel);
    let timeout_sec = app_conf.as_ref().map_or(300, config::AppConfig::default_timeout_sec);
    let auto_yes = app_conf.as_ref().is_none_or(config::AppConfig::auto_yes_enabled);
    let dangerous_bypass = app_conf
        .as_ref()
        .is_none_or(config::AppConfig::dangerous_bypass_enabled);
    let debug_enabled = app_conf
        .as_ref()
        .is_none_or(config::AppConfig::debug_enabled);
    let model_bin = default_model_bin();

    let tasks_list_path = Path::new(".project").join("drafts_list.yaml");
    let preflight_msg = preflight_parallel_build(&tasks_list_path)?;
    println!("{}", preflight_msg);
    let check_msg = check_and_improve_drafts_before_parallel()?;
    println!("{}", check_msg);

    let project_info = read_project_info()?;
    let task_template_path = resolve_task_template_path()?;
    let task_template = fs::read_to_string(&task_template_path)
        .map_err(|e| format!("failed to read {}: {}", task_template_path.display(), e))?;
    let mut pending = collect_parallel_feature_tasks()?;
    if pending.is_empty() {
        return Ok("no feature draft to run".to_string());
    }

    let mut statuses: Vec<(String, ui::TaskRuntimeState)> = pending
        .iter()
        .map(|t| (t.name.clone(), ui::TaskRuntimeState::Inactive))
        .collect();
    print_parallel_modal(&statuses);

    let semaphore = Arc::new(Semaphore::new(max_parallel));
    let mut finished: HashSet<String> = HashSet::new();
    let mut success = 0usize;
    let mut failed = 0usize;

    loop {
        if pending.is_empty() {
            break;
        }
        let runnable_names: HashSet<String> = pending
            .iter()
            .filter(|task| task.depends_on.iter().all(|dep| finished.contains(dep)))
            .map(|task| task.name.clone())
            .collect();

        if runnable_names.is_empty() {
            for task in pending {
                failed += 1;
                let reason = format!("blocked by unresolved depends_on: {:?}", task.depends_on);
                let _ = append_failure_log(&task.name, &reason);
            }
            break;
        }

        let mut round = Vec::new();
        let mut remain = Vec::new();
        for task in pending {
            if runnable_names.contains(&task.name) {
                round.push(task);
            } else {
                remain.push(task);
            }
        }
        pending = remain;

        let mut handles = Vec::new();
        for task in round {
            statuses = update_task_status(&statuses, &task.name, ui::TaskRuntimeState::Active);
            print_parallel_modal(&statuses);
            let prompt = build_task_prompt(&task_template, &project_info, &task.draft_path)?;
            handles.push(tokio::spawn(run_one_parallel_task(
                semaphore.clone(),
                model_bin.clone(),
                task.name.clone(),
                prompt,
                timeout_sec,
                dangerous_bypass,
                debug_enabled,
            )));
        }

        for handle in handles {
            match handle.await {
                Ok(Ok(name)) => {
                    success += 1;
                    finished.insert(name.clone());
                    statuses = update_task_status(&statuses, &name, ui::TaskRuntimeState::Clear);
                    print_parallel_modal(&statuses);
                }
                Ok(Err(reason)) => {
                    failed += 1;
                    let task_name = reason.split_whitespace().next().unwrap_or("parallel_task");
                    let _ = append_failure_log(task_name, &reason);
                }
                Err(join_err) => {
                    failed += 1;
                    let _ = append_failure_log("parallel_task", &join_err.to_string());
                }
            }
        }
    }
    let finished_list: Vec<String> = finished.into_iter().collect();
    promote_planned_to_features(&finished_list)?;
    let move_msg = move_finished_features_to_clear(&finished_list)?;
    let feedback_msg = write_parallel_feedback(&finished_list, failed, &move_msg)?;
    Ok(format!(
        "run_parallel_build_code finished: success={}, failed={} | {} | {}",
        success, failed, move_msg, feedback_msg
    ))
}

pub async fn press_key(key: &str) -> Result<String, String> {
    let config = load_app_config();
    let run_parallel_key = config
        .as_ref()
        .map_or("p", config::AppConfig::run_parallel_key);
    if key == run_parallel_key {
        run_parallel_build_code().await
    } else {
        Err(format!("unmapped key: {} (run_parallel key: {})", key, run_parallel_key))
    }
}
