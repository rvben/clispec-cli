use std::io::IsTerminal;

use owo_colors::OwoColorize;

use crate::scorer::Score;

/// Print the score. `as_json` is the already-resolved output decision (see
/// `OutputFormat::is_json`): JSON to stdout, or the human-readable report to
/// stdout. Either way the primary output goes to stdout so it composes cleanly.
pub fn print_score(score: &Score, as_json: bool) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(score).expect("serialize")
        );
        return;
    }

    // Only colorize for a terminal, so `clispec -o text score X | cat` stays
    // plain text instead of leaking ANSI escapes.
    let color = std::io::stdout().is_terminal();
    let paint = |text: String, style: fn(&str) -> String| {
        if color { style(&text) } else { text }
    };

    println!();

    for p in &score.principles {
        let bar = render_bar(p.score, p.max);
        println!(
            "  {:<24}{} {}/{}",
            paint(p.name.to_string(), |s| s.bold().to_string()),
            bar,
            p.score,
            p.max
        );

        for check in &p.checks {
            if check.passed {
                println!(
                    "    {} {}",
                    paint("\u{2713}".to_string(), |s| s.green().to_string()),
                    paint(check.name.to_string(), |s| s.green().to_string())
                );
            } else {
                // Cite the conformance checklist item behind the failure so
                // the score points at spec text, not scorer behavior.
                let citation = check
                    .checklist
                    .map(|id| format!(" [{id}]"))
                    .unwrap_or_default();
                println!(
                    "    {} {}{}",
                    paint("\u{2717}".to_string(), |s| s.red().to_string()),
                    paint(check.name.to_string(), |s| s.red().to_string()),
                    paint(citation, |s| s.dimmed().to_string())
                );
            }
        }
        println!();
    }

    // Summarize which conformance checklist items are not yet satisfied,
    // in checklist order, so the score points back at spec text.
    let failing: Vec<&str> = score
        .principles
        .iter()
        .flat_map(|p| &p.checks)
        .filter(|c| !c.passed)
        .filter_map(|c| c.checklist)
        .collect();
    let unsatisfied: Vec<(&str, &str)> = crate::checks::CHECKLIST_ITEMS
        .iter()
        .filter(|(id, _)| failing.contains(id))
        .copied()
        .collect();
    if !unsatisfied.is_empty() {
        println!(
            "  {}",
            paint("Unsatisfied checklist items:".to_string(), |s| s
                .bold()
                .to_string())
        );
        for (id, summary) in unsatisfied {
            println!(
                "    {} {}",
                paint(format!("[{id}]"), |s| s.yellow().to_string()),
                summary
            );
        }
        println!();
    }

    println!(
        "  {}",
        paint(
            format!(
                "Overall: {}/{} ({}%) \u{2014} {}",
                score.score, score.max, score.percentage, score.grade
            ),
            |s| s.bold().to_string()
        )
    );
    println!("  Spec: https://clispec.dev/#conformance");
    println!();
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
