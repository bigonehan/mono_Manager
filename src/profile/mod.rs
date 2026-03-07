use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub(crate) trait TemplateProvider: Send + Sync {
    fn project_template_path(&self) -> PathBuf;
    fn plan_template_path(&self) -> PathBuf;
    fn drafts_template_path(&self) -> PathBuf;
}

pub(crate) trait PromptProvider: Send + Sync {
    fn add_project_detail_prompt_path(&self) -> PathBuf;
    fn infer_plan_prompt_path(&self) -> PathBuf;
    fn infer_draft_prompt_path(&self) -> PathBuf;
    fn impl_draft_prompt_path(&self) -> PathBuf;
}

pub(crate) trait ParallelRunner: Send + Sync {
    fn run_parallel_build<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

pub(crate) trait ProjectService: Send + Sync {
    fn create(&self, args: &[String]) -> Result<String, String>;
    fn delete(&self, _args: &[String]) -> Result<String, String>;
    fn update(&self, _args: &[String]) -> Result<String, String>;
    fn detail(&self) -> Result<String, String>;
    fn add_domain(&self) -> Result<String, String>;
    fn auto_message(&self, message: &str) -> Result<String, String>;
    fn auto_from_input(&self) -> Result<String, String>;
}

pub(crate) trait PlanService: Send + Sync {
    fn create(&self, args: &[String]) -> Result<String, String>;
    fn delete(&self, _args: &[String]) -> Result<String, String>;
    fn update(&self, args: &[String]) -> Result<String, String>;
    fn add_feature(&self, args: &[String]) -> Result<String, String>;
    fn create_draft(&self) -> Result<String, String>;
    fn create_input(&self) -> Result<String, String>;
}

pub(crate) trait DraftService: Send + Sync {
    fn add(&self, args: &[String]) -> Result<String, String>;
    fn add_item(&self, args: &[String]) -> Result<String, String>;
    fn move_item_to_drafts_yaml(&self, args: &[String]) -> Result<String, String>;
    fn run_parallel<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

pub(crate) trait FeedbackService: Send + Sync {
    fn check(&self, auto_yes: bool) -> Result<String, String>;
    fn decide_policy(&self) -> Result<String, String>;
    fn check_draft(&self) -> Result<String, String>;
}

pub(crate) trait Profile: Send + Sync {
    fn name(&self) -> &str;
    fn templates(&self) -> &dyn TemplateProvider;
    fn prompts(&self) -> &dyn PromptProvider;
    fn project_service(&self) -> &dyn ProjectService;
    fn plan_service(&self) -> &dyn PlanService;
    fn draft_service(&self) -> &dyn DraftService;
    fn feedback_service(&self) -> &dyn FeedbackService;
    fn parallel_runner(&self) -> &dyn ParallelRunner;
}

struct CodeTemplateProvider;
struct CodePromptProvider;
struct CodeParallelRunner;
struct CodeProjectService;
struct CodePlanService;
struct CodeDraftService;
struct CodeFeedbackService;
struct StoryTemplateProvider;
struct StoryPromptProvider;
struct StoryParallelRunner;
struct StoryProjectService;
struct StoryPlanService;
struct StoryDraftService;
struct StoryFeedbackService;

pub(crate) struct CodeProfile {
    templates: CodeTemplateProvider,
    prompts: CodePromptProvider,
    project: CodeProjectService,
    plan: CodePlanService,
    draft: CodeDraftService,
    feedback: CodeFeedbackService,
    parallel: CodeParallelRunner,
}

pub(crate) struct StoryProfile {
    templates: StoryTemplateProvider,
    prompts: StoryPromptProvider,
    project: StoryProjectService,
    plan: StoryPlanService,
    draft: StoryDraftService,
    feedback: StoryFeedbackService,
    parallel: StoryParallelRunner,
}

impl CodeProfile {
    pub(crate) fn new() -> Self {
        Self {
            templates: CodeTemplateProvider,
            prompts: CodePromptProvider,
            project: CodeProjectService,
            plan: CodePlanService,
            draft: CodeDraftService,
            feedback: CodeFeedbackService,
            parallel: CodeParallelRunner,
        }
    }
}

impl StoryProfile {
    pub(crate) fn new() -> Self {
        Self {
            templates: StoryTemplateProvider,
            prompts: StoryPromptProvider,
            project: StoryProjectService,
            plan: StoryPlanService,
            draft: StoryDraftService,
            feedback: StoryFeedbackService,
            parallel: StoryParallelRunner,
        }
    }
}

impl TemplateProvider for CodeTemplateProvider {
    fn project_template_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("templates")
            .join("project.md")
    }

    fn plan_template_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("templates")
            .join("plan.yaml")
    }

    fn drafts_template_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("templates")
            .join("drafts.yaml")
    }
}

impl PromptProvider for CodePromptProvider {
    fn add_project_detail_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("add_detail_project_code.txt")
    }

    fn infer_plan_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("infer_plan_yaml.txt")
    }

    fn infer_draft_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("infer_draft_item.txt")
    }

    fn impl_draft_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("impl_code_draft.txt")
    }
}

impl TemplateProvider for StoryTemplateProvider {
    fn project_template_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("story")
            .join("templates")
            .join("project.md")
    }

    fn plan_template_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("story")
            .join("templates")
            .join("plan.yaml")
    }

    fn drafts_template_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("story")
            .join("templates")
            .join("drafts.yaml")
    }
}

impl PromptProvider for StoryPromptProvider {
    fn add_project_detail_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("add_detail_project_code.txt")
    }

    fn infer_plan_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("infer_plan_yaml.txt")
    }

    fn infer_draft_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("infer_draft_item.txt")
    }

    fn impl_draft_prompt_path(&self) -> PathBuf {
        crate::source_root()
            .join("assets")
            .join("code")
            .join("prompts")
            .join("impl_code_draft.txt")
    }
}

impl ParallelRunner for CodeParallelRunner {
    fn run_parallel_build<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { crate::parallel::run_parallel_build_code().await })
    }
}

impl ParallelRunner for StoryParallelRunner {
    fn run_parallel_build<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { crate::parallel::run_parallel_build_code().await })
    }
}

impl ProjectService for CodeProjectService {
    fn create(&self, args: &[String]) -> Result<String, String> {
        crate::code::init_code_project(args)
    }

    fn delete(&self, _args: &[String]) -> Result<String, String> {
        Err("project delete is not implemented for profile=code".to_string())
    }

    fn update(&self, _args: &[String]) -> Result<String, String> {
        Err("project update is not implemented for profile=code".to_string())
    }

    fn detail(&self) -> Result<String, String> {
        crate::code::detail_code_project()
    }

    fn add_domain(&self) -> Result<String, String> {
        crate::code::create_code_domain()
    }

    fn auto_message(&self, message: &str) -> Result<String, String> {
        crate::code::auto_code_message(message)
    }

    fn auto_from_input(&self) -> Result<String, String> {
        crate::code::auto_code_from_input_file()
    }
}

impl ProjectService for StoryProjectService {
    fn create(&self, args: &[String]) -> Result<String, String> {
        crate::story::init_story_project(args)
    }

    fn delete(&self, _args: &[String]) -> Result<String, String> {
        Err("project delete is not implemented for profile=story".to_string())
    }

    fn update(&self, _args: &[String]) -> Result<String, String> {
        Err("project update is not implemented for profile=story".to_string())
    }

    fn detail(&self) -> Result<String, String> {
        Ok("detail_story_project is not implemented yet".to_string())
    }

    fn add_domain(&self) -> Result<String, String> {
        Ok("create_story_domain is not implemented yet".to_string())
    }

    fn auto_message(&self, message: &str) -> Result<String, String> {
        crate::story::auto_story_message(message)
    }

    fn auto_from_input(&self) -> Result<String, String> {
        crate::story::auto_story_from_input_file()
    }
}

impl PlanService for CodePlanService {
    fn create(&self, args: &[String]) -> Result<String, String> {
        crate::code::init_code_plan(args)
    }

    fn delete(&self, _args: &[String]) -> Result<String, String> {
        Err("plan delete is not implemented for profile=code".to_string())
    }

    fn update(&self, args: &[String]) -> Result<String, String> {
        crate::code::add_code_plan(args)
    }

    fn add_feature(&self, args: &[String]) -> Result<String, String> {
        crate::code::add_code_plan(args)
    }

    fn create_draft(&self) -> Result<String, String> {
        crate::code::create_code_draft()
    }

    fn create_input(&self) -> Result<String, String> {
        crate::code::create_input_md()
    }
}

impl PlanService for StoryPlanService {
    fn create(&self, args: &[String]) -> Result<String, String> {
        crate::story::init_story_plan(args)
    }

    fn delete(&self, _args: &[String]) -> Result<String, String> {
        Err("plan delete is not implemented for profile=story".to_string())
    }

    fn update(&self, args: &[String]) -> Result<String, String> {
        crate::story::add_story_plan(args)
    }

    fn add_feature(&self, args: &[String]) -> Result<String, String> {
        crate::story::add_story_plan(args)
    }

    fn create_draft(&self) -> Result<String, String> {
        crate::story::create_story_draft()
    }

    fn create_input(&self) -> Result<String, String> {
        crate::story::create_story_input_md()
    }
}

impl DraftService for CodeDraftService {
    fn add(&self, args: &[String]) -> Result<String, String> {
        crate::code::add_code_draft(args)
    }

    fn add_item(&self, args: &[String]) -> Result<String, String> {
        crate::code::add_code_draft_item(args)
    }

    fn move_item_to_drafts_yaml(&self, args: &[String]) -> Result<String, String> {
        crate::code::add_code_draft_item(args)
    }

    fn run_parallel<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { crate::code::impl_code_draft().await })
    }
}

impl DraftService for StoryDraftService {
    fn add(&self, args: &[String]) -> Result<String, String> {
        crate::story::add_story_draft(args)
    }

    fn add_item(&self, args: &[String]) -> Result<String, String> {
        crate::code::add_code_draft_item(args)
    }

    fn move_item_to_drafts_yaml(&self, args: &[String]) -> Result<String, String> {
        crate::code::add_code_draft_item(args)
    }

    fn run_parallel<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { crate::story::impl_story_draft().await })
    }
}

impl FeedbackService for CodeFeedbackService {
    fn check(&self, auto_yes: bool) -> Result<String, String> {
        crate::code::check_code_draft(auto_yes)
    }

    fn decide_policy(&self) -> Result<String, String> {
        crate::code::check_task()
    }

    fn check_draft(&self) -> Result<String, String> {
        crate::code::check_draft()
    }
}

impl FeedbackService for StoryFeedbackService {
    fn check(&self, auto_yes: bool) -> Result<String, String> {
        crate::story::check_story_draft(auto_yes)
    }

    fn decide_policy(&self) -> Result<String, String> {
        crate::story::check_story_task()
    }

    fn check_draft(&self) -> Result<String, String> {
        crate::story::check_story_only()
    }
}

impl Profile for CodeProfile {
    fn name(&self) -> &str {
        "code"
    }

    fn templates(&self) -> &dyn TemplateProvider {
        &self.templates
    }

    fn prompts(&self) -> &dyn PromptProvider {
        &self.prompts
    }

    fn project_service(&self) -> &dyn ProjectService {
        &self.project
    }

    fn plan_service(&self) -> &dyn PlanService {
        &self.plan
    }

    fn draft_service(&self) -> &dyn DraftService {
        &self.draft
    }

    fn feedback_service(&self) -> &dyn FeedbackService {
        &self.feedback
    }

    fn parallel_runner(&self) -> &dyn ParallelRunner {
        &self.parallel
    }
}

impl Profile for StoryProfile {
    fn name(&self) -> &str {
        "story"
    }

    fn templates(&self) -> &dyn TemplateProvider {
        &self.templates
    }

    fn prompts(&self) -> &dyn PromptProvider {
        &self.prompts
    }

    fn project_service(&self) -> &dyn ProjectService {
        &self.project
    }

    fn plan_service(&self) -> &dyn PlanService {
        &self.plan
    }

    fn draft_service(&self) -> &dyn DraftService {
        &self.draft
    }

    fn feedback_service(&self) -> &dyn FeedbackService {
        &self.feedback
    }

    fn parallel_runner(&self) -> &dyn ParallelRunner {
        &self.parallel
    }
}

pub(crate) fn is_known_profile_name(name: &str) -> bool {
    matches!(name, "code" | "story" | "write" | "movie")
}

pub(crate) fn resolve_profile(name: &str) -> Result<Box<dyn Profile>, String> {
    match name {
        "code" => Ok(Box::new(CodeProfile::new())),
        "story" => Ok(Box::new(StoryProfile::new())),
        "write" | "movie" => Err(format!(
            "profile `{}` is not implemented yet. use profile `code`",
            name
        )),
        _ => Err(format!("unknown profile: {}", name)),
    }
}
