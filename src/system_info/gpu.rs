#[cfg(target_os = "linux")]
pub(crate) fn get_gpus() -> Vec<String> {
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
pub(crate) fn get_gpus() -> Vec<String> {
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
pub(crate) fn get_gpus() -> Vec<String> {
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
pub(crate) fn get_gpus() -> Vec<String> {
    vec!["unknown".into()]
}
