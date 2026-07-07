use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct PioArduinoExtension {
    pio_path: Option<String>,
}

struct PioEnv {
    name: String,
    platform: Option<String>,
    board: Option<String>,
    framework: Vec<String>,
    lib_deps: Vec<String>,
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
        if self.pio_path.is_none() {
            self.pio_path = worktree
                .which("pio")
                .or_else(|| resolve_penv_pio());
        }

        let pio_path = match &self.pio_path {
            Some(p) => p.clone(),
            None => {
                return Err(
                    "pioarduino/PlatformIO Core not found. \
                     Open platformio.ini and click the ▶ run button next to any [env:...] \
                     section — the task will install it automatically via uv."
                        .to_string(),
                );
            }
        };

        let shell_env = worktree.shell_env();

        let config = fetch_project_config(&pio_path, &shell_env);

        if let Ok(ref envs) = config {
            let has_cc = envs.iter().any(|env| {
                let cc_path = format!(".pio/build/{}/compile_commands.json", env.name);
                worktree.read_text_file(&cc_path).is_ok()
            });
            if !has_cc {
                install_packages(&pio_path, &shell_env);
            }
        }

        let clangd_path = worktree.which("clangd").ok_or_else(|| {
            "clangd not found in PATH. Install clangd for C/C++ IntelliSense.".to_string()
        })?;

        let mut args = Vec::new();
        let root = worktree.root_path();

        if let Ok(ref envs) = config {
            for env in envs {
                let cc_path = format!(".pio/build/{}/compile_commands.json", env.name);
                if worktree.read_text_file(&cc_path).is_ok() {
                    filter_compile_commands(&cc_path);
                    args.push("--compile-commands-dir".to_string());
                    args.push(format!("{}/.pio/build/{}", root, env.name));
                    break;
                }
            }
        }

        if args.is_empty() {
            if let Ok(pio_ini) = worktree.read_text_file("platformio.ini") {
                let envs = extract_env_names(&pio_ini);
                for env in &envs {
                    let cc_path = format!(".pio/build/{}/compile_commands.json", env);
                    if worktree.read_text_file(&cc_path).is_ok() {
                        filter_compile_commands(&cc_path);
                        args.push("--compile-commands-dir".to_string());
                        args.push(format!("{}/.pio/build/{}", root, env));
                        break;
                    }
                }
            }
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

        let data = match section_arr[1].as_array() {
            Some(arr) => arr,
            None => continue,
        };

        let mut env = PioEnv {
            name: env_name.to_string(),
            platform: None,
            board: None,
            framework: Vec::new(),
            lib_deps: Vec::new(),
        };

        for entry in data {
            let entry_arr = match entry.as_array() {
                Some(arr) if arr.len() == 2 => arr,
                _ => continue,
            };

            let key = match entry_arr[0].as_str() {
                Some(k) => k,
                None => continue,
            };

            let val = &entry_arr[1];

            match key {
                "platform" => {
                    env.platform = val.as_str().map(String::from);
                }
                "board" => {
                    env.board = val.as_str().map(String::from);
                }
                "framework" => {
                    if let Some(arr) = val.as_array() {
                        env.framework =
                            arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    } else if let Some(s) = val.as_str() {
                        env.framework.push(s.to_string());
                    }
                }
                "lib_deps" => {
                    if let Some(arr) = val.as_array() {
                        env.lib_deps =
                            arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                }
                _ => {}
            }
        }

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

fn filter_compile_commands(path: &str) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut entries: Vec<zed::serde_json::Value> = match zed::serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return,
    };

    let xtensa_flags = [
        "-mno-target-align",
        "-mtext-section-literals",
        "-mlongcalls",
        "-mno-serialize-volatile",
        "-mtarget-align",
        "-mforce-no-pic",
        "-mconst16",
        "-mno-const16",
    ];

    for entry in &mut entries {
        let Some(command_val) = entry.get_mut("command") else { continue };
        let Some(command) = command_val.as_str() else { continue };

        let filtered: String = command
            .split(' ')
            .filter(|word| !xtensa_flags.contains(word))
            .collect::<Vec<_>>()
            .join(" ");

        if filtered != command {
            *command_val = zed::serde_json::Value::String(filtered);
        }
    }

    if let Ok(json) = zed::serde_json::to_string(&entries) {
        let _ = std::fs::write(path, json);
    }
}

fn resolve_penv_pio() -> Option<String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;

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
