use std::io::IsTerminal;

use owo_colors::OwoColorize;

use crate::scorer::Score;

pub fn print_score(score: &Score, json: bool) {
    // JSON mode or piped: only JSON to stdout, no stderr output
    if json || !std::io::stdout().is_terminal() {
        println!(
            "{}",
            serde_json::to_string_pretty(score).expect("serialize")
        );
        return;
    }

    // TTY mode: human-readable to stderr, JSON to stdout
    eprintln!();

    for p in &score.principles {
        let bar = render_bar(p.score, p.max);
        eprintln!("  {:<24}{} {}/{}", p.name.bold(), bar, p.score, p.max);

        for check in &p.checks {
            if check.passed {
                eprintln!("    {} {}", "\u{2713}".green(), check.name.green());
            } else {
                eprintln!("    {} {}", "\u{2717}".red(), check.name.red());
            }
        }
        eprintln!();
    }

    eprintln!(
        "  {}",
        format!(
            "Overall: {}/{} ({}%) \u{2014} {}",
            score.score, score.max, score.percentage, score.grade
        )
        .bold()
    );
    eprintln!("  Spec: https://clispec.dev");
    eprintln!();
}

fn render_bar(score: u32, max: u32) -> String {
    let width = 10;
    let filled = if max > 0 {
        ((score as f32 / max as f32) * width as f32) as usize
    } else {
        0
    };
    let empty = width - filled;
    format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty))
}
