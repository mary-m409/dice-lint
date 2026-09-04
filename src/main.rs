mod lexer;
mod linter;

use linter::Severity;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let mut lenient = false;
    let mut path: Option<String> = None;

    for arg in &args[1..] {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "--version" => {
                println!("dicelint {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("dicelint: unknown flag '{}'", other);
                return ExitCode::from(2);
            }
            other => path = Some(other.to_string()),
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("usage: dicelint [--lenient] <file>");
            return ExitCode::from(2);
        }
    };

    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dicelint: cannot read '{}': {}", path, e);
            return ExitCode::from(2);
        }
    };

    let mut findings = Vec::new();
    for (i, raw_line) in contents.lines().enumerate() {
        let line_no = i + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        findings.extend(linter::lint_line(line_no, raw_line));
    }

    let mut has_error = false;
    let mut has_warning = false;

    for finding in &findings {
        println!("{}:{}:{}: {}", path, finding.line, finding.col, finding.render());
        match finding.severity {
            Severity::Error => has_error = true,
            Severity::Warning => has_warning = true,
        }
    }

    // Strict by default: a warning is a failure unless --lenient says
    // otherwise. Errors always fail, lenient or not.
    if has_error || (has_warning && !lenient) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_help() {
    println!(
        "dicelint - a linter for dice notation\n\
         \n\
         usage:\n\
         \x20 dicelint [--lenient] <file>\n\
         \n\
         reads <file> as one dice expression per line (blank lines and lines\n\
         starting with '#' are skipped) and reports findings as:\n\
         \x20 <file>:<line>:<col>: <severity>[<code>]: <message>\n\
         \n\
         by default any warning fails the run (exit code 1). pass --lenient\n\
         to only fail on errors and let warnings through as advisories."
    );
}
