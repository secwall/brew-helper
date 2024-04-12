use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::process::Command;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(about = "A little brew helper to remove non needed formulas")]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Remove all formulas not used as deps")]
    List {},
    #[command(arg_required_else_help = true)]
    #[command(about = "Remove a formula with all unused deps")]
    RMDep {
        #[arg(value_name = "Name for a target formula")]
        name: Option<String>,
    },
}

#[derive(Deserialize)]
struct BrewFormulaVersion {
    bottle: bool,
}

#[derive(Deserialize)]
struct BrewRuntimeDependency {
    full_name: String,
}

#[derive(Deserialize)]
struct BrewInstalledVersion {
    runtime_dependencies: Vec<BrewRuntimeDependency>,
}

#[derive(Deserialize)]
struct BrewFormula {
    full_name: String,
    dependencies: Vec<String>,
    oldnames: Vec<String>,
    build_dependencies: Vec<String>,
    versions: BrewFormulaVersion,
    installed: Vec<BrewInstalledVersion>,
}

fn get_brew_list() -> Vec<String> {
    let out = Command::new("brew")
        .arg("list")
        .arg("--formula")
        .output()
        .expect("failed to run brew list");
    if !out.status.success() {
        let stderr_str = String::from_utf8(out.stderr).expect("unable to parse brew list stderr");
        println!("{stderr_str}");
        std::process::exit(1);
    }
    let out_str = String::from_utf8(out.stdout).expect("unable to parse brew list result");
    out_str.split_whitespace().map(str::to_string).collect()
}

fn get_brew_deps(formulas: &Vec<String>) -> HashSet<String> {
    let mut renames = HashMap::new();
    let mut ret = HashSet::new();
    let out = Command::new("brew")
        .arg("info")
        .arg("--json")
        .args(formulas)
        .output()
        .expect("failed to run brew info");
    if !out.status.success() {
        let stderr_str = String::from_utf8(out.stderr).expect("unable to parse brew info stderr");
        println!("{stderr_str}");
        std::process::exit(1);
    }
    let out_str = String::from_utf8(out.stdout).expect("unable to parse brew info result");
    let parsed: Vec<BrewFormula> =
        serde_json::from_str(&out_str).expect("unable to parse brew info result");
    for formula in parsed {
        for oldname in formula.oldnames {
            renames.insert(oldname, formula.full_name.clone());
        }
        for dep in formula.dependencies {
            ret.insert(dep);
        }
        for version in formula.installed {
            for runtime_dependency in version.runtime_dependencies {
                ret.insert(runtime_dependency.full_name);
            }
        }
        if formula.versions.bottle {
            continue;
        }
        for dep in formula.build_dependencies {
            ret.insert(dep);
        }
    }
    for (oldname, newname) in renames.into_iter() {
        if ret.contains(&newname) {
            ret.insert(oldname);
        }
    }
    ret
}

fn get_non_dep_formulas() -> Vec<String> {
    let mut ret = Vec::new();
    let brew_list = get_brew_list();
    let used_as_deps = get_brew_deps(&brew_list);
    for formula in brew_list {
        if !used_as_deps.contains(&formula) {
            ret.push(formula);
        }
    }
    ret
}

fn remove_brew_formula_with_deps(formula: String) {
    let initial = get_non_dep_formulas();
    let mut initial_set = HashSet::new();
    for formula in initial {
        initial_set.insert(formula);
    }
    if !initial_set.contains(&formula) {
        println!("{formula} is some other formula dep");
    } else {
        let mut targets = Vec::new();
        targets.push(formula);
        loop {
            while let Some(target) = targets.pop() {
                println!("Removing {target}");
                let res = Command::new("brew")
                    .arg("rm")
                    .arg(target)
                    .output()
                    .expect("failed to run brew rm {target}");
                if !res.status.success() {
                    let stderr_str =
                        String::from_utf8(res.stderr).expect("unable to parse brew rm stderr");
                    println!("{stderr_str}");
                    std::process::exit(1);
                }
            }
            let current = get_non_dep_formulas();
            for target in current {
                if !initial_set.contains(&target) {
                    println!("Found new unused dep: {target}");
                    targets.push(target)
                }
            }
            if targets.is_empty() {
                break;
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::List {}) => {
            let formulas = get_non_dep_formulas();
            for formula in formulas {
                println!("{}", formula);
            }
        }
        Some(Commands::RMDep { name }) => {
            if let Some(name) = name.as_deref() {
                remove_brew_formula_with_deps(name.to_string())
            }
        }
        None => {}
    }
}
