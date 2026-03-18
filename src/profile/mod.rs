use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub(crate) trait TemplateProvider: Send + Sync {
    fn project_template_path(&self) -> PathBuf;
    fn job_template_path(&self) -> PathBuf;
    fn drafts_template_path(&self) -> PathBuf;
}

pub(crate) trait PromptProvider: Send + Sync {
    fn build_domains_prompt_path(&self) -> PathBuf;
    fn init_project_prompt_path(&self) -> PathBuf;
    fn check_code_prompt_path(&self) -> PathBuf;
    fn build_parallel_prompt_path(&self) -> PathBuf;
}

pub(crate) trait ParallelRunner: Send + Sync {
    fn run_parallel_build<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

pub(crate) trait ProjectService: Send + Sync {
    fn init_project(&self, args: &[String]) -> Result<String, String>;
    fn build_domains(&self) -> Result<String, String>;
    fn init_job(&self) -> Result<String, String>;
    fn auto_message(&self, message: &str) -> Result<String, String>;
}

pub(crate) trait DraftService: Send + Sync {
    fn add_drafts(&self) -> Result<String, String>;
    fn run_parallel<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

pub(crate) trait FeedbackService: Send + Sync {
    fn check(&self) -> Result<String, String>;
}

pub(crate) trait Profile: Send + Sync {
    fn name(&self) -> &str;
    fn templates(&self) -> &dyn TemplateProvider;
    fn prompts(&self) -> &dyn PromptProvider;
    fn project_service(&self) -> &dyn ProjectService;
    fn draft_service(&self) -> &dyn DraftService;
    fn feedback_service(&self) -> &dyn FeedbackService;
}

struct CodeTemplateProvider;
struct CodePromptProvider;
struct CodeProjectService;
struct CodeDraftService;
struct CodeFeedbackService;

pub(crate) struct CodeProfile {
    templates: CodeTemplateProvider,
    prompts: CodePromptProvider,
    project: CodeProjectService,
    draft: CodeDraftService,
    feedback: CodeFeedbackService,
}

impl CodeProfile {
    pub(crate) fn new() -> Self {
        Self {
            templates: CodeTemplateProvider,
            prompts: CodePromptProvider,
            project: CodeProjectService,
            draft: CodeDraftService,
            feedback: CodeFeedbackService,
        }
    }
}

impl TemplateProvider for CodeTemplateProvider {
    fn project_template_path(&self) -> PathBuf {
        crate::source_root().join("assets").join("templates").join("project.md")
    }
    fn job_template_path(&self) -> PathBuf {
        crate::source_root().join("assets").join("templates").join("job.md")
    }
    fn drafts_template_path(&self) -> PathBuf {
        crate::source_root().join("assets").join("templates").join("drafts.yaml")
    }
}

impl PromptProvider for CodePromptProvider {
    fn build_domains_prompt_path(&self) -> PathBuf {
        crate::source_root().join("assets").join("prompts").join("build_domains.md")
    }
    fn init_project_prompt_path(&self) -> PathBuf {
        crate::source_root().join("assets").join("prompts").join("init_project.md")
    }
    fn check_code_prompt_path(&self) -> PathBuf {
        crate::source_root().join("assets").join("prompts").join("check_code.md")
    }
    fn build_parallel_prompt_path(&self) -> PathBuf {
        crate::source_root().join("assets").join("prompts").join("build_parallel.md")
    }
}

impl ProjectService for CodeProjectService {
    fn init_project(&self, args: &[String]) -> Result<String, String> {
        crate::code::init_orc_project(args)
    }
    fn build_domains(&self) -> Result<String, String> {
        crate::code::build_orc_domains()
    }
    fn init_job(&self) -> Result<String, String> {
        crate::code::init_orc_job()
    }
    fn auto_message(&self, message: &str) -> Result<String, String> {
        // ... (can be mapped to a combined flow)
        Ok("auto_message not fully refactored yet".to_string())
    }
}

impl DraftService for CodeDraftService {
    fn add_drafts(&self) -> Result<String, String> {
        crate::code::add_orc_drafts()
    }
    fn run_parallel<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { crate::code::impl_orc_code().await })
    }
}

impl FeedbackService for CodeFeedbackService {
    fn check(&self) -> Result<String, String> {
        crate::code::check_orc_code()
    }
}

impl Profile for CodeProfile {
    fn name(&self) -> &str { "code" }
    fn templates(&self) -> &dyn TemplateProvider { &self.templates }
    fn prompts(&self) -> &dyn PromptProvider { &self.prompts }
    fn project_service(&self) -> &dyn ProjectService { &self.project }
    fn draft_service(&self) -> &dyn DraftService { &self.draft }
    fn feedback_service(&self) -> &dyn FeedbackService { &self.feedback }
}

pub(crate) fn is_known_profile_name(name: &str) -> bool {
    matches!(name, "code")
}

pub(crate) fn resolve_profile(name: &str) -> Result<Box<dyn Profile>, String> {
    match name {
        "code" => Ok(Box::new(CodeProfile::new())),
        _ => Err(format!("unknown profile: {}", name)),
    }
}
