pub(crate) fn add_plan(request_input: Option<String>) -> Result<String, String> {
    // In new flow, this maps to add_orc_drafts or similar.
    // For now, keeping it simple to satisfy calls.
    crate::code::add_orc_drafts()
}
