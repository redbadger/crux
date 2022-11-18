use anyhow::{anyhow, bail, Result};
use serde_generate::test_utils::Runtime;
use serde_reflection::{Registry, Tracer, TracerConfig};
use std::{
    fs::{self, File},
    io::Write,
    mem,
    path::{Path, PathBuf},
};

enum State {
    Tracer(Tracer),
    Registry(Registry),
}

pub struct TypeGen {
    state: State,
}

impl Default for TypeGen {
    fn default() -> Self {
        TypeGen {
            state: State::Tracer(Tracer::new(TracerConfig::default())),
        }
    }
}

impl TypeGen {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register_type<'de, T>(&mut self) -> Result<()>
    where
        T: serde::Deserialize<'de>,
    {
        match &mut self.state {
            State::Tracer(tracer) => match tracer.trace_simple_type::<T>() {
                Ok(_) => Ok(()),
                Err(e) => bail!("type tracing failed: {}", e),
            },
            _ => bail!("code has been generated, too late to register types"),
        }
    }

    pub fn swift(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;

        let mut source = Vec::new();
        let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
            .with_encodings(vec![serde_generate::Encoding::Bcs]);

        let generator = serde_generate::swift::CodeGenerator::new(&config);
        let registry = match &self.state {
            State::Registry(registry) => registry,
            _ => panic!("registry creation failed"),
        };

        generator.output(&mut source, registry)?;

        // FIXME workaround for odd namespacing behaviour in Swift output
        // which as far as I can tell does not support namespaces in this way
        let out = String::from_utf8_lossy(&source).replace("shared.", "");

        let out = format!(
            "{out}\n\n{}",
            include_str!("../typegen_extensions/swift/requests.swift")
        );

        let path = path.as_ref().to_path_buf().join("shared_types.swift");
        let mut output = File::create(path)?;
        write!(output, "{}", out)?;

        Ok(())
    }

    pub fn java(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;

        let config =
            serde_generate::CodeGeneratorConfig::new("com.redbadger.rmm.shared_types".to_string())
                .with_encodings(vec![serde_generate::Encoding::Bcs]);

        let registry = match &self.state {
            State::Registry(registry) => registry,
            _ => panic!("registry creation failed"),
        };

        let generator = serde_generate::java::CodeGenerator::new(&config);
        generator.write_source_files(path.as_ref().to_path_buf(), registry)?;

        let extensions_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("typegen_extensions/java");

        fs::copy(
            extensions_dir.join("com/redbadger/rmm/shared_types/Requests.java"),
            path.as_ref()
                .to_path_buf()
                .join("com/redbadger/rmm/shared_types/Requests.java"),
        )?;

        Ok(())
    }

    pub fn typescript(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;
        let output_dir = path.as_ref().to_path_buf();

        let extensions_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("typegen_extensions/typescript");
        let mut source = Vec::new();

        // FIXME this should be the actual route, but the runtime is built
        // for Deno, so we patch it heavily in extensions:
        //
        // let installer = typescript::Installer::new(output_dir.clone());
        // installer.install_serde_runtime()?;
        // installer.install_bcs_runtime()?;
        copy(extensions_dir, path).expect("Could not copy TS runtime");

        let registry = match &self.state {
            State::Registry(registry) => registry,
            _ => panic!("registry creation failed"),
        };

        let runtime = Runtime::Bcs;
        let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
            .with_serialization(true)
            .with_encodings(vec![runtime.into()]);

        let generator = serde_generate::typescript::CodeGenerator::new(&config);
        generator.output(&mut source, registry)?;
        // FIXME fix import paths in generated code which assume running on Deno
        let out = String::from_utf8_lossy(&source).replace(".ts'", "'");

        let types_dir = output_dir.join("types");
        fs::create_dir_all(types_dir)?;

        let mut output = File::create(output_dir.join("types/shared.ts"))?;
        write!(output, "{}", out)?;

        // Install dependencies
        std::process::Command::new("pnpm")
            .current_dir(output_dir.clone())
            .arg("install")
            .status()
            .expect("Could not pnpm install");

        // Build TS code and emit declarations
        std::process::Command::new("pnpm")
            .current_dir(output_dir)
            .arg("exec")
            .arg("tsc")
            .arg("--build")
            .status()
            .expect("Could tsc --build");

        Ok(())
    }

    fn ensure_registry(&mut self) -> Result<()> {
        if let State::Tracer(_) = self.state {
            // replace the current state with a dummy tracer
            let old_state = mem::replace(
                &mut self.state,
                State::Tracer(Tracer::new(TracerConfig::default())),
            );

            // convert tracer to registry
            if let State::Tracer(tracer) = old_state {
                // replace dummy with registry
                self.state = State::Registry(tracer.registry().map_err(|e| anyhow!("{e}"))?);
            }
        }
        Ok(())
    }
}

fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(to.as_ref())?;

    let entries = fs::read_dir(from)?;
    for entry in entries {
        let entry = entry?;

        let to = to.as_ref().to_path_buf().join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(entry.path(), to)?;
        } else {
            fs::copy(entry.path(), to)?;
        };
    }

    Ok(())
}
