use rand;
use rand::Rng;
use rayon::prelude::*;
use serde_json::Value;
use std::{process::Command, thread::sleep, time::Duration};

// command structure:
// brew-outdate
//      flags:
//          --cask
//          --no-update

const RUN_UPDATE: bool = false;
const IS_CASK: bool = true;

fn main() {
    if RUN_UPDATE {
        println!("running `brew update`");

        let output = Command::new("brew")
            .arg("update")
            .output()
            .expect("`brew update` failed");

        if !output.status.success() {
            eprintln!(
                "`brew update` returns non zero exit code: {}",
                output.status
            );

            return;
        }
    }

    let mut args = vec!["leaves"];
    if IS_CASK {
        args = vec!["list", "--cask"];
    }

    let cmd_name = format!("brew {}", args.join(" "));

    let output = Command::new("brew")
        .args(args.clone())
        .output()
        .expect(format!("`{}` failed", cmd_name).as_str());

    if !output.status.success() {
        eprintln!(
            "`{}` returns non zero exit code: {}",
            cmd_name, output.status
        );

        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut formulae_names: Vec<String> = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // sort in place
    formulae_names.sort();

    // run concurrently based on cpu cores available
    let formulaes: Vec<Formulae> = formulae_names
        .par_iter()
        .map(|f| get_formulae_versions(f.to_string(), IS_CASK))
        .collect();

    let mut need_update: Vec<Formulae> = vec![];
    let mut up_to_date: Vec<Formulae> = vec![];
    let mut unknowns: Vec<Formulae> = vec![];

    for formulae in formulaes.into_iter() {
        match (&formulae.current, &formulae.latest) {
            (None, _) => unknowns.push(formulae),
            (Some(c), Some(l)) if c == l => up_to_date.push(formulae),
            _ => need_update.push(formulae),
        }
    }

    print_table("Need to update: ", need_update);

    print_table("Up to update: ", up_to_date);

    print_table("Unknowns: ", unknowns);
}

fn max_width(formulaes: &[Formulae], get: impl Fn(&Formulae) -> &str) -> usize {
    formulaes.iter().map(|f| get(f).len()).max().unwrap_or(0)
}

fn print_table(title: &str, formulaes: Vec<Formulae>) {
    if formulaes.is_empty() {
        return;
    }

    let name_width = max_width(&formulaes, |f| f.name.as_str()) + 2;
    let current_width = max_width(&formulaes, |f| f.current.as_deref().unwrap_or("-")) + 2;
    let latest_width = max_width(&formulaes, |f| f.latest.as_deref().unwrap_or("-")) + 2;

    println!("{}", title);
    println!("");
    for f in &formulaes {
        println!(
            "{:<name_width$} {:<current_width$} ->   {:<latest_width$}",
            f.name.clone(),
            f.current.clone().unwrap_or("-".into()),
            f.latest.clone().unwrap_or("-".into()),
        );
    }
    println!("");
}

struct Formulae {
    name: String,
    current: Option<String>,
    latest: Option<String>,
}

fn get_formulae_versions(formulae: String, is_cask: bool) -> Formulae {
    let mut args = vec!["info", "--json=v2"];
    if is_cask {
        args.push("--cask")
    }
    args.push(&formulae);

    let output = Command::new("brew")
        .args(args)
        .output()
        .expect("failed to execute command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        eprintln!(
            "`brew info` return non zero exit code: {} and error: {}",
            output.status, stderr
        );

        return Formulae {
            name: formulae,
            current: None,
            latest: None,
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    match serde_json::from_str::<Value>(&stdout) {
        Ok(json) => {
            let current = if is_cask {
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

            Formulae {
                name: formulae,
                current: current,
                latest: latest,
            }
        }
        Err(e) => {
            eprintln!("failed to parse json: {}", e);

            Formulae {
                name: formulae,
                current: None,
                latest: None,
            }
        }
    }
}

#[allow(dead_code)]
fn get_formulae_mock(formulae: String) -> Formulae {
    println!("Processing: {}", formulae);

    let n: u32 = rand::thread_rng().gen_range(200..1500);

    sleep(Duration::from_millis(n.into()));

    let prob = rand::random::<f64>();
    if prob <= 0.1 {
        return Formulae {
            name: formulae,
            current: None,
            latest: None,
        };
    }

    if prob >= 0.1 && prob <= 0.75 {
        return Formulae {
            name: formulae,
            current: Some("1.0.3".to_string()),
            latest: Some("1.0.3".to_string()),
        };
    }

    Formulae {
        name: formulae,
        current: Some("2.5.3".to_string()),
        latest: Some("2.8.11".to_string()),
    }
}
