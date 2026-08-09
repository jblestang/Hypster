//! Hypster configuration compiler CLI.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hv_config_model::artifact::GeneratedArtifacts;
use hv_config_model::pipeline::compile_config;
use hv_config_model::yaml::read_yaml_file;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hv-config error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("validate") if args.len() == 2 => validate(Path::new(&args[1])),
        Some("generate") => {
            let path = args.get(1).ok_or_else(usage).map(PathBuf::from)?;
            let output = parse_output_flag(&args[2..])?;
            generate(&path, &output)
        }
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

fn validate(path: &Path) -> Result<(), String> {
    let raw = read_yaml_file(path.to_str().ok_or("invalid path")?).map_err(|e| e.to_string())?;
    let compiled = compile_config(raw).map_err(|e| e.to_string())?;
    println!("valid: {}", compiled.intent.name);
    println!("hash: {}", compiled.validated.config_hash.to_hex());
    Ok(())
}

fn generate(path: &Path, output: &Path) -> Result<(), String> {
    let raw = read_yaml_file(path.to_str().ok_or("invalid path")?).map_err(|e| e.to_string())?;
    let compiled = compile_config(raw).map_err(|e| e.to_string())?;
    let artifacts = GeneratedArtifacts::from_compiled(&compiled);
    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
    for artifact in artifacts.files {
        let out_path = output.join(&artifact.path);
        std::fs::write(out_path, artifact.contents).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn usage() -> String {
    "usage: hv-config <validate <path>|generate <path> [--output dir]>".into()
}
