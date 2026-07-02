use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct PioArduinoExtension;

impl zed::Extension for PioArduinoExtension {
    fn new() -> Self {
        PioArduinoExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let clangd_path = worktree.which("clangd").ok_or_else(|| {
            "clangd not found in PATH. Install clangd for C/C++ IntelliSense.".to_string()
        })?;

        let mut args = Vec::new();

        if let Ok(pio_ini) = worktree.read_text_file("platformio.ini") {
            let envs = extract_env_names(&pio_ini);
            let root = worktree.root_path();

            for env in &envs {
                let cc_path = format!(".pio/build/{}/compile_commands.json", env);
                if worktree.read_text_file(&cc_path).is_ok() {
                    args.push("--compile-commands-dir".to_string());
                    args.push(format!("{}/.pio/build/{}", root, env));
                    break;
                }
            }
        }

        Ok(zed::Command {
            command: clangd_path,
            args,
            env: worktree.shell_env(),
        })
    }
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
