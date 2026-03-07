use axum::extract::{Query, State};
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    repo_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProjectType {
    Story,
    Movie,
    Code,
    Mono,
}

impl Default for ProjectType {
    fn default() -> Self {
        Self::Code
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectRecord {
    #[serde(default)]
    id: String,
    name: String,
    path: String,
    description: String,
    created_at: String,
    updated_at: String,
    selected: bool,
    #[serde(default)]
    project_type: ProjectType,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<ProjectState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProjectRegistry {
    #[serde(default, rename = "recentActivepane")]
    recent_active_pane: String,
    #[serde(default)]
    projects: Vec<ProjectRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProjectState {
    Init,
    Basic,
    Work,
    Wait,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DomainDetail {
    name: String,
    description: String,
    features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectDetail {
    id: String,
    name: String,
    description: String,
    path: String,
    memo: String,
    project_type: ProjectType,
    spec: String,
    goal: String,
    rules: Vec<String>,
    constraints: Vec<String>,
    features: Vec<String>,
    domains: Vec<DomainDetail>,
    planned: Vec<String>,
    #[serde(rename = "plannedDisplay")]
    planned_display: Vec<String>,
    generated: Vec<String>,
    state: ProjectState,
    #[serde(rename = "hasDraftsYaml")]
    has_drafts_yaml: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DraftsListDoc {
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    planned: Vec<String>,
    #[serde(default)]
    worked: Vec<String>,
    #[serde(default)]
    complete: Vec<String>,
    #[serde(default)]
    failed: Vec<String>,
    #[serde(default, rename = "planned_items")]
    planned_items: Vec<PlannedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PlannedItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DraftsDoc {
    #[serde(default)]
    planned: Vec<String>,
    #[serde(default)]
    worked: Vec<String>,
    #[serde(default)]
    complete: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedProjectMd {
    name: String,
    description: String,
    spec: String,
    goal: String,
    rules: Vec<String>,
    constraints: Vec<String>,
    features: Vec<String>,
    domains: Vec<DomainDetail>,
}

#[derive(Debug, Deserialize)]
struct ProjectDetailQuery {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CreateProjectRequest {
    name: String,
    description: String,
    path: String,
    #[serde(default)]
    spec: String,
    #[serde(default)]
    project_type: Option<ProjectType>,
}

#[derive(Debug, Deserialize)]
struct LoadProjectRequest {
    path: String,
    #[serde(default)]
    create_if_missing: bool,
    #[serde(default)]
    project_type: Option<ProjectType>,
}

#[derive(Debug, Deserialize)]
struct ProjectSelectRequest {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ProjectDeleteRequest {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ProjectInfoRequest {
    id: String,
    name: String,
    description: String,
    spec: String,
    goal: String,
}

#[derive(Debug, Deserialize)]
struct ProjectListsRequest {
    id: String,
    rules: Vec<String>,
    constraints: Vec<String>,
    features: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectMemoRequest {
    id: String,
    memo: String,
}

#[derive(Debug, Deserialize)]
struct RunRequest {
    id: String,
    action: String,
    #[serde(default)]
    payload: String,
}

#[derive(Debug, Deserialize)]
struct BrowseQuery {
    #[serde(default)]
    path: String,
}

#[derive(Debug, Serialize)]
struct BrowseEntry {
    name: String,
    path: String,
    #[serde(rename = "hasProjectMeta")]
    has_project_meta: bool,
}

#[derive(Debug, Clone, Serialize)]
struct MonorepoPackageEntry {
    id: String,
    name: String,
    path: String,
    kind: String,
}

pub(crate) async fn serve_web_api(addr: &str) -> Result<String, String> {
    let repo_root = std::env::current_dir().map_err(|e| format!("failed to get cwd: {}", e))?;
    let state = Arc::new(AppState { repo_root });
    let router = Router::new()
        .route("/api/projects", get(get_projects).post(post_projects))
        .route("/api/project-load", post(post_project_load))
        .route("/api/project-browse", get(get_project_browse))
        .route("/api/monorepo-sync", post(post_monorepo_sync))
        .route("/api/project-detail", get(get_project_detail))
        .route("/api/project-select", post(post_project_select))
        .route("/api/project-delete", post(post_project_delete))
        .route("/api/project-info", post(post_project_info))
        .route("/api/project-lists", post(post_project_lists))
        .route("/api/project-memo", post(post_project_memo))
        .route("/api/run", post(post_run))
        .route("/api/tui-map", get(get_tui_map))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(Any),
        )
        .with_state(state);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("failed to bind {}: {}", addr, e))?;
    axum::serve(listener, router.into_make_service())
        .await
        .map_err(|e| format!("server failed: {}", e))?;
    Ok(format!("web api stopped: {}", addr))
}

async fn get_projects(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match list_projects(&state.repo_root) {
        Ok(projects) => ok_json(json!({ "projects": projects })),
        Err(e) => err_json(e),
    }
}

async fn post_projects(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    match create_project(&state.repo_root, body) {
        Ok(project) => ok_json(json!({ "project": project })),
        Err(e) => err_json(e),
    }
}

async fn post_project_load(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoadProjectRequest>,
) -> impl IntoResponse {
    match load_project_from_path(&state.repo_root, body) {
        Ok((project, created_project_meta)) => match load_project_detail(&state.repo_root, &project.id) {
            Ok(detail) => ok_json(json!({
                "project": project,
                "detail": detail,
                "created_project_meta": created_project_meta
            })),
            Err(e) => err_json(e),
        },
        Err(e) => err_json(e),
    }
}

async fn get_project_detail(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProjectDetailQuery>,
) -> impl IntoResponse {
    match load_project_detail(&state.repo_root, &query.id) {
        Ok(detail) => ok_json(json!({ "detail": detail })),
        Err(e) => err_json(e),
    }
}

async fn get_project_browse(
    Query(query): Query<BrowseQuery>,
) -> impl IntoResponse {
    match browse_project_dirs(&query.path) {
        Ok((current_path, parent_path, entries)) => ok_json(json!({
            "currentPath": current_path,
            "parentPath": parent_path,
            "entries": entries
        })),
        Err(e) => err_json(e),
    }
}

async fn post_monorepo_sync(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match sync_monorepo_projects(&state.repo_root) {
        Ok((root, domains, packages, created, updated)) => ok_json(json!({
            "root": root,
            "domains": domains,
            "packages": packages,
            "created": created,
            "updated": updated
        })),
        Err(e) => err_json(e),
    }
}

async fn post_project_select(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProjectSelectRequest>,
) -> impl IntoResponse {
    match update_project_selected(&state.repo_root, &body.id) {
        Ok(project) => match load_project_detail(&state.repo_root, &project.id) {
            Ok(detail) => ok_json(json!({ "project": project, "detail": detail })),
            Err(e) => err_json(e),
        },
        Err(e) => err_json(e),
    }
}

async fn post_project_delete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProjectDeleteRequest>,
) -> impl IntoResponse {
    match delete_project(&state.repo_root, &body.id) {
        Ok(_) => ok_json(json!({ "ok": true })),
        Err(e) => err_json(e),
    }
}

async fn post_project_info(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProjectInfoRequest>,
) -> impl IntoResponse {
    match save_project_info(
        &state.repo_root,
        &body.id,
        &body.name,
        &body.description,
        &body.spec,
        &body.goal,
    ) {
        Ok(detail) => ok_json(json!({ "detail": detail })),
        Err(e) => err_json(e),
    }
}

async fn post_project_lists(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProjectListsRequest>,
) -> impl IntoResponse {
    match save_project_lists(
        &state.repo_root,
        &body.id,
        body.rules,
        body.constraints,
        body.features,
    ) {
        Ok(detail) => ok_json(json!({ "detail": detail })),
        Err(e) => err_json(e),
    }
}

async fn post_project_memo(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProjectMemoRequest>,
) -> impl IntoResponse {
    match save_project_memo(&state.repo_root, &body.id, &body.memo) {
        Ok(detail) => ok_json(json!({ "detail": detail })),
        Err(e) => err_json(e),
    }
}

async fn post_run(State(state): State<Arc<AppState>>, Json(body): Json<RunRequest>) -> impl IntoResponse {
    match run_orc_action(&state.repo_root, &body.id, &body.action, &body.payload).await {
        Ok(output) => ok_json(json!({ "output": output })),
        Err(e) => err_json(e),
    }
}

async fn get_tui_map() -> impl IntoResponse {
    ok_json(json!({
        "features": [
            "Project CRUD (create/update/delete/select)",
            "Detail fields (name/description/spec/goal)",
            "Rules/Constraints/Features list editing",
            "Plan/Drafts panels (planned/generated)",
            "create_code_draft, add_code_draft, impl_code_draft",
            "check_code_draft -a, check_draft"
        ]
    }))
}

fn ok_json(body: serde_json::Value) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(body))
}

fn err_json(message: String) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

fn registry_path(repo_root: &Path) -> PathBuf {
    repo_root.join("configs").join("project.yaml")
}

fn project_meta_dir(project_path: &Path) -> PathBuf {
    project_path.join(".project")
}

fn project_md_path(project_path: &Path) -> PathBuf {
    project_meta_dir(project_path).join("project.md")
}

fn drafts_list_path(project_path: &Path) -> PathBuf {
    project_meta_dir(project_path).join("drafts_list.yaml")
}

fn drafts_yaml_path(project_path: &Path) -> PathBuf {
    project_meta_dir(project_path).join("drafts.yaml")
}

fn memo_path(project_path: &Path) -> PathBuf {
    project_meta_dir(project_path).join("memo.md")
}

fn now_unix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .to_string()
}

fn random_id() -> String {
    const ALNUM: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789";
    let mut out = String::new();
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    for _ in 0..4 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (seed as usize) % ALNUM.len();
        out.push(ALNUM[idx] as char);
    }
    out
}

fn load_registry(repo_root: &Path) -> Result<ProjectRegistry, String> {
    let path = registry_path(repo_root);
    if !path.exists() {
        return Ok(ProjectRegistry::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    serde_yaml::from_str::<ProjectRegistry>(&raw).map_err(|e| format!("invalid yaml {}: {}", path.display(), e))
}

fn save_registry(repo_root: &Path, registry: &ProjectRegistry) -> Result<(), String> {
    let path = registry_path(repo_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let raw = serde_yaml::to_string(registry).map_err(|e| format!("yaml encode error: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn browse_root() -> PathBuf {
    std::env::var("ORC_BROWSE_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/tree"))
}

fn browse_project_dirs(input_path: &str) -> Result<(String, Option<String>, Vec<BrowseEntry>), String> {
    let root = browse_root()
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("/home/tree"));
    let requested = if input_path.trim().is_empty() {
        root.clone()
    } else {
        PathBuf::from(input_path.trim())
    };
    let candidate = requested
        .canonicalize()
        .unwrap_or_else(|_| requested.clone());
    let current = if candidate.starts_with(&root) {
        candidate
    } else {
        root.clone()
    };
    if !current.exists() || !current.is_dir() {
        return Err(format!("directory not found: {}", current.display()));
    }

    let parent = current.parent().and_then(|p| {
        if p.starts_with(&root) && p != current {
            Some(p.display().to_string())
        } else {
            None
        }
    });

    let mut entries = Vec::new();
    for entry in fs::read_dir(&current).map_err(|e| format!("failed to read {}: {}", current.display(), e))? {
        let entry = entry.map_err(|e| format!("failed to read entry: {}", e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(BrowseEntry {
            name,
            path: path.display().to_string(),
            has_project_meta: path.join(".project").exists(),
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((current.display().to_string(), parent, entries))
}

fn monorepo_root_path() -> PathBuf {
    if let Ok(v) = std::env::var("ORC_MONOREPO_ROOT") {
        return PathBuf::from(v);
    }
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home).join("home");
    }
    PathBuf::from("/home/tree/home")
}

fn list_immediate_dirs(base: &Path) -> Vec<String> {
    if !base.exists() || !base.is_dir() {
        return vec![];
    }
    let mut out = vec![];
    if let Ok(read_dir) = fs::read_dir(base) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            out.push(name);
        }
    }
    out.sort();
    out
}

fn collect_monorepo_packages(root: &Path) -> Vec<(String, PathBuf, String)> {
    let buckets = vec![
        ("app".to_string(), vec!["apps", "app"], true),
        (
            "feature".to_string(),
            vec!["packages/features", "features", "feature"],
            false,
        ),
        ("template".to_string(), vec!["template", "templates"], true),
    ];
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out = vec![];
    for (kind, dirs, deep_scan) in buckets {
        for rel in dirs {
            let parent = root.join(rel);
            for child in list_immediate_dirs(&parent) {
                let first = parent.join(&child);
                if deep_scan {
                    let nested = list_immediate_dirs(&first);
                    let mut added_nested = false;
                    for grandchild in nested {
                        let second = first.join(&grandchild);
                        if !seen.insert(second.clone()) {
                            continue;
                        }
                        out.push((kind.clone(), second, format!("{}/{}", child, grandchild)));
                        added_nested = true;
                    }
                    if !added_nested {
                        let fallback = if first.join("next.config.js").exists() || first.join("next.config.ts").exists() {
                            "next".to_string()
                        } else if first.join("astro.config.mjs").exists() || first.join("astro.config.ts").exists() {
                            "astro".to_string()
                        } else if first.join("app.json").exists() {
                            "expo".to_string()
                        } else {
                            "app".to_string()
                        };
                        if seen.insert(first.clone()) {
                            out.push((kind.clone(), first, format!("{}/{}", child, fallback)));
                        }
                    }
                    continue;
                }
                if !seen.insert(first.clone()) {
                    continue;
                }
                out.push((kind.clone(), first, child));
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

fn collect_monorepo_domains(root: &Path) -> Vec<String> {
    list_immediate_dirs(&root.join("packages").join("domains"))
}

fn path_is_inside(base: &Path, child: &Path) -> bool {
    if let (Ok(base), Ok(child)) = (base.canonicalize(), child.canonicalize()) {
        return child.starts_with(base);
    }
    false
}

fn is_monorepo_managed_path(project_path: &Path, root: &Path) -> bool {
    let monitored = [
        root.join("apps"),
        root.join("app"),
        root.join("packages").join("features"),
        root.join("features"),
        root.join("feature"),
        root.join("template"),
        root.join("templates"),
    ];
    monitored.iter().any(|p| path_is_inside(p, project_path))
}

fn monorepo_domain_details(root: &Path) -> Vec<DomainDetail> {
    collect_monorepo_domains(root)
        .into_iter()
        .map(|name| DomainDetail {
            name,
            description: String::new(),
            features: vec![],
        })
        .collect()
}

fn sync_monorepo_projects(
    repo_root: &Path,
) -> Result<(String, Vec<String>, Vec<MonorepoPackageEntry>, usize, usize), String> {
    let root = monorepo_root_path();
    let domains = collect_monorepo_domains(&root);
    let package_rows = collect_monorepo_packages(&root);
    let mut registry = load_registry(repo_root)?;
    let now = now_unix();
    registry
        .projects
        .retain(|p| !(p.project_type == ProjectType::Code && is_monorepo_managed_path(Path::new(&p.path), &root)));
    let mut created = 0usize;
    let mut updated = 0usize;
    for (kind, pkg_path, name) in &package_rows {
        let path_string = pkg_path.display().to_string();
        if let Some(project) = registry.projects.iter_mut().find(|p| p.path == path_string) {
            let next_description = format!("monorepo {} package", kind);
            if project.name != *name
                || project.description != next_description
                || project.project_type != ProjectType::Mono
            {
                project.name = name.clone();
                project.description = next_description;
                project.project_type = ProjectType::Mono;
                project.updated_at = now.clone();
                updated += 1;
            }
            ensure_project_files(project)?;
            continue;
        }
        let record = ProjectRecord {
            id: random_id(),
            name: name.clone(),
            path: path_string,
            description: format!("monorepo {} package", kind),
            created_at: now.clone(),
            updated_at: now.clone(),
            selected: false,
            project_type: ProjectType::Mono,
            state: None,
        };
        ensure_project_files(&record)?;
        registry.projects.push(record);
        created += 1;
    }
    save_registry(repo_root, &registry)?;
    let listed = list_projects(repo_root)?;
    let mut packages = vec![];
    for (kind, pkg_path, _) in &package_rows {
        let path_string = pkg_path.display().to_string();
        if let Some(project) = listed.iter().find(|p| p.path == path_string) {
            packages.push(MonorepoPackageEntry {
                id: project.id.clone(),
                name: project.name.clone(),
                path: project.path.clone(),
                kind: kind.clone(),
            });
        }
    }
    Ok((
        root.display().to_string(),
        domains,
        packages,
        created,
        updated,
    ))
}

fn ensure_project_files(project: &ProjectRecord) -> Result<(), String> {
    let project_path = PathBuf::from(&project.path);
    fs::create_dir_all(&project_path)
        .map_err(|e| format!("failed to create {}: {}", project_path.display(), e))?;
    let meta = project_meta_dir(&project_path);
    fs::create_dir_all(&meta).map_err(|e| format!("failed to create {}: {}", meta.display(), e))?;
    let pmd = project_md_path(&project_path);
    if !pmd.exists() {
        let raw = format!(
            "# info\nname: {}\ndescription: {}\nspec: auto\ngoal: init\n\n# rules\n- \n\n# constraints\n- \n\n# features\n- \n",
            project.name, project.description
        );
        fs::write(&pmd, raw).map_err(|e| format!("failed to write {}: {}", pmd.display(), e))?;
    }
    let dlist = drafts_list_path(&project_path);
    if !dlist.exists() {
        let raw = serde_yaml::to_string(&DraftsListDoc::default()).map_err(|e| format!("yaml encode error: {}", e))?;
        fs::write(&dlist, raw).map_err(|e| format!("failed to write {}: {}", dlist.display(), e))?;
    }
    let memo = memo_path(&project_path);
    if !memo.exists() {
        fs::write(&memo, "").map_err(|e| format!("failed to write {}: {}", memo.display(), e))?;
    }
    Ok(())
}

fn list_projects(repo_root: &Path) -> Result<Vec<ProjectRecord>, String> {
    let mut registry = load_registry(repo_root)?;
    for project in &mut registry.projects {
        project.state = Some(resolve_project_state(project));
    }
    Ok(registry.projects)
}

fn create_project(repo_root: &Path, input: CreateProjectRequest) -> Result<ProjectRecord, String> {
    let mut registry = load_registry(repo_root)?;
    let normalized_path = input.path.trim().to_string();
    if let Some(existing_idx) = registry.projects.iter().position(|p| p.path == normalized_path) {
        for p in &mut registry.projects {
            p.selected = false;
        }
        let existing = &mut registry.projects[existing_idx];
        existing.name = input.name.clone();
        existing.description = input.description.clone();
        existing.updated_at = now_unix();
        existing.selected = true;
        registry.recent_active_pane = existing.id.clone();
        let project = existing.clone();
        save_registry(repo_root, &registry)?;
        ensure_project_files(&project)?;
        return Ok(project);
    }
    if registry.projects.iter().any(|p| p.name == input.name) {
        return Err(format!("project already exists: {}", input.name));
    }
    for p in &mut registry.projects {
        p.selected = false;
    }
    let now = now_unix();
    let record = ProjectRecord {
        id: random_id(),
        name: input.name,
        path: normalized_path,
        description: input.description,
        created_at: now.clone(),
        updated_at: now,
        selected: true,
        project_type: input.project_type.unwrap_or_default(),
        state: None,
    };
    registry.recent_active_pane = record.id.clone();
    registry.projects.push(record.clone());
    save_registry(repo_root, &registry)?;
    ensure_project_files(&record)?;

    if !input.spec.trim().is_empty() {
        let _ = save_project_info(
            repo_root,
            &record.id,
            &record.name,
            &record.description,
            input.spec.trim(),
            "init",
        )?;
    }

    Ok(record)
}

fn load_project_from_path(
    repo_root: &Path,
    input: LoadProjectRequest,
) -> Result<(ProjectRecord, bool), String> {
    let project_path = input.path.trim();
    if project_path.is_empty() {
        return Err("project path is required".to_string());
    }
    let dir = PathBuf::from(project_path);
    if !dir.exists() {
        return Err(format!("path not found: {}", project_path));
    }
    if !dir.is_dir() {
        return Err(format!("path is not directory: {}", project_path));
    }
    let meta = project_meta_dir(&dir);
    let has_meta = meta.exists();
    if !has_meta && !input.create_if_missing {
        return Err("PROJECT_META_MISSING".to_string());
    }

    let base_name = dir
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("project")
        .to_string();
    let mut parsed_name = base_name.clone();
    let mut parsed_description = "loaded project".to_string();
    if has_meta && project_md_path(&dir).exists() {
        let raw = fs::read_to_string(project_md_path(&dir)).unwrap_or_default();
        let parsed = parse_project_md(&raw);
        if !parsed.name.trim().is_empty() {
            parsed_name = parsed.name;
        }
        if !parsed.description.trim().is_empty() {
            parsed_description = parsed.description;
        }
    }

    let mut registry = load_registry(repo_root)?;
    let now = now_unix();
    let path_string = dir.display().to_string();
    for p in &mut registry.projects {
        p.selected = false;
    }
    let project = if let Some(existing) = registry.projects.iter_mut().find(|p| p.path == path_string) {
        existing.name = parsed_name;
        existing.description = parsed_description;
        existing.updated_at = now.clone();
        existing.selected = true;
        existing.clone()
    } else {
        let record = ProjectRecord {
            id: random_id(),
            name: parsed_name,
            path: path_string,
            description: parsed_description,
            created_at: now.clone(),
            updated_at: now,
            selected: true,
            project_type: input.project_type.unwrap_or_default(),
            state: None,
        };
        registry.projects.push(record.clone());
        record
    };
    registry.recent_active_pane = project.id.clone();
    save_registry(repo_root, &registry)?;
    ensure_project_files(&project)?;
    Ok((project, !has_meta))
}

fn update_project_selected(repo_root: &Path, id: &str) -> Result<ProjectRecord, String> {
    let mut registry = load_registry(repo_root)?;
    if !registry.projects.iter().any(|p| p.id == id) {
        return Err(format!("project not found: {}", id));
    }
    for project in &mut registry.projects {
        project.selected = project.id == id;
    }
    registry.recent_active_pane = id.to_string();
    let found = registry
        .projects
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| format!("project not found: {}", id))?;
    save_registry(repo_root, &registry)?;
    Ok(found)
}

fn delete_project(repo_root: &Path, id: &str) -> Result<(), String> {
    let mut registry = load_registry(repo_root)?;
    let target = registry
        .projects
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| format!("project not found: {}", id))?;
    registry.projects.retain(|p| p.id != id);
    if registry.recent_active_pane == id {
        registry.recent_active_pane = registry
            .projects
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
    }
    if !registry.projects.is_empty() && !registry.projects.iter().any(|p| p.selected) {
        if let Some(first) = registry.projects.first_mut() {
            first.selected = true;
        }
    }
    save_registry(repo_root, &registry)?;
    let meta = project_meta_dir(Path::new(&target.path));
    if meta.exists() {
        fs::remove_dir_all(&meta).map_err(|e| format!("failed to remove {}: {}", meta.display(), e))?;
    }
    Ok(())
}

fn load_project_detail(repo_root: &Path, id: &str) -> Result<ProjectDetail, String> {
    let registry = load_registry(repo_root)?;
    let project = registry
        .projects
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| format!("project not found: {}", id))?;
    ensure_project_files(&project)?;
    let project_path = PathBuf::from(&project.path);
    let raw = fs::read_to_string(project_md_path(&project_path))
        .map_err(|e| format!("failed to read project.md: {}", e))?;
    let parsed = parse_project_md(&raw);
    let monorepo_root = monorepo_root_path();
    let domains = if is_monorepo_managed_path(&project_path, &monorepo_root) {
        monorepo_domain_details(&monorepo_root)
    } else {
        parsed.domains.clone()
    };
    let drafts = load_drafts_list(&project_path)?;
    let planned = drafts.planned.clone();
    let planned_display = planned
        .iter()
        .map(|key| {
            drafts
                .planned_items
                .iter()
                .find(|row| row.name == *key)
                .map(|row| row.value.clone())
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| key.clone())
        })
        .collect::<Vec<_>>();
    Ok(ProjectDetail {
        id: project.id.clone(),
        name: if parsed.name.is_empty() {
            project.name.clone()
        } else {
            parsed.name
        },
        description: if parsed.description.is_empty() {
            project.description.clone()
        } else {
            parsed.description
        },
        path: project.path.clone(),
        memo: fs::read_to_string(memo_path(&project_path)).unwrap_or_default(),
        project_type: project.project_type.clone(),
        spec: parsed.spec,
        goal: parsed.goal,
        rules: parsed.rules,
        constraints: parsed.constraints,
        features: parsed.features,
        domains,
        planned,
        planned_display,
        generated: collect_generated(&project_path),
        state: resolve_project_state(&project),
        has_drafts_yaml: drafts_yaml_path(&project_path).exists(),
    })
}

fn save_project_memo(repo_root: &Path, id: &str, memo: &str) -> Result<ProjectDetail, String> {
    let detail = load_project_detail(repo_root, id)?;
    let path = memo_path(Path::new(&detail.path));
    fs::write(&path, memo).map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
    load_project_detail(repo_root, id)
}

fn save_project_info(
    repo_root: &Path,
    id: &str,
    name: &str,
    description: &str,
    spec: &str,
    goal: &str,
) -> Result<ProjectDetail, String> {
    let mut registry = load_registry(repo_root)?;
    let project = registry
        .projects
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("project not found: {}", id))?;
    project.name = name.to_string();
    project.description = description.to_string();
    project.updated_at = now_unix();
    let project_path = PathBuf::from(&project.path);
    let current = load_project_detail(repo_root, id)?;
    write_project_md(
        &project_path,
        &ParsedProjectMd {
            name: name.to_string(),
            description: description.to_string(),
            spec: spec.to_string(),
            goal: goal.to_string(),
            rules: current.rules,
            constraints: current.constraints,
            features: current.features,
            domains: current.domains,
        },
    )?;
    save_registry(repo_root, &registry)?;
    load_project_detail(repo_root, id)
}

fn save_project_lists(
    repo_root: &Path,
    id: &str,
    rules: Vec<String>,
    constraints: Vec<String>,
    features: Vec<String>,
) -> Result<ProjectDetail, String> {
    let current = load_project_detail(repo_root, id)?;
    let project_path = PathBuf::from(&current.path);
    write_project_md(
        &project_path,
        &ParsedProjectMd {
            name: current.name.clone(),
            description: current.description.clone(),
            spec: current.spec.clone(),
            goal: current.goal.clone(),
            rules: rules.clone(),
            constraints: constraints.clone(),
            features: features.clone(),
            domains: current.domains.clone(),
        },
    )?;
    let mut drafts = load_drafts_list(&project_path)?;
    drafts.features = features;
    save_drafts_list(&project_path, &drafts)?;
    load_project_detail(repo_root, id)
}

async fn run_orc_action(repo_root: &Path, id: &str, action: &str, payload: &str) -> Result<String, String> {
    let detail = load_project_detail(repo_root, id)?;
    let previous = std::env::current_dir().map_err(|e| format!("failed to get cwd: {}", e))?;
    std::env::set_current_dir(repo_root).map_err(|e| format!("failed to enter repo root: {}", e))?;
    let output = match action {
        "create_draft" => crate::code::create_code_draft(),
        "add_draft" => {
            let args = if payload.trim().is_empty() {
                vec!["-a".to_string()]
            } else {
                vec!["-m".to_string(), payload.to_string()]
            };
            crate::code::add_code_draft(&args)
        }
        "impl_draft" => crate::code::impl_code_draft().await,
        "check_code" => crate::code::check_code_draft(true),
        "check_draft" => crate::code::check_draft(),
        _ => Err(format!("unsupported action: {}", action)),
    };
    let _ = std::env::set_current_dir(previous);
    output.map(|msg| format!("action={} project={} output={}", action, detail.name, msg))
}

fn load_drafts_list(project_path: &Path) -> Result<DraftsListDoc, String> {
    let path = drafts_list_path(project_path);
    if !path.exists() {
        return Ok(DraftsListDoc::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    serde_yaml::from_str::<DraftsListDoc>(&raw).map_err(|e| format!("invalid yaml {}: {}", path.display(), e))
}

fn save_drafts_list(project_path: &Path, doc: &DraftsListDoc) -> Result<(), String> {
    let path = drafts_list_path(project_path);
    let raw = serde_yaml::to_string(doc).map_err(|e| format!("yaml encode error: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

fn load_drafts_doc(project_path: &Path) -> DraftsDoc {
    let path = drafts_yaml_path(project_path);
    if !path.exists() {
        return DraftsDoc::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_yaml::from_str::<DraftsDoc>(&raw).ok())
        .unwrap_or_default()
}

fn resolve_project_state(project: &ProjectRecord) -> ProjectState {
    let project_path = Path::new(&project.path);
    if !project_md_path(project_path).exists() {
        return ProjectState::Init;
    }
    let drafts = load_drafts_doc(project_path);
    if !drafts.planned.is_empty() || !drafts.worked.is_empty() {
        return ProjectState::Work;
    }
    if !drafts.complete.is_empty() && drafts.planned.is_empty() && drafts.worked.is_empty() {
        return ProjectState::Wait;
    }
    if !is_bootstrap_completed(project_path) {
        return ProjectState::Init;
    }
    ProjectState::Basic
}

fn is_bootstrap_completed(project_path: &Path) -> bool {
    if !project_path.exists() {
        return false;
    }
    fs::read_dir(project_path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| {
            let name = entry.file_name();
            let value = name.to_string_lossy();
            value != ".project" && value != ".git" && value != ".jj"
        })
}

fn collect_generated(project_path: &Path) -> Vec<String> {
    let root = project_path.join(".project").join("feature");
    if !root.exists() {
        return vec![];
    }
    let mut out = vec![];
    if let Ok(iter) = fs::read_dir(&root) {
        for entry in iter.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let has_draft = dir.join("drafts.yaml").exists() || dir.join("tasks.yaml").exists();
            if has_draft {
                out.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    out.sort();
    out
}

fn parse_project_md(raw: &str) -> ParsedProjectMd {
    let mut out = ParsedProjectMd {
        name: String::new(),
        description: String::new(),
        spec: String::new(),
        goal: String::new(),
        rules: vec![],
        constraints: vec![],
        features: vec![],
        domains: vec![],
    };
    let mut section = "none";
    let mut in_domains = false;
    let mut active_domain_idx: Option<usize> = None;
    let mut domain_subsection = String::new();
    for line in raw.lines() {
        let t = line.trim();
        let lower = t.to_ascii_lowercase();
        if lower == "# rules" {
            section = "rules";
            continue;
        }
        if lower == "# constraints" {
            section = "constraints";
            continue;
        }
        if lower == "# features" {
            section = "features";
            continue;
        }
        if lower == "# domains" {
            section = "none";
            in_domains = true;
            continue;
        }
        if t.starts_with('#') {
            section = "none";
            if in_domains && t.starts_with("# ") && lower != "# domains" {
                in_domains = false;
                active_domain_idx = None;
                domain_subsection.clear();
            }
        }
        if in_domains && t.starts_with("## ") {
            let heading = t.trim_start_matches("## ").trim().replace('`', "");
            if !heading.is_empty() {
                let mut parts = heading.splitn(2, ['|', ':']);
                let name = parts.next().unwrap_or("").trim();
                let description = parts.next().unwrap_or("").trim();
                if !name.is_empty() && name.to_ascii_lowercase() != "name" {
                    out.domains.push(DomainDetail {
                        name: name.to_string(),
                        description: description.to_string(),
                        features: vec![],
                    });
                    active_domain_idx = Some(out.domains.len() - 1);
                    domain_subsection.clear();
                }
            }
            continue;
        }
        if in_domains {
            if t.starts_with("### ") {
                domain_subsection = t.trim_start_matches("### ").trim().to_ascii_lowercase();
                continue;
            }
            if let Some(item) = t.strip_prefix("- ").map(str::trim).filter(|v| !v.is_empty()) {
                if let Some(idx) = active_domain_idx {
                    if matches!(domain_subsection.as_str(), "action" | "feature" | "features") {
                        if !out.domains[idx].features.iter().any(|v| v == item) {
                            out.domains[idx].features.push(item.to_string());
                        }
                    } else if (domain_subsection == "rules" || domain_subsection == "description")
                        && out.domains[idx].description.is_empty()
                    {
                        out.domains[idx].description = item.to_string();
                    }
                }
            }
            continue;
        }
        if section == "rules" && t.starts_with("- ") {
            out.rules.push(t.trim_start_matches("- ").trim().to_string());
            continue;
        }
        if section == "constraints" && t.starts_with("- ") {
            out.constraints
                .push(t.trim_start_matches("- ").trim().to_string());
            continue;
        }
        if section == "features" && t.starts_with("- ") {
            out.features.push(t.trim_start_matches("- ").trim().to_string());
            continue;
        }
        if let Some((key, value)) = t.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if key == "name" {
                out.name = value;
            } else if key == "description" {
                out.description = value;
            } else if key == "spec" {
                out.spec = value;
            } else if key == "goal" {
                out.goal = value;
            }
        }
    }
    out.rules.retain(|v| !v.is_empty());
    out.constraints.retain(|v| !v.is_empty());
    out.features.retain(|v| !v.is_empty());
    out
}

fn write_project_md(project_path: &Path, doc: &ParsedProjectMd) -> Result<(), String> {
    let mut lines = vec![
        "# info".to_string(),
        format!("name: {}", doc.name),
        format!("description: {}", doc.description),
        format!("spec: {}", doc.spec),
        format!("goal: {}", doc.goal),
        String::new(),
        "# rules".to_string(),
    ];
    if doc.rules.is_empty() {
        lines.push("- ".to_string());
    } else {
        for row in &doc.rules {
            lines.push(format!("- {}", row));
        }
    }
    lines.push(String::new());
    lines.push("# constraints".to_string());
    if doc.constraints.is_empty() {
        lines.push("- ".to_string());
    } else {
        for row in &doc.constraints {
            lines.push(format!("- {}", row));
        }
    }
    lines.push(String::new());
    lines.push("# features".to_string());
    if doc.features.is_empty() {
        lines.push("- ".to_string());
    } else {
        for row in &doc.features {
            lines.push(format!("- {}", row));
        }
    }
    if !doc.domains.is_empty() {
        lines.push(String::new());
        lines.push("# domains".to_string());
        for domain in &doc.domains {
            lines.push(format!("## {}", domain.name));
            if !domain.description.is_empty() {
                lines.push("### description".to_string());
                lines.push(format!("- {}", domain.description));
            }
            if !domain.features.is_empty() {
                lines.push("### feature".to_string());
                for feature in &domain.features {
                    lines.push(format!("- {}", feature));
                }
            }
            lines.push(String::new());
        }
    } else {
        lines.push(String::new());
    }
    fs::write(project_md_path(project_path), lines.join("\n"))
        .map_err(|e| format!("failed to write project.md: {}", e))
}
