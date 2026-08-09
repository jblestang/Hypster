//! Hypster development task runner.

mod qemu;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use hv_config_model::artifact::GeneratedArtifacts;
use hv_config_model::pipeline::compile_config;
use hv_config_model::yaml::read_yaml_file;
use hv_core::fixture::qemu_validation_observed;
use hv_core::resolve_platform;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None => Err(usage()),
        Some("test") if args.len() == 1 => run_tests(),
        Some("build") if args.len() == 1 => run_build(),
        Some("config") => match args.get(1).map(String::as_str) {
            Some("validate") if args.len() == 3 => validate_config(Path::new(&args[2])),
            Some("generate") => {
                let path = args.get(2).ok_or_else(usage).map(PathBuf::from)?;
                let output = parse_output_flag(&args[3..])?;
                generate_config(&path, &output)
            }
            _ => Err(usage()),
        },
        Some("platform") if args.len() == 3 && args[1] == "resolve" => {
            platform_resolve(Path::new(&args[2]))
        }
        Some("datapath") if args.len() == 2 && args[1] == "smoke" => datapath_smoke(),
        Some("datapath") if args.len() == 2 && args[1] == "e2e" => datapath_e2e(),
        Some("qemu") => match args.get(1).map(String::as_str) {
            Some("prepare") if args.len() == 2 => qemu_prepare(None),
            Some("prepare") if args.len() == 3 => qemu_prepare(Some(Path::new(&args[2]))),
            Some("run") if args.len() == 2 => qemu_run(None, false),
            Some("run") if args.len() == 3 && args[2] == "--headless" => qemu_run(None, true),
            Some("run") if args.len() == 4 && args[2] == "--headless" => {
                qemu_run(Some(Path::new(&args[3])), true)
            }
            Some("run") if args.len() == 3 => qemu_run(Some(Path::new(&args[2])), false),
            Some("smoke") if args.len() == 2 => qemu_smoke(None),
            Some("smoke") if args.len() == 3 => qemu_smoke(Some(Path::new(&args[2]))),
            _ => Err(usage()),
        },
        _ => Err(usage()),
    }
}

fn parse_output_flag(args: &[String]) -> Result<PathBuf, String> {
    let mut output = PathBuf::from("target/generated");
    let mut idx = 0;
    while idx < args.len() {
        if args[idx] == "--output" {
            let value =
                args.get(idx + 1).ok_or_else(|| "missing value for --output".to_string())?;
            output = PathBuf::from(value);
            idx += 2;
        } else {
            return Err(format!("unexpected argument `{}`", args[idx]));
        }
    }
    Ok(output)
}

fn run_tests() -> Result<(), String> {
    run_cmd(workspace_root(), "cargo", &["test", "--workspace"])
}

fn run_build() -> Result<(), String> {
    run_cmd(workspace_root(), "cargo", &["build", "--workspace"])
}

fn datapath_smoke() -> Result<(), String> {
    run_cmd(
        workspace_root(),
        "cargo",
        &[
            "test",
            "-p",
            "hv-runtime",
            "gate_d_e2e_datapath_moves_payload_through_engine",
            "--",
            "--nocapture",
        ],
    )
}

fn datapath_e2e() -> Result<(), String> {
    qemu::datapath_e2e(&workspace_root(), &qemu::default_config_path(&workspace_root()))
}

fn qemu_prepare(config: Option<&Path>) -> Result<(), String> {
    let root = workspace_root();
    let default = qemu::default_config_path(&root);
    let path = config.unwrap_or(&default);
    let report = qemu::prepare(&root, path)?;
    println!("prepared ESP at {}", report.esp_root.display());
    println!("loader={}", report.loader_efi.display());
    println!("hypervisor={}", report.hypervisor_bin.display());
    Ok(())
}

fn qemu_run(config: Option<&Path>, headless: bool) -> Result<(), String> {
    let root = workspace_root();
    let default = qemu::default_config_path(&root);
    let path = config.unwrap_or(&default);
    qemu::run(&root, path, headless)
}

fn qemu_smoke(config: Option<&Path>) -> Result<(), String> {
    let root = workspace_root();
    let default = qemu::default_config_path(&root);
    let path = config.unwrap_or(&default);
    qemu::smoke(&root, path)
}

fn validate_config(path: &Path) -> Result<(), String> {
    let raw = read_yaml_file(path.to_str().ok_or("invalid config path")?)
        .map_err(|err| err.to_string())?;
    let compiled = compile_config(raw).map_err(|err| err.to_string())?;
    println!(
        "configuration `{}` valid; hash={}",
        compiled.intent.name,
        compiled.validated.config_hash.to_hex()
    );
    println!(
        "partitions={} ipc={} min_physical_cores={}",
        compiled.intent.partitions.len(),
        compiled.intent.ipc.len(),
        compiled.intent.platform_requirements.min_physical_cores
    );
    Ok(())
}

fn platform_resolve(path: &Path) -> Result<(), String> {
    let raw = read_yaml_file(path.to_str().ok_or("invalid config path")?)
        .map_err(|err| err.to_string())?;
    let compiled = compile_config(raw).map_err(|err| err.to_string())?;
    let observed = qemu_validation_observed().map_err(|err| err.to_string())?;
    let resolved = resolve_platform(&compiled.intent, &observed).map_err(|err| err.to_string())?;
    println!(
        "platform `{}` resolved; hash={}",
        resolved.intent.name,
        resolved.config_hash.to_hex()
    );
    println!(
        "cpu_assignments={} memory_regions={} ept_partitions={} vtd_domains={}",
        resolved.cpu.assignments.len(),
        resolved.memory.regions.len(),
        resolved.ept.partitions.len(),
        resolved.vtd.domains.len()
    );
    Ok(())
}

fn generate_config(path: &Path, output: &Path) -> Result<(), String> {
    let raw = read_yaml_file(path.to_str().ok_or("invalid config path")?)
        .map_err(|err| err.to_string())?;
    let compiled = compile_config(raw).map_err(|err| err.to_string())?;
    let artifacts = GeneratedArtifacts::from_compiled(&compiled);
    std::fs::create_dir_all(output).map_err(|err| err.to_string())?;
    for artifact in artifacts.files {
        let file_path = output.join(&artifact.path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        std::fs::write(&file_path, artifact.contents).map_err(|err| err.to_string())?;
        println!("generated {}", file_path.display());
    }
    Ok(())
}

fn run_cmd(current_dir: PathBuf, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command `{program} {}` failed", args.join(" ")))
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn usage() -> String {
    "usage: cargo xtask <test|build|config validate <path>|config generate <path> [--output dir]|platform resolve <path>|datapath smoke|datapath e2e|qemu prepare [path]|qemu run [path] [--headless]|qemu smoke [path]>".into()
}
