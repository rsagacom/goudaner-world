use std::path::PathBuf;

use host_adapter::{WebShellBootstrap, default_mobile_web_bootstrap};

#[derive(Debug)]
pub(crate) struct BootstrapPaths {
    pub(crate) state_dir: PathBuf,
    pub(crate) web_generated_dir: PathBuf,
}

fn current_dir_path() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| format!("resolve current directory failed: {error}"))
}

pub(crate) fn resolve_bootstrap_paths() -> Result<BootstrapPaths, String> {
    resolve_bootstrap_paths_with(current_dir_path)
}

fn resolve_bootstrap_paths_with<F>(mut current_dir: F) -> Result<BootstrapPaths, String>
where
    F: FnMut() -> Result<PathBuf, String>,
{
    let cwd = current_dir()?;
    let state_dir = std::env::var_os("LOBSTER_TUI_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| cwd.join(".lobster-chat-dev"));
    Ok(BootstrapPaths {
        state_dir,
        web_generated_dir: std::env::var_os("LOBSTER_WEB_GENERATED_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| cwd.join("apps/lobster-web-shell/generated")),
    })
}

pub(crate) fn default_mobile_bootstrap_from_env() -> WebShellBootstrap {
    let mut bootstrap = default_mobile_web_bootstrap();
    if let Ok(base_url) = std::env::var("LOBSTER_WAKU_GATEWAY_URL") {
        bootstrap.gateway_base_url = Some(base_url);
    }
    bootstrap
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn resolve_bootstrap_paths_uses_current_working_directory() {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var_os("LOBSTER_TUI_STATE_DIR");
        unsafe {
            std::env::remove_var("LOBSTER_TUI_STATE_DIR");
        }

        let paths =
            resolve_bootstrap_paths_with(|| Ok(PathBuf::from("/tmp/lobster-chat"))).unwrap();

        match previous {
            Some(value) => unsafe {
                std::env::set_var("LOBSTER_TUI_STATE_DIR", value);
            },
            None => unsafe {
                std::env::remove_var("LOBSTER_TUI_STATE_DIR");
            },
        }

        assert_eq!(
            paths.state_dir,
            PathBuf::from("/tmp/lobster-chat/.lobster-chat-dev")
        );
        assert_eq!(
            paths.web_generated_dir,
            PathBuf::from("/tmp/lobster-chat/apps/lobster-web-shell/generated")
        );
    }

    #[test]
    fn resolve_bootstrap_paths_surfaces_current_directory_errors() {
        let error = resolve_bootstrap_paths_with(|| Err("cwd unavailable".into())).unwrap_err();

        assert!(error.contains("cwd unavailable"));
    }

    #[test]
    fn resolve_bootstrap_paths_prefers_state_dir_env_override() {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var_os("LOBSTER_TUI_STATE_DIR");
        unsafe {
            std::env::set_var("LOBSTER_TUI_STATE_DIR", "/tmp/lobster-smoke-state");
        }

        let paths =
            resolve_bootstrap_paths_with(|| Ok(PathBuf::from("/tmp/lobster-chat"))).unwrap();

        match previous {
            Some(value) => unsafe {
                std::env::set_var("LOBSTER_TUI_STATE_DIR", value);
            },
            None => unsafe {
                std::env::remove_var("LOBSTER_TUI_STATE_DIR");
            },
        }

        assert_eq!(paths.state_dir, PathBuf::from("/tmp/lobster-smoke-state"));
        assert_eq!(
            paths.web_generated_dir,
            PathBuf::from("/tmp/lobster-chat/apps/lobster-web-shell/generated")
        );
    }

    #[test]
    fn resolve_bootstrap_paths_prefers_web_generated_dir_env_override() {
        let _guard = env_lock().lock().unwrap();
        let previous = std::env::var_os("LOBSTER_WEB_GENERATED_DIR");
        unsafe {
            std::env::set_var("LOBSTER_WEB_GENERATED_DIR", "/tmp/lobster-web-generated");
        }

        let paths =
            resolve_bootstrap_paths_with(|| Ok(PathBuf::from("/tmp/lobster-chat"))).unwrap();

        match previous {
            Some(value) => unsafe {
                std::env::set_var("LOBSTER_WEB_GENERATED_DIR", value);
            },
            None => unsafe {
                std::env::remove_var("LOBSTER_WEB_GENERATED_DIR");
            },
        }

        assert_eq!(
            paths.web_generated_dir,
            PathBuf::from("/tmp/lobster-web-generated")
        );
    }

    #[test]
    fn mobile_bootstrap_uses_gateway_env_when_present() {
        let previous = std::env::var_os("LOBSTER_WAKU_GATEWAY_URL");
        unsafe {
            std::env::set_var("LOBSTER_WAKU_GATEWAY_URL", "http://127.0.0.1:8790");
        }

        let bootstrap = default_mobile_bootstrap_from_env();

        match previous {
            Some(value) => unsafe {
                std::env::set_var("LOBSTER_WAKU_GATEWAY_URL", value);
            },
            None => unsafe {
                std::env::remove_var("LOBSTER_WAKU_GATEWAY_URL");
            },
        }

        assert_eq!(
            bootstrap.gateway_base_url.as_deref(),
            Some("http://127.0.0.1:8790")
        );
    }
}
