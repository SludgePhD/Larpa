use crate::Context;

pub fn default_formatter(context: &Context) -> String {
    format!(
        "{} {}",
        context.command_desc().canonical_name(),
        context.command_desc().version().unwrap_or("<no version>")
    )
}
