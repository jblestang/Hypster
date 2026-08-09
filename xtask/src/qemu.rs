//! QEMU/OVMF integration helpers for Gate D end-to-end validation.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use hv_config_model::artifact::GeneratedArtifacts;
use hv_config_model::pipeline::compile_config;
use hv_config_model::yaml::read_yaml_file;

const DEFAULT_CONFIG: &str = "configs/qemu.yaml";
const GENERATED_DIR: &str = "target/generated";
const ESP_ROOT: &str = "target/qemu/esp";
const QEMU_ARGS_FILE: &str = "qemu-args.txt";

/// Summary returned after a successful `qemu prepare`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrepareReport {
    /// Absolute path to the FAT ESP host directory.
    pub esp_root: PathBuf,
    /// Absolute path to generated `qemu-args.txt`.
    pub qemu_args: PathBuf,
    /// Loader copied to `EFI/BOOT/BOOTX64.EFI`.
    pub loader_efi: PathBuf,
    /// Hypervisor flat binary at `EFI/hypster/hypster.bin`.
    pub hypervisor_bin: PathBuf,
}

/// Builds loader, hypervisor, guests, generated artifacts, and the ESP layout.
pub fn prepare(workspace: &Path, config: &Path) -> Result<PrepareReport, String> {
    let generated = workspace.join(GENERATED_DIR);
    generate_config(config, &generated)?;
    build_artifacts(workspace)?;
    layout_esp(workspace)
}

/// Launches QEMU using generated args plus the prepared ESP directory.
pub fn run(workspace: &Path, config: &Path, headless: bool) -> Result<(), String> {
    let esp_root = workspace.join(ESP_ROOT);
    if !esp_root.join("EFI/BOOT/BOOTX64.EFI").is_file() {
        return Err("ESP not prepared; run `cargo xtask qemu prepare` first".into());
    }

    let qemu = find_qemu()?;
    let mut args = assemble_qemu_args(workspace, config)?;
    if headless {
        args.extend(["-serial", "stdio", "-display", "none", "-no-reboot"].map(str::to_string));
    }
    println!("running `{}` {}", qemu, args.join(" "));
    run_cmd(&qemu, &args)
}

struct LaunchBootOptions {
    nested_vmx: bool,
}

impl Default for LaunchBootOptions {
    fn default() -> Self {
        Self { nested_vmx: false }
    }
}

/// Prepares artifacts and boots QEMU when available, waiting for the running marker.
pub fn smoke(workspace: &Path, config: &Path) -> Result<(), String> {
    prepare(workspace, config)?;
    boot_until_marker(
        workspace,
        config,
        Duration::from_secs(60),
        "hypster: gate-d running",
        LaunchBootOptions::default(),
    )
}

/// Prepares launch artifacts including hardware-enabled hypervisor and static guests.
pub fn launch_prepare(workspace: &Path, config: &Path) -> Result<PrepareReport, String> {
    let _report = prepare(workspace, config)?;
    run_cargo_with_rustflags(
        workspace,
        &[
            "build",
            "-p",
            "hypster",
            "--target",
            "x86_64-unknown-none",
            "--release",
            "--features",
            "bare-metal-bin,hardware",
        ],
        Some("-C relocation-model=static"),
    )?;
    layout_esp(workspace)
}

/// Boots QEMU under KVM and waits for the guest VMLAUNCH marker.
pub fn launch_smoke(workspace: &Path, config: &Path) -> Result<(), String> {
    launch_prepare(workspace, config)?;
    if !kvm_usable() {
        println!("KVM unavailable (need read access to /dev/kvm); skipping launch smoke");
        return Ok(());
    }
    boot_until_marker(
        workspace,
        config,
        Duration::from_secs(90),
        "hypster: vmlaunch ok",
        LaunchBootOptions { nested_vmx: true },
    )
}

/// Prepares artifacts and boots QEMU when available, waiting for the datapath marker.
pub fn datapath_e2e(workspace: &Path, config: &Path) -> Result<(), String> {
    prepare(workspace, config)?;
    boot_until_marker(
        workspace,
        config,
        Duration::from_secs(90),
        "hypster: e2e ok",
        LaunchBootOptions::default(),
    )
}

fn generate_config(config: &Path, output: &Path) -> Result<(), String> {
    let raw = read_yaml_file(config.to_str().ok_or("invalid config path")?)
        .map_err(|err| err.to_string())?;
    let compiled = compile_config(raw).map_err(|err| err.to_string())?;
    let artifacts = GeneratedArtifacts::from_compiled(&compiled);
    fs::create_dir_all(output).map_err(|err| err.to_string())?;
    for artifact in artifacts.files {
        let file_path = output.join(&artifact.path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        fs::write(&file_path, artifact.contents).map_err(|err| err.to_string())?;
        println!("generated {}", file_path.display());
    }
    Ok(())
}

fn build_artifacts(workspace: &Path) -> Result<(), String> {
    let builds: [(&str, &str, &[&str], Option<&str>); 5] = [
        ("hv-loader", "x86_64-unknown-uefi", &["--features", "uefi-bin"], None),
        (
            "hypster",
            "x86_64-unknown-none",
            &["--features", "bare-metal-bin"],
            Some("-C relocation-model=static"),
        ),
        ("guest-in", "x86_64-unknown-none", &["--features", "bare-metal-bin"], Some("-C relocation-model=static")),
        ("guest-mid", "x86_64-unknown-none", &["--features", "bare-metal-bin"], Some("-C relocation-model=static")),
        ("guest-out", "x86_64-unknown-none", &["--features", "bare-metal-bin"], Some("-C relocation-model=static")),
    ];
    for (package, target, features, rustflags) in builds {
        let mut args = vec!["build", "-p", package, "--target", target, "--release"];
        args.extend_from_slice(features);
        run_cargo_with_rustflags(workspace, &args, rustflags)?;
    }
    Ok(())
}

fn layout_esp(workspace: &Path) -> Result<PrepareReport, String> {
    let esp_root = workspace.join(ESP_ROOT);
    if esp_root.exists() {
        fs::remove_dir_all(&esp_root).map_err(|err| err.to_string())?;
    }
    let boot_dir = esp_root.join("EFI/BOOT");
    let hypster_dir = esp_root.join("EFI/hypster");
    fs::create_dir_all(&boot_dir).map_err(|err| err.to_string())?;
    fs::create_dir_all(&hypster_dir).map_err(|err| err.to_string())?;

    let loader_src = workspace.join("target/x86_64-unknown-uefi/release/hv-loader.efi");
    let loader_dst = boot_dir.join("BOOTX64.EFI");
    if !loader_src.is_file() {
        return Err(format!("missing loader artifact at {}", loader_src.display()));
    }
    fs::copy(&loader_src, &loader_dst).map_err(|err| err.to_string())?;

    let hypervisor_elf = workspace.join("target/x86_64-unknown-none/release/hypster");
    let hypervisor_bin = hypster_dir.join("hypster.bin");
    if !hypervisor_elf.is_file() {
        return Err(format!("missing hypervisor artifact at {}", hypervisor_elf.display()));
    }
    objcopy_to_binary(&hypervisor_elf, &hypervisor_bin)?;

    let qemu_args = workspace.join(GENERATED_DIR).join(QEMU_ARGS_FILE);
    Ok(PrepareReport {
        esp_root: esp_root.clone(),
        qemu_args: qemu_args.clone(),
        loader_efi: loader_dst,
        hypervisor_bin,
    })
}

fn objcopy_to_binary(elf: &Path, bin: &Path) -> Result<(), String> {
    let program = find_objcopy()?;
    let status = Command::new(&program)
        .args([
            "-O",
            "binary",
            elf.to_str().ok_or("invalid elf path")?,
            bin.to_str().ok_or("invalid bin path")?,
        ])
        .status()
        .map_err(|err| format!("failed to run `{program}`: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{}` failed for {}", program, elf.display()))
    }
}

fn assemble_qemu_args(workspace: &Path, config: &Path) -> Result<Vec<String>, String> {
    let args_path = workspace.join(GENERATED_DIR).join(QEMU_ARGS_FILE);
    if !args_path.is_file() {
        return Err(format!("missing generated args at {}", args_path.display()));
    }
    let mut args = parse_qemu_args_file(&args_path)?;
    let ovmf = find_ovmf(config)?;
    replace_bios_arg(&mut args, &ovmf);
    prefer_kvm_accel(&mut args);
    if !kvm_usable() {
        replace_cpu_arg(&mut args, "max");
    }
    append_esp_drive(&mut args, &workspace.join(ESP_ROOT));
    Ok(args)
}

fn parse_qemu_args_file(path: &Path) -> Result<Vec<String>, String> {
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut args = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        args.extend(split_qemu_arg_line(line));
    }
    Ok(args)
}

fn split_qemu_arg_line(line: &str) -> Vec<String> {
    if let Some((flag, value)) = line.split_once(' ') {
        vec![flag.to_string(), value.to_string()]
    } else {
        vec![line.to_string()]
    }
}

fn replace_bios_arg(args: &mut Vec<String>, ovmf: &Path) {
    let ovmf_text = ovmf.to_string_lossy().into_owned();
    if let Some(idx) = args.iter().position(|arg| arg == "-bios") {
        if let Some(value) = args.get_mut(idx + 1) {
            *value = ovmf_text;
            return;
        }
    }
    args.push("-bios".into());
    args.push(ovmf_text);
}

fn replace_cpu_arg(args: &mut [String], cpu: &str) {
    if let Some(idx) = args.iter().position(|arg| arg == "-cpu") {
        if let Some(value) = args.get_mut(idx + 1) {
            *value = cpu.into();
        }
    }
}

fn prefer_kvm_accel(args: &mut [String]) {
    if !kvm_usable() {
        return;
    }
    if let Some(idx) = args.iter().position(|arg| arg == "-accel") {
        if let Some(value) = args.get_mut(idx + 1) {
            *value = "kvm".into();
        }
    }
}

fn kvm_usable() -> bool {
    fs::OpenOptions::new().read(true).open("/dev/kvm").is_ok()
}

fn enable_nested_vmx_args(args: &mut Vec<String>) {
    prefer_kvm_accel(args);
    if kvm_usable() {
        replace_cpu_arg(args, "host");
    } else {
        replace_cpu_arg(args, "max");
    }
}

fn append_esp_drive(args: &mut Vec<String>, esp_root: &Path) {
    let esp = esp_root.to_string_lossy().into_owned();
    args.push("-drive".into());
    args.push(format!("file=fat:rw:{esp},format=raw,if=none,id=esp"));
    args.push("-device".into());
    args.push("ide-hd,drive=esp,bus=ide.0".into());
}

fn find_qemu() -> Result<String, String> {
    for candidate in ["qemu-system-x86_64", "/usr/bin/qemu-system-x86_64"] {
        if Command::new(candidate).arg("--version").stdout(Stdio::null()).status().is_ok() {
            return Ok(candidate.into());
        }
    }
    Err("qemu-system-x86_64 not found (install qemu-system-x86)".into())
}

fn find_objcopy() -> Result<String, String> {
    for candidate in ["objcopy", "llvm-objcopy"] {
        if Command::new(candidate).arg("--version").stdout(Stdio::null()).status().is_ok() {
            return Ok(candidate.into());
        }
    }
    Err("objcopy not found (required to produce hypster.bin)".into())
}

fn find_ovmf(config: &Path) -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("HV_OVMF_CODE") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    let raw = read_yaml_file(config.to_str().ok_or("invalid config path")?)
        .map_err(|err| err.to_string())?;
    let compiled = compile_config(raw).map_err(|err| err.to_string())?;
    let configured = PathBuf::from(&compiled.intent.qemu.plan.ovmf);
    if configured.is_file() {
        return Ok(configured);
    }

    for candidate in [
        "/usr/share/OVMF/OVMF_CODE.fd",
        "/usr/share/OVMF/OVMF.fd",
        "/usr/share/qemu/OVMF.fd",
        "/opt/OVMF/OVMF_CODE.fd",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(format!(
        "OVMF firmware not found (set HV_OVMF_CODE or install ovmf; configured `{}`)",
        configured.display()
    ))
}

fn boot_until_marker(
    workspace: &Path,
    config: &Path,
    timeout: Duration,
    marker: &str,
    options: LaunchBootOptions,
) -> Result<(), String> {
    let qemu = match find_qemu() {
        Ok(path) => path,
        Err(err) => {
            println!("{err}; prepare succeeded, skipping QEMU boot");
            return Ok(());
        }
    };

    let ovmf = match find_ovmf(config) {
        Ok(path) => path,
        Err(err) => {
            println!("{err}; prepare succeeded, skipping QEMU boot");
            return Ok(());
        }
    };

    let mut args = assemble_qemu_args(workspace, config)?;
    replace_bios_arg(&mut args, &ovmf);
    if options.nested_vmx {
        enable_nested_vmx_args(&mut args);
    } else if !kvm_usable() {
        replace_cpu_arg(&mut args, "max");
    }
    args.extend(["-serial", "stdio", "-display", "none", "-no-reboot"].map(str::to_string));

    println!("booting `{}` until marker `{marker}`", qemu);
    let mut child = Command::new(&qemu)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|err| format!("failed to spawn `{qemu}`: {err}"))?;

    let stdout = child.stdout.take().ok_or("QEMU stdout unavailable")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    if tx.send(text).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let start = Instant::now();
    let mut captured = String::new();
    loop {
        if start.elapsed() > timeout {
            let _ = child.kill();
            return Err(format!(
                "timeout waiting for `{marker}` after {}s; serial output:\n{captured}",
                timeout.as_secs()
            ));
        }
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                println!("{line}");
                captured.push_str(&line);
                captured.push('\n');
                if captured.contains(marker) {
                    let _ = child.kill();
                    println!("marker `{marker}` observed");
                    return Ok(());
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if child.try_wait().ok().flatten().is_some() {
                    return Err(format!(
                        "QEMU exited before `{marker}`; serial output:\n{captured}"
                    ));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if child.try_wait().ok().flatten().is_some() {
                    return Err(format!(
                        "QEMU exited before `{marker}`; serial output:\n{captured}"
                    ));
                }
            }
        }
    }
}

fn run_cargo_with_rustflags(
    workspace: &Path,
    args: &[&str],
    rustflags: Option<&str>,
) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command.args(args).current_dir(workspace);
    if let Some(flags) = rustflags {
        command.env("RUSTFLAGS", flags);
    }
    let status = command.status().map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command `cargo {}` failed", args.join(" ")))
    }
}

fn run_cmd(program: &str, args: &[String]) -> Result<(), String> {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    run_cmd_in_dir(Path::new("."), program, &borrowed)
}

fn run_cmd_in_dir(current_dir: &Path, program: &str, args: &[&str]) -> Result<(), String> {
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

pub fn default_config_path(workspace: &Path) -> PathBuf {
    workspace.join(DEFAULT_CONFIG)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn parse_qemu_args_skips_comments_and_blank_lines() {
        let dir = std::env::temp_dir().join(format!("hypster-qemu-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("qemu-args.txt");
        fs::write(&path, "# comment\n-machine q35\n\n-cpu host\n").expect("write args");
        let args = parse_qemu_args_file(&path).expect("parse");
        assert_eq!(
            args,
            vec!["-machine".to_string(), "q35".to_string(), "-cpu".to_string(), "host".to_string(),]
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn append_esp_drive_adds_fat_directory() {
        let mut args = vec!["-machine q35".to_string()];
        append_esp_drive(&mut args, Path::new("/tmp/esp"));
        assert!(args.iter().any(|arg| arg.contains("file=fat:rw:/tmp/esp")));
        assert!(args.iter().any(|arg| arg.contains("ide-hd,drive=esp")));
    }
}
