//! System information utilities.
//! This module provides functionality to gather and represent
//! system-related information such as current directory, home directory,
//! temporary directory, username, hostname, shell, operating system, and architecture.

use std::env;
use std::path::PathBuf;

use crate::SystemError;

#[derive(Debug)]
pub struct SystemInfo {
    pub current_dir: PathBuf,
    pub home_dir: PathBuf,
    pub temp_dir: PathBuf,

    pub username: String,
    pub hostname: String,
    pub shell: String,

    pub os: String,
    pub arch: String,

    pub total_ram_kb: u64,
    pub used_ram_kb: u64,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub gpu_names: Vec<String>,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemInfo {
    #[must_use]
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|_| Self::fallback())
    }

    pub fn try_new() -> Result<Self, SystemError> {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();

        Ok(Self {
            current_dir: env::current_dir()?,

            home_dir: PathBuf::from(get_env_any(
                &["HOME", "USERPROFILE"],
                "HOME/USERPROFILE",
            )?),

            temp_dir: env::temp_dir(),

            username: get_env_any(&["USER", "USERNAME"], "USER/USERNAME")?,
            hostname: get_env_any(
                &["HOSTNAME", "COMPUTERNAME"],
                "HOSTNAME/COMPUTERNAME",
            )
            .unwrap_or_else(|_| "unknown_host".into()),

            shell: get_env_any(&["SHELL", "ComSpec"], "SHELL/ComSpec")
                .unwrap_or_else(|_| "unknown_shell".into()),

            os: env::consts::OS.into(),
            arch: env::consts::ARCH.into(),

            total_ram_kb: sys.total_memory(),
            used_ram_kb: sys.used_memory(),
            cpu_brand: sys.cpus().first().map_or_else(
                || "unknown_cpu".into(),
                |cpu| cpu.brand().to_string(),
            ),
            cpu_cores: sys.cpus().len(),
            gpu_names: get_gpus(),
        })
    }

    fn fallback() -> Self {
        Self {
            current_dir: PathBuf::from("."),
            home_dir: PathBuf::new(),
            temp_dir: env::temp_dir(),

            username: "unknown_user".into(),
            hostname: "unknown_host".into(),
            shell: "unknown_shell".into(),

            os: env::consts::OS.into(),
            arch: env::consts::ARCH.into(),
            total_ram_kb: 0,
            used_ram_kb: 0,
            cpu_brand: "unknown_cpu".into(),
            cpu_cores: 0,
            gpu_names: vec!["unknown".to_string()],
        }
    }

    #[must_use]
    pub fn is_windows(&self) -> bool {
        self.os == "windows"
    }

    #[must_use]
    pub fn is_unix(&self) -> bool {
        matches!(self.os.as_str(), "linux" | "macos")
    }

    pub fn print(&self) {
        let usage_pct = if self.total_ram_kb == 0 {
            0.0
        } else {
            (self.used_ram_kb as f64 / self.total_ram_kb as f64) * 100.0
        };

        println!(
            "\
System Information
------------------
OS           : {} ({})
User         : {}
Host         : {}
Shell        : {}
Working dir  : {}
Home dir     : {}
Temp dir     : {}
Total RAM    : {:.2} KB
Used RAM     : {:.2} KB
RAM Usage    : {:.2} %
CPU Brand    : {}
CPU Cores    : {}
GPUs         : {}
",
            self.os,
            self.arch,
            self.username,
            self.hostname,
            self.shell,
            self.current_dir.display(),
            self.home_dir.display(),
            self.temp_dir.display(),
            self.total_ram_kb,
            self.used_ram_kb,
            usage_pct,
            self.cpu_brand,
            self.cpu_cores,
            self.gpu_names.join(", ")
        );
    }
}

#[cfg(target_os = "linux")]
fn get_gpus() -> Vec<String> {
    use std::process::Command;

    let output = Command::new("lspci").arg("-nn").output();
    let output = match output {
        Ok(o) => o,
        Err(_) => return vec!["unknown".into()],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();

    for line in stdout.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains("vga") || line_lower.contains("3d") {
            if let Some(pos) = line.find(": ") {
                let desc = line[pos + 2..].trim();
                let desc_clean = desc.trim().to_string();
                gpus.push(desc_clean);
            }
        }
    }

    if gpus.is_empty() { vec!["unknown".into()] } else { gpus }
}

#[cfg(target_os = "windows")]
fn get_gpus() -> Vec<String> {
    use std::process::Command;

    let output = match Command::new("powershell")
        .arg("-Command")
        .arg("Get-WmiObject Win32_VideoController | Select-Object Name")
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec!["unknown".into()],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .skip(2)
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(target_os = "macos")]
fn get_gpus() -> Vec<String> {
    use std::process::Command;

    let output = match Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec!["unknown".into()],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|line| line.trim_start().starts_with("Chipset Model:"))
        .map(|line| line.replace("Chipset Model:", "").trim().to_string())
        .collect()
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos"
)))]
fn get_gpus() -> Vec<String> {
    vec!["unknown".into()]
}

fn get_env_any(
    keys: &[&str],
    label: &'static str,
) -> Result<String, SystemError> {
    for &key in keys {
        if let Ok(val) = env::var(key) {
            return Ok(val);
        }
    }
    Err(SystemError::MissingEnv(label))
}
