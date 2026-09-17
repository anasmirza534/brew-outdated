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

const RUN_UPDATE: bool = true;

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

    let output = Command::new("brew")
        .arg("leaves")
        .output()
        .expect("`brew leaves` failed");

    if !output.status.success() {
        eprintln!(
            "`brew leaves` returns non zero exit code: {}",
            output.status
        );

        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut formulae_names: Vec<String> = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    formulae_names.sort();

    // run concurrently based on cpu cores available
    let formulaes: Vec<Formulae> = formulae_names
        .par_iter()
        .map(|f| get_formulae_versions(f.to_string()))
        .collect();

    let mut need_update: Vec<Formulae> = vec![];
    let mut up_to_date: Vec<Formulae> = vec![];
    let mut unknowns: Vec<Formulae> = vec![];

    for formulae in formulaes.into_iter() {
        let current = formulae.current.clone().unwrap_or("null".to_string());
        let latest = formulae.latest.clone().unwrap_or("null".to_string());

        if current == "null" {
            unknowns.push(formulae);
        } else if current == latest {
            up_to_date.push(formulae);
        } else {
            need_update.push(formulae);
        }
    }

    println!("============================");
    println!("Need to update: ");
    println!("============================");
    for f in need_update {
        println!(
            "{} {} -> {}",
            f.name,
            f.current.unwrap_or("".to_string()),
            f.latest.unwrap_or("".to_string())
        );
    }

    println!("");
    println!("============================");
    println!("Up to update: ");
    println!("============================");
    for f in up_to_date {
        println!("{} {}", f.name, f.current.unwrap_or("".to_string()),);
    }

    println!("");
    println!("============================");
    println!("Unknowns: ");
    println!("============================");
    for f in unknowns {
        println!("{} {}", f.name, f.current.unwrap_or("".to_string()),);
    }
}

#[derive(Debug)]
struct Formulae {
    name: String,
    current: Option<String>,
    latest: Option<String>,
}

fn get_formulae_versions(formulae: String) -> Formulae {
    let output = Command::new("brew")
        .arg("info")
        .arg("--json=v2")
        .arg(formulae.clone())
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
            let current = json["formulae"][0]["installed"][0]["version"]
                .as_str()
                .map(|s| s.to_string());

            let latest = json["formulae"][0]["versions"]["stable"]
                .as_str()
                .map(|s| s.to_string());

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
