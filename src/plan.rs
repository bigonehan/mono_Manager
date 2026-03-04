pub(crate) fn add_plan(request_input: Option<String>) -> Result<String, String> {
    let mut args = Vec::new();
    if let Some(v) = request_input {
        if !v.trim().is_empty() {
            args.push("-m".to_string());
            args.push(v);
        }
    }
    crate::code::add_code_plan(&args)
}
