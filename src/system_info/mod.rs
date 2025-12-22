mod cpu;
mod env;
pub mod error;
mod gpu;

use crate::system_info::env::get_env_any;
use crate::system_info::error::SystemError;
use crate::system_info::gpu::get_gpus;

use std::env as std_env;
use std::path::PathBuf;

#[derive(Debug)]
pub struct SystemInfo {
    pub current_dir: PathBuf,
    pub home_dir: PathBuf,
    pub temp_dir: PathBuf,

    pub username: String,
    pub hostname: String,
    pub shell: String,
    pub os_arch: String,

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
            current_dir: std_env::current_dir()?,

            home_dir: PathBuf::from(get_env_any(
                &["HOME", "USERPROFILE"],
                "HOME/USERPROFILE",
            )?),

            temp_dir: std_env::temp_dir(),

            username: get_env_any(&["USER", "USERNAME"], "USER/USERNAME")?,
            hostname: get_env_any(
                &["HOSTNAME", "COMPUTERNAME"],
                "HOSTNAME/COMPUTERNAME",
            )
            .unwrap_or_else(|_| "unknown_host".into()),

            shell: get_env_any(&["SHELL", "ComSpec"], "SHELL/ComSpec")
                .unwrap_or_else(|_| "unknown_shell".into()),

            os_arch: format!("{} {}", std_env::consts::OS, std_env::consts::ARCH),

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
            temp_dir: std_env::temp_dir(),

            username: "unknown_user".into(),
            hostname: "unknown_host".into(),
            shell: "unknown_shell".into(),

            os_arch: format!("{} {}", std_env::consts::OS, std_env::consts::ARCH),
            total_ram_kb: 0,
            used_ram_kb: 0,
            cpu_brand: "unknown_cpu".into(),
            cpu_cores: 0,
            gpu_names: vec!["unknown".to_string()],
        }
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
OS           : {}
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
            self.os_arch,
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
