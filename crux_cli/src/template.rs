use ramhorns::Content;

use crate::config::{Core, Shell, Workspace};

pub enum Context {
    Core(CoreContext),
    Shell(ShellContext),
}

#[derive(Content)]
pub struct CoreContext {
    pub workspace: String,
    pub core_name: String,
    pub core_name_dashes: String,
}

impl CoreContext {
    pub fn new(workspace: &Workspace, core: &Core) -> Context {
        Context::Core(Self {
            workspace: workspace.name.to_ascii_lowercase().replace(" ", "_"),
            core_name: core.name.clone(),
            core_name_dashes: core.name.replace("_", "-"),
        })
    }
}

#[derive(Content)]
pub struct ShellContext {
    pub workspace: String,
    pub core_dir: String,
    pub core_name: String,
    pub type_gen: String,
    pub shell_dir: String,
    pub shell_name: String,
    pub shell_name_dashes: String,
}

impl ShellContext {
    pub fn new(workspace: &Workspace, core: &Core, shell: &Shell) -> Context {
        Context::Shell(Self {
            workspace: workspace.name.to_ascii_lowercase().replace(" ", "_"),
            core_dir: core.source.to_string_lossy().to_string(),
            core_name: core.name.replace("-", "_"),
            type_gen: core.type_gen.to_string_lossy().to_string(),
            shell_dir: shell.source.to_string_lossy().to_string(),
            shell_name: shell.name.replace("-", "_"),
            shell_name_dashes: shell.name.replace("_", "-"),
        })
    }
}
