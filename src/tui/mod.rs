use crate::ui;

pub(crate) struct TuiRuntime;

impl TuiRuntime {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn run_ui_entry(&self) -> Result<String, String> {
        let registry_path = crate::registry_path();
        let mut registry = crate::load_registry(&registry_path)?;
        let normalized = crate::normalize_registry(&mut registry);
        crate::save_registry(&registry_path, &registry)?;
        let result = ui::run_ui(&mut registry.projects, &mut registry.recent_active_pane)?;
        if normalized {
            registry.recent_active_pane = registry
                .recent_active_pane
                .as_ref()
                .and_then(|id| {
                    registry
                        .projects
                        .iter()
                        .find(|p| &p.id == id)
                        .map(|p| p.id.clone())
                });
        }
        if result.changed || normalized {
            crate::save_registry(&registry_path, &registry)?;
        }
        Ok(result.message)
    }
}

pub(crate) fn open_ui() -> Result<String, String> {
    TuiRuntime::new().run_ui_entry()
}
