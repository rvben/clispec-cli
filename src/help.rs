pub struct HelpInfo {
    pub flags: Vec<String>,
    pub subcommands: Vec<String>,
    /// Subcommand names parsed from a "Commands:" help section, in order.
    pub listed_subcommands: Vec<String>,
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

    HelpInfo {
        flags,
        subcommands,
        listed_subcommands: parse_command_section(help_text),
    }
}

/// Parse subcommand names from a "Commands:" / "Subcommands:" / "Available
/// Commands:" section (the clap, click, and cobra layouts): indented lines
/// whose first word is the command name. The section ends at the next
/// un-indented line (the next section header).
fn parse_command_section(help_text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_section = false;

    for line in help_text.lines() {
        let header = line.trim().to_lowercase();
        if !line.starts_with([' ', '\t']) && header.ends_with("commands:") {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            in_section = false;
            continue;
        }
        if let Some(name) = line.split_whitespace().next()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            names.push(name.to_string());
        }
    }

    names
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
        assert!(info.listed_subcommands.is_empty());
    }

    #[test]
    fn parses_clap_command_section() {
        let help = "Usage: mytool [OPTIONS] <COMMAND>\n\n\
                    Commands:\n  \
                    apps   Manage apps\n  \
                    score  Score a tool\n  \
                    help   Print this message\n\n\
                    Options:\n  -h, --help  Print help";
        let info = parse_help(help);
        assert_eq!(info.listed_subcommands, vec!["apps", "score", "help"]);
    }

    #[test]
    fn parses_cobra_available_commands_section() {
        let help = "Usage:\n  mytool [command]\n\n\
                    Available Commands:\n  \
                    completion  Generate completions\n  \
                    list        List things\n\n\
                    Flags:\n  -h, --help  help for mytool";
        let info = parse_help(help);
        assert_eq!(info.listed_subcommands, vec!["completion", "list"]);
    }

    #[test]
    fn prose_outside_command_section_is_not_a_subcommand() {
        let help = "Usage: mytool\n\nA tool that manages things.\n\n\
                    Commands:\n  list  List things\n\n\
                    Options:\n  --verbose  Be chatty";
        let info = parse_help(help);
        assert_eq!(info.listed_subcommands, vec!["list"]);
    }
}
