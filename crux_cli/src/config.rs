use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) authors: Vec<String>,
    pub(crate) repository: Option<String>,
    pub(crate) cores: BTreeMap<String, Core>,
    pub(crate) shells: BTreeMap<String, Shell>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Core {
    pub(crate) source: PathBuf,
    pub(crate) type_gen: PathBuf,
    pub(crate) capability_crates: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shell {
    pub(crate) template: Option<PathBuf>,
    pub(crate) source: PathBuf,
    pub(crate) cores: Vec<String>,
}
