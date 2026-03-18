use crate::ui;

pub(crate) struct TuiRuntime;

impl TuiRuntime {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn run_ui_entry(&self) -> Result<String, String> {
        let mut registry = crate::load_registry()?;
        let result = ui::run_ui(&mut registry.projects, &mut registry.recent_active_pane)?;
        if result.changed {
            crate::save_registry(&registry)?;
        }
        Ok(result.message)
    }
}
