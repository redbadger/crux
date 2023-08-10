use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub repository: Option<String>,
    pub cores: BTreeMap<String, Core>,
    pub shells: Option<BTreeMap<String, Shell>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Core {
    pub source: PathBuf,
    pub type_gen: PathBuf,
    pub crux_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shell {
    pub template: Option<PathBuf>,
    pub source: PathBuf,
    pub cores: Vec<String>,
}
