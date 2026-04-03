pub struct HelpInfo {
    pub flags: Vec<String>,
    pub subcommands: Vec<String>,
}

pub fn parse_help(help_text: &str) -> HelpInfo {
    let lower = help_text.to_lowercase();
    let mut flags = Vec::new();
    let mut subcommands = Vec::new();

    // Detect flags
    for flag in &[
        "--json", "--quiet", "-q", "--yes", "--force", "--limit", "--offset", "--cursor", "--page",
        "--fields", "--format", "--output", "-o",
    ] {
        if lower.contains(flag) {
            flags.push(flag.to_string());
        }
    }

    // Detect subcommands
    for sub in &[
        "schema",
        "init",
        "config init",
        "completions",
        "list",
        "ls",
        "status",
        "info",
        "get",
        "show",
    ] {
        if lower.contains(sub) {
            subcommands.push(sub.to_string());
        }
    }

    HelpInfo { flags, subcommands }
}

impl HelpInfo {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    pub fn has_subcommand(&self, sub: &str) -> bool {
        self.subcommands.iter().any(|s| s == sub)
    }

    pub fn first_list_subcommand(&self) -> Option<&str> {
        ["list", "ls", "status", "info", "get", "show"]
            .iter()
            .find(|&&sub| self.has_subcommand(sub))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_json_flag() {
        let info = parse_help("Usage: mytool [OPTIONS]\n\n  --json  Output JSON");
        assert!(info.has_flag("--json"));
    }

    #[test]
    fn detects_subcommands() {
        let info = parse_help("Commands:\n  list    List items\n  schema  Show schema");
        assert!(info.has_subcommand("list"));
        assert!(info.has_subcommand("schema"));
    }

    #[test]
    fn finds_first_list_subcommand() {
        let info = parse_help("Commands:\n  show  Show\n  list  List\n  delete  Delete");
        assert_eq!(info.first_list_subcommand(), Some("list"));
    }

    #[test]
    fn no_flags_in_empty_help() {
        let info = parse_help("");
        assert!(info.flags.is_empty());
        assert!(info.subcommands.is_empty());
    }
}
