use rand::Rng;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{
    env,
    io::Write,
    process::{Command, ExitCode},
    thread,
};

fn main() -> ExitCode {
    let mut help_flag = false;
    let mut no_update_flag = false;
    let mut cask_flag = false;

    let args: Vec<String> = env::args().skip(1).collect();

    for arg in args.iter() {
        if arg == "--help" {
            help_flag = true;
        } else if arg == "--no-update" {
            no_update_flag = true;
        } else if arg == "--cask" {
            cask_flag = true;
        } else {
            eprintln!("unknown flag: {}", arg);
            eprintln!();
            print_help_stderr();

            return ExitCode::FAILURE;
        }
    }

    if help_flag {
        print_help_stdout();

        return ExitCode::SUCCESS;
    }

    if !no_update_flag {
        println!("running `brew update`");

        let output = match Command::new("brew").arg("update").output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("`brew update` failed: {}", e);
                return ExitCode::FAILURE;
            }
        };

        if !output.status.success() {
            eprintln!(
                "`brew update` returns non zero exit code: {}",
                output.status
            );
            return ExitCode::FAILURE;
        }
    }

    let mut brew_args = vec!["leaves"];
    if cask_flag {
        brew_args = vec!["list", "--cask"];
    }

    let cmd_name = format!("brew {}", brew_args.join(" "));

    let output = match Command::new("brew").args(brew_args.clone()).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("`{}` failed: {}", cmd_name, e);
            return ExitCode::FAILURE;
        }
    };

    if !output.status.success() {
        eprintln!(
            "`{}` returns non zero exit code: {}",
            cmd_name, output.status
        );
        return ExitCode::FAILURE;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut formulae_names: Vec<String> = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // sort in place
    formulae_names.sort();

    let formulaes = process_via_workers(
        formulae_names,
        move |f| get_formulae_versions(f.to_string(), cask_flag),
        None,
    );

    let mut need_update: Vec<Formulae> = vec![];
    let mut up_to_date: Vec<Formulae> = vec![];
    let mut unknowns: Vec<Formulae> = vec![];

    for formulae in formulaes.into_iter() {
        match (&formulae.installed, &formulae.latest) {
            (None, _) => unknowns.push(formulae),
            (Some(c), Some(l)) if c == l => up_to_date.push(formulae),
            _ => need_update.push(formulae),
        }
    }

    print_table("Need to update: ", need_update, false);

    print_table("Up to update: ", up_to_date, true);

    print_table("Unknowns: ", unknowns, true);

    ExitCode::SUCCESS
}

fn process_via_workers(
    data: Vec<String>,
    get: impl Fn(&String) -> Formulae + Send + Sync + 'static,
    worker_num: Option<usize>,
) -> Vec<Formulae> {
    let debug = false;

    let worker_count = worker_num.unwrap_or_else(|| {
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8)
    });

    let get = Arc::new(get);

    let mut handlers = vec![];
    let (tx, rx) = mpsc::channel();

    let indexed: VecDeque<(usize, String)> = data.into_iter().enumerate().collect();
    let queue = Arc::new(Mutex::new(indexed));

    for wc in 1..=worker_count {
        let worker_name = format!("{:0>2}", wc.to_string());
        if debug {
            println!("creating worker: {}", worker_name);
        }

        let get = Arc::clone(&get);
        let tx1 = tx.clone();
        let queue1 = Arc::clone(&queue);

        handlers.push(thread::spawn(move || {
            // single worker
            loop {
                let val = queue1
                    .lock()
                    .unwrap_or_else(|queue| queue.into_inner())
                    .pop_front();

                match val {
                    Some((idx, val)) => {
                        if debug {
                            println!("processing worker name: {} -> val {}", worker_name, val);
                        }

                        let formulae = get(&val);
                        tx1.send((idx, formulae)).unwrap();
                    }
                    None => break,
                }
            }
        }));
    }

    for handler in handlers.into_iter() {
        handler.join().unwrap();
    }

    drop(tx);

    let mut result: Vec<(usize, Formulae)> = rx.into_iter().collect();
    result.sort_by_key(|(idx, _)| *idx);

    result.into_iter().map(|(_, f)| f).collect()
}

fn print_help_stdout() {
    print_help(std::io::stdout());
}

fn print_help_stderr() {
    print_help(std::io::stderr());
}

fn print_help(mut out: impl Write) {
    let _ = writeln!(out, "brew-outdate: list oudated package from brew");
    let _ = writeln!(out, "version: 0.1.1");
    let _ = writeln!(out, "usage: brew-outdate [flags]");
    let _ = writeln!(out, "   flags:");
    let _ = writeln!(out, "      --cask");
    let _ = writeln!(out, "      --no-update");
    let _ = writeln!(out, "      --help");
}

fn max_width(formulaes: &[Formulae], get: impl Fn(&Formulae) -> &str) -> usize {
    formulaes.iter().map(|f| get(f).len()).max().unwrap_or(0)
}

fn print_table(title: &str, formulaes: Vec<Formulae>, only_installed: bool) {
    if formulaes.is_empty() {
        return;
    }

    let name_width = max_width(&formulaes, |f| f.name.as_str()).max("Name".len()) + 2;
    let installed_width =
        max_width(&formulaes, |f| f.installed.as_deref().unwrap_or("-")).max("Installed".len()) + 2;
    let latest_width =
        max_width(&formulaes, |f| f.latest.as_deref().unwrap_or("-")).max("Latest".len()) + 2;

    println!("{}", title);
    println!();
    if only_installed {
        println!("{:<name_width$} {:<installed_width$}", "Name", "Installed",);
    } else {
        println!(
            "{:<name_width$} {:<installed_width$}   {:<latest_width$}   Version Diff",
            "Name", "Installed", "Latest"
        );
    }

    for f in &formulaes {
        if only_installed {
            println!(
                "{:<name_width$} {:<installed_width$}",
                f.name.clone(),
                f.installed.as_deref().unwrap_or("-"),
            );
        } else {
            println!(
                "{:<name_width$} {:<installed_width$}   {:<latest_width$}   {}",
                f.name.clone(),
                f.installed.as_deref().unwrap_or("-"),
                f.latest.as_deref().unwrap_or("-"),
                f.version_diff()
            );
        }
    }
    println!();
}

struct Formulae {
    name: String,
    installed: Option<String>,
    latest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionDiff {
    Missing,
    Unparsable,
    NoChange,
    Major,
    Minor,
    Patch,
    Build,
    Downgrade,
}

impl std::fmt::Display for VersionDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Missing => "missing",
            Self::Unparsable => "unparsable",
            Self::NoChange => "no change",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::Build => "build",
            Self::Downgrade => "downgrade",
        };
        write!(f, "{s}")
    }
}

impl Formulae {
    fn new(name: String, installed: Option<String>, latest: Option<String>) -> Formulae {
        Formulae {
            name,
            installed,
            latest,
        }
    }

    fn unknown(name: String) -> Formulae {
        Formulae {
            name,
            installed: None,
            latest: None,
        }
    }

    /// Classifies the change between the installed and latest version.
    fn version_diff(&self) -> VersionDiff {
        let (Some(old), Some(new)) = (&self.installed, &self.latest) else {
            return VersionDiff::Missing;
        };

        let (Some((a, a_rev)), Some((b, b_rev))) =
            (parse_version_numbers(old), parse_version_numbers(new))
        else {
            return VersionDiff::Unparsable;
        };

        for i in 0..a.len().max(b.len()) {
            let x = a.get(i).copied().unwrap_or(0);
            let y = b.get(i).copied().unwrap_or(0);
            if x != y {
                if x > y {
                    return VersionDiff::Downgrade;
                }

                return match i {
                    0 => VersionDiff::Major,
                    1 => VersionDiff::Minor,
                    2 => VersionDiff::Patch,
                    _ => VersionDiff::Build, // 4th+ segment: revision/build number
                };
            }
        }

        // Homebrew revision (`_N` suffix) only breaks ties between equal versions
        if a_rev != b_rev {
            return if a_rev > b_rev {
                VersionDiff::Downgrade
            } else {
                VersionDiff::Build
            };
        }

        // Numbers equal but raw strings differ (e.g. only the hash/build after the comma changed)
        if version_build_id(old) == version_build_id(new) {
            VersionDiff::NoChange
        } else {
            VersionDiff::Build
        }
    }
}

/// Returns the build identifier that follows the first comma in a version
/// string, trimmed (e.g. `"3.23.12,2d29876d"` -> `Some("2d29876d")`).
///
/// Returns `None` when there is no comma (e.g. `"157.0"`).
fn version_build_id(v: &str) -> Option<&str> {
    v.split_once(',').map(|(_, m)| m.trim())
}

/// Parses the numeric part of a version string into its segments and its
/// Homebrew revision (e.g. `"v9.4.1_2,44384"` -> `Some(([9, 4, 1], 2))`).
///
/// Ignores anything after the first comma, strips a leading `v`, splits a
/// trailing `_N` revision (0 if absent), and splits the rest on `.` and `-`.
/// Returns `None` if any segment is not a plain number (e.g. `"1.2.3-beta"`).
fn parse_version_numbers(v: &str) -> Option<(Vec<u64>, u64)> {
    let v = v.split(',').next()?.trim().trim_start_matches('v');
    let (base, rev) = match v.split_once('_') {
        Some((b, r)) => (b, r.parse().ok()?),
        None => (v, 0),
    };
    let nums = base
        .split(['.', '-'])
        .map(|p| p.parse().ok())
        .collect::<Option<Vec<u64>>>()?;
    Some((nums, rev))
}

fn get_formulae_versions(formulae: String, is_cask: bool) -> Formulae {
    let mut brew_args = vec!["info", "--json=v2"];
    if is_cask {
        brew_args.push("--cask")
    }
    brew_args.push(&formulae);

    let output = match Command::new("brew").args(brew_args).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!(
                "failed to get info for formulae: {}, error: {}",
                formulae, e
            );

            return Formulae::unknown(formulae);
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        eprintln!(
            "`brew info` return non zero exit code: {} and error: {}",
            output.status, stderr
        );

        return Formulae::unknown(formulae);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    match serde_json::from_str::<Value>(&stdout) {
        Ok(json) => {
            let installed = if is_cask {
                json["casks"][0]["installed"]
                    .as_str()
                    .map(|s| s.to_string())
            } else {
                json["formulae"][0]["installed"][0]["version"]
                    .as_str()
                    .map(|s| s.to_string())
            };

            let latest = if is_cask {
                json["casks"][0]["version"].as_str().map(|s| s.to_string())
            } else {
                json["formulae"][0]["versions"]["stable"]
                    .as_str()
                    .map(|s| s.to_string())
            };

            Formulae::new(formulae, installed, latest)
        }
        Err(e) => {
            eprintln!("failed to parse json: {}", e);

            Formulae::unknown(formulae)
        }
    }
}

#[allow(dead_code)]
fn get_formulae_mock(formulae: String) -> Formulae {
    let wait_mili: u32 = rand::thread_rng().gen_range(200..1500);

    println!("Processing: {}\t\twait: {:<5} mili", formulae, wait_mili);

    thread::sleep(Duration::from_millis(wait_mili as u64));

    let prob = rand::random::<f64>();
    if prob <= 0.1 {
        return Formulae {
            name: formulae,
            installed: None,
            latest: None,
        };
    }

    if (0.1..=0.75).contains(&prob) {
        return Formulae {
            name: formulae,
            installed: Some("1.0.3".to_string()),
            latest: Some("1.0.3".to_string()),
        };
    }

    Formulae {
        name: formulae,
        installed: Some("2.5.3".to_string()),
        latest: Some("2.8.11".to_string()),
    }
}

#[test]
fn test_process_via_workers() {
    let vals: Vec<String> = (1..=18).map(|n| n.to_string()).collect();

    let result = process_via_workers(vals.clone(), |f| get_formulae_mock(f.to_string()), None);

    assert!(vals.len() == result.len());

    assert!(result.iter().map(|f| &f.name).eq(vals.iter()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(a: &str, b: &str) -> VersionDiff {
        Formulae {
            name: "test".into(),
            installed: Some(a.into()),
            latest: Some(b.into()),
        }
        .version_diff()
    }

    #[test]
    fn classifies() {
        assert_eq!(diff("157.0", "157.0.1"), VersionDiff::Patch);
        assert_eq!(diff("8.1_1", "9.0.2"), VersionDiff::Major);
        assert_eq!(diff("7.1.2-27", "7.1.2-32"), VersionDiff::Build);
        assert_eq!(diff("1.0", "1.0.0"), VersionDiff::NoChange);
        assert_eq!(diff("3.23.12,abc", "3.23.12,def"), VersionDiff::Build);
        assert_eq!(diff("1.2.3-beta", "1.2.3"), VersionDiff::Unparsable);
        assert_eq!(diff("1.2.3_1", "1.2.3"), VersionDiff::Downgrade);
        assert_eq!(diff("1.2.3", "1.2.3_1"), VersionDiff::Build);
        assert_eq!(diff("1.2.3_1", "1.2.3_1"), VersionDiff::NoChange);
        assert_eq!(diff("1.2.3", "1.3.0"), VersionDiff::Minor);
        assert_eq!(diff("2.0", "1.9"), VersionDiff::Downgrade);
    }
}
