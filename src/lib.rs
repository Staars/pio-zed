use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct PioArduinoExtension {
    pio_path: Option<String>,
}

struct PioEnv {
    name: String,
}

impl zed::Extension for PioArduinoExtension {
    fn new() -> Self {
        PioArduinoExtension { pio_path: None }
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let shell_env = worktree.shell_env();

        if self.pio_path.is_none() {
            self.pio_path = worktree
                .which("pio")
                .or_else(|| resolve_penv_pio(&shell_env));
        }

        let pio_path = match &self.pio_path {
            Some(p) => p.clone(),
            None => {
                return Err("pioarduino/PlatformIO Core not found. \
                     Open platformio.ini and click the ▶ run button next to any [env:...] \
                     section — the task will install it automatically via uv."
                    .to_string());
            }
        };
        let root = worktree.root_path();

        let config = fetch_project_config(&pio_path, &shell_env);

        let clangd_path = worktree.which("clangd").ok_or_else(|| {
            "clangd not found in PATH. Install clangd for C/C++ IntelliSense.".to_string()
        })?;

        // Prefer a project-root database so clangd behaves exactly as it would
        // when launched directly and can apply the project's native .clangd file.
        let has_root_compile_commands = worktree.read_text_file("compile_commands.json").is_ok();
        let mut active_env = None;

        if !has_root_compile_commands {
            if let Ok(ref envs) = config {
                for env in envs {
                    let cc_path = format!(".pio/build/{}/compile_commands.json", env.name);
                    if worktree.read_text_file(&cc_path).is_ok() {
                        active_env = Some(env.name.clone());
                        break;
                    }
                }
            }
        }

        if !has_root_compile_commands && active_env.is_none() {
            if let Ok(pio_ini) = worktree.read_text_file("platformio.ini") {
                let envs = extract_env_names(&pio_ini);
                for env in &envs {
                    let cc_path = format!(".pio/build/{}/compile_commands.json", env);
                    if worktree.read_text_file(&cc_path).is_ok() {
                        active_env = Some(env.clone());
                        break;
                    }
                }
            }
        }

        if !has_root_compile_commands && active_env.is_none() {
            install_packages(&pio_path, &shell_env);
        }

        let mut args = Vec::new();
        if let Some(env_name) = active_env {
            args.push(format!(
                "--compile-commands-dir={}/.pio/build/{}",
                root, env_name
            ));
        }
        if has_root_compile_commands || !args.is_empty() {
            args.push("--query-driver=**/.platformio/packages/toolchain-*/bin/*".to_string());
        }

        Ok(zed::Command {
            command: clangd_path,
            args,
            env: shell_env,
        })
    }
}

fn fetch_project_config(pio_path: &str, shell_env: &[(String, String)]) -> Result<Vec<PioEnv>> {
    let mut cmd = zed::Command::new(pio_path);
    cmd.args = vec![
        "project".to_string(),
        "config".to_string(),
        "--json-output".to_string(),
    ];
    cmd.env = shell_env.to_vec();

    let output = cmd.output()?;
    if output.status != Some(0) {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let root: zed::serde_json::Value = zed::serde_json::from_str(&json_str)
        .map_err(|e| format!("failed to parse pio project config JSON: {}", e))?;

    let top_array = root
        .as_array()
        .ok_or_else(|| "expected top-level array".to_string())?;

    let mut envs = Vec::new();

    for section in top_array {
        let section_arr = match section.as_array() {
            Some(arr) if arr.len() == 2 => arr,
            _ => continue,
        };

        let section_name = match section_arr[0].as_str() {
            Some(s) => s,
            None => continue,
        };

        let env_name = match section_name.strip_prefix("env:") {
            Some(name) => name,
            None => continue,
        };

        let env = PioEnv {
            name: env_name.to_string(),
        };

        envs.push(env);
    }

    Ok(envs)
}

fn install_packages(pio_path: &str, shell_env: &[(String, String)]) {
    let mut cmd = zed::Command::new(pio_path);
    cmd.args = vec!["pkg".to_string(), "install".to_string()];
    cmd.env = shell_env.to_vec();
    let _ = cmd.output();
}

fn resolve_penv_pio(shell_env: &[(String, String)]) -> Option<String> {
    let home = shell_env
        .iter()
        .find(|(k, _)| k == "HOME" || k == "USERPROFILE")
        .map(|(_, v)| v.clone())
        .or_else(|| std::env::var("HOME").ok())
        .or_else(|| std::env::var("USERPROFILE").ok())?;

    let unix_path = format!("{}/.platformio/penv/bin/pio", home);
    if std::path::Path::new(&unix_path).exists() {
        return Some(unix_path);
    }

    let win_path = format!(r"{}\.platformio\penv\Scripts\pio.exe", home);
    if std::path::Path::new(&win_path).exists() {
        return Some(win_path);
    }

    None
}

fn extract_env_names(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if let Some(inner) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                inner.strip_prefix("env:").map(|name| name.to_string())
            } else {
                None
            }
        })
        .collect()
}

zed::register_extension!(PioArduinoExtension);
