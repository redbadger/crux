use std::{fs, path::PathBuf};

use crate::config::Workspace;
use anyhow::{bail, Result};

const CONFIG_FILE: &str = "Crux.toml";

pub fn read_config() -> Result<Workspace> {
    let path = PathBuf::from(CONFIG_FILE);
    if let Ok(file) = &fs::read_to_string(path) {
        let workspace: Workspace = toml::from_str(file)?;
        let all_cores = workspace.cores.keys().cloned().collect::<Vec<_>>();
        for (name, core) in &workspace.cores {
            if !core.source.exists() {
                bail!(
                    "Crux.toml: core ({name}) source directory ({path}) does not exist",
                    path = core.source.display()
                );
            }
        }
        for (name, shell) in &workspace.shells {
            if !shell.source.exists() {
                bail!(
                    "Crux.toml: shell ({name}) source directory ({path}) does not exist",
                    path = shell.source.display()
                );
            }
            if !shell.cores.iter().all(|core| all_cores.contains(core)) {
                bail!("Crux.toml: shell ({name}) references a core that does not exist");
            }
        }
        Ok(workspace)
    } else {
        Ok(Workspace::default())
    }
}

pub fn write_config(workspace: &Workspace) -> Result<()> {
    let path = PathBuf::from(CONFIG_FILE);
    let toml = toml::to_string(workspace)?;
    fs::write(path, toml)?;
    Ok(())
}
