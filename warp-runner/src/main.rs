use anyhow::Context;
use anyhow::{anyhow, Result};
use log::{trace, Level};
use std::env;
use std::ffi::CStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

mod executor;
mod extractor;

static TARGET_FILE_NAME_BUF: &[u8] = b"tVQhhsFFlGGD3oWV4lEPST8I8FEPP54IM0q7daes4E1y3p2U2wlJRYmWmjPYfkhZ0PlT14Ls0j8fdDkoj33f2BlRJavLj3mWGibJsGt5uLAtrCDtvxikZ8UX2mQDCrgE\0";

fn target_file_name() -> &'static str {
    let nul_pos = TARGET_FILE_NAME_BUF
        .iter()
        .position(|elem| *elem == b'\0')
        .expect("TARGET_FILE_NAME_BUF has no NUL terminator");

    let slice = &TARGET_FILE_NAME_BUF[..=nul_pos];
    CStr::from_bytes_with_nul(slice)
        .expect("Can't convert TARGET_FILE_NAME_BUF slice to CStr")
        .to_str()
        .expect("Can't convert TARGET_FILE_NAME_BUF CStr to str")
}

fn cache_path(target: &str) -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .ok_or_else(|| anyhow!("No data local dir found"))?
        .join("warp")
        .join("packages")
        .join(target))
}

fn extract(exe_path: &Path, cache_path: &Path) -> Result<()> {
    if cache_path.exists() {
        fs::remove_dir_all(cache_path)
            .with_context(|| format!("Failed to remove directory {}", cache_path.display()))?;
    }
    extractor::extract_to(exe_path, cache_path).with_context(|| {
        format!(
            "Failed to extract {} to {}",
            exe_path.display(),
            cache_path.display()
        )
    })
}

fn main() -> Result<()> {
    if env::var("WARP_TRACE").is_ok() {
        simple_logger::init_with_level(Level::Trace)?;
    }

    let self_path = env::current_exe()?;
    let self_file_name = self_path.file_name().unwrap();
    let cache_path = cache_path(&self_file_name.to_string_lossy())?;

    trace!("self_path={:?}", self_path);
    trace!("self_file_name={:?}", self_file_name);
    trace!("cache_path={:?}", cache_path);

    let target_file_name = target_file_name();
    let target_path = cache_path.join(target_file_name);

    trace!("target_exec={:?}", target_file_name);
    trace!("target_path={:?}", target_path);

    match fs::metadata(&cache_path) {
        Ok(cache) => {
            if cache.modified()? >= fs::metadata(&self_path)?.modified()? {
                trace!("cache is up-to-date");
            } else {
                trace!("cache is outdated");
                extract(&self_path, &cache_path)?;
            }
        }
        Err(_) => {
            trace!("cache not found");
            extract(&self_path, &cache_path)?;
        }
    }

    let exit_code = executor::execute(&target_path)?;
    process::exit(exit_code);
}
