//! Generation of foreign language types (currently Swift, Java, TypeScript) for Crux
//!
//! In order to use this module, you'll need a separate crate from your shared library, possibly
//! called `shared_types`. This is necessary because we need to reference types from your shared library
//! during the build process (`build.rs`).
//!
//! This module is behind the feature called `facet_typegen`, and is not compiled into the default crate.
//!
//! Ensure that you have the following line in the `Cargo.toml` of your `shared_types` library.
//!
//! ```rust,ignore
//! [build-dependencies]
//! crux_core = { version = "0.7", features = ["facet_typegen"] }
//! ```
//!
//! * Your `shared_types` library, will have an empty `lib.rs`, since we only use it for generating foreign language type declarations.
//! * Create a `build.rs` in your `shared_types` library, that looks something like this:
//!
//! ```rust
//! # mod shared {
//! #     use crux_core::Command;
//! #     use crux_core::render::Render;
//! #     use crux_core::macros::Effect;
//! #     use facet::Facet;
//! #     use serde::{Deserialize, Serialize};
//! #     #[derive(Default)]
//! #     pub struct App;
//! #     #[derive(Facet, Serialize, Deserialize)]
//! #     #[repr(C)]
//! #     pub enum Event {
//! #         None,
//! #     }
//! #     #[derive(Facet, Serialize, Deserialize)]
//! #     pub struct ViewModel;
//! #     impl crux_core::App for App {
//! #         type Event = Event;
//! #         type Model = ();
//! #         type ViewModel = ViewModel;
//! #         type Capabilities = Capabilities;
//! #         type Effect = Effect;
//! #         fn update(&self, _event: Event, _model: &mut Self::Model, _caps: &Capabilities) -> Command<Effect, Event> {
//! #             todo!()
//! #         }
//! #         fn view(&self, _model: &Self::Model) -> Self::ViewModel {
//! #             todo!();
//! #         }
//! #     }
//! #     #[derive(Effect)]
//! #     pub struct Capabilities {
//! #         pub render: Render<Event>,
//! #     }
//! # }
//!use shared::{App, EffectFfi, Event};
//!use crux_core::{bridge::Request, type_generation::facet::TypeGen};
//!use uuid::Uuid;
//!
//!#[test]
//!fn generate_types() -> anyhow::Result<()> {
//!    let mut typegen = TypeGen::new();
//!
//!    typegen.register_app::<App>()?;
//!
//!    let temp = assert_fs::TempDir::new()?;
//!    let output_root = temp.join("crux_core_typegen_test");
//!
//!    typegen.swift("SharedTypes", output_root.join("swift"))?;
//!
//!    typegen.java("com.example.counter.shared_types", output_root.join("java"))?;
//!
//!    typegen.typescript("shared_types", output_root.join("typescript"))?;
//!}
//! ```
//!
//! ## Custom extensions
//!
//! If you need to use customized files for one of:
//!
//! - `generated/typescript/*`,
//! - `generated/swift/(requests | Package).swift` -
//! - `generated/java/Requests.java`
//!
//! Then create the `typegen_extensions/{target}/{target-file}`
//! with the desired content next to your `build.rs` file.
//!
//! For example `typegen_extensions/swift/Package.swift`:
//!
//! ```swift
//! // swift-tools-version: 5.7.1
//! // The swift-tools-version declares the minimum version of Swift required to build this package.
//!
//! import PackageDescription
//!
//! let package = Package(
//!     name: "SharedTypes",
//!     products: [
//!         // Products define the executables and libraries a package produces, and make them visible to other packages.
//!         .library(
//!             name: "SharedTypes",
//!             targets: ["SharedTypes"]),
//!     ],
//!     dependencies: [
//!         // Dependencies declare other packages that this package depends on.
//!         // .package(url: /* package url */, from: "1.0.0"),
//!     ],
//!     targets: [
//!         // Targets are the basic building blocks of a package. A target can define a module or a test suite.
//!         // Targets can depend on other targets in this package, and on products in packages this package depends on.
//!         .target(
//!             name: "Serde",
//!             dependencies: []),
//!         .target(
//!             name: "SharedTypes",
//!             dependencies: ["Serde"]),
//!     ]
//! )
//! ```

use facet::Facet;
use facet_generate::{
    Registry,
    namespace::Namespace,
    serde_generate::{Encoding, SourceInstaller, java, swift, typescript},
};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Write,
    mem,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::App;

pub type Result = std::result::Result<(), TypeGenError>;

#[derive(Error, Debug)]
pub enum TypeGenError {
    #[error("code has been generated, too late to register types")]
    LateRegistration,
    #[error("type generation failed: {0}")]
    Generation(String),
    #[error("error writing generated types")]
    Io(#[from] std::io::Error),
    #[error(
        "`pnpm` is needed for TypeScript type generation, but it could not be found in PATH.\nPlease install it from https://pnpm.io/installation"
    )]
    PnpmNotFound(#[source] std::io::Error),
}

#[derive(Debug)]
pub enum State {
    Registering(Registry),
    Generating(Registry),
}

pub trait Export {
    /// Register types with the type generator.
    ///
    /// This method should be called before any types are registered.
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if the type tracing fails.
    fn register_types(generator: &mut TypeGen) -> Result;
}

impl Export for () {
    fn register_types(_generator: &mut TypeGen) -> Result {
        Ok(())
    }
}

/// The `TypeGen` struct stores the registered types so that they can be generated for foreign languages
/// use `TypeGen::new()` to create an instance
pub struct TypeGen {
    pub state: State,
}

impl Default for TypeGen {
    fn default() -> Self {
        TypeGen {
            state: State::Registering(Registry::default()),
        }
    }
}

impl TypeGen {
    /// Creates an instance of the `TypeGen` struct
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register all the types used in app `A` to be shared with the Shell.
    ///
    /// Do this before calling `TypeGen::swift`, `TypeGen::java` or `TypeGen::typescript`.
    /// This method would normally be called in a build.rs file of a sister crate responsible for
    /// creating "foreign language" type definitions for the FFI boundary.
    /// See the section on
    /// [creating the shared types crate](https://redbadger.github.io/crux/getting_started/core.html#create-the-shared-types-crate)
    /// in the Crux book for more information.
    ///
    /// # Errors
    ///
    /// If any of the types used in the app cannot be registered, an error will be returned.
    pub fn register_app<'a, A: App>(&mut self) -> Result
    where
        A::Effect: Export,
        A::Event: Deserialize<'static> + Facet<'a>,
        A::ViewModel: Deserialize<'static> + Facet<'a> + 'static,
    {
        A::Effect::register_types(self)?;
        self.register_type::<A::Event>()?;
        self.register_type::<A::ViewModel>()?;

        Ok(())
    }

    /// For each of the types that you want to share with the Shell, call this method:
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeGen;
    /// # use serde::{Serialize, Deserialize};
    /// # use anyhow::Error;
    /// #[derive(facet::Facet, Serialize, Deserialize)]
    /// #[repr(C)]
    /// enum MyNestedEnum { None }
    /// #[derive(facet::Facet, Serialize, Deserialize)]
    /// #[repr(C)]
    /// enum MyEnum { None, Nested(MyNestedEnum) }
    /// fn register() -> Result<(), Error> {
    ///   let mut typegen = TypeGen::new();
    ///   typegen.register_type::<MyEnum>()?;
    ///   typegen.register_type::<MyNestedEnum>()?;
    ///   Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Errors that can occur during type registration.
    pub fn register_type<'a, 'de, T>(&mut self) -> Result
    where
        T: serde::Deserialize<'de> + Facet<'a>,
    {
        match &mut self.state {
            State::Registering(registry) => {
                let incoming = facet_generate::reflect::<T>();
                registry.extend(Registry::from(incoming));
                Ok(())
            }
            State::Generating(_) => Err(TypeGenError::LateRegistration),
        }
    }

    /// Generates types for Swift
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.swift("SharedTypes", output_root.join("swift"))?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    ///
    /// # Panics
    /// Panics if the registry creation fails.
    pub fn swift(&mut self, package_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry();

        let path = path.as_ref().join(package_name);

        fs::create_dir_all(&path)?;

        let mut installer = swift::Installer::new(package_name.to_string(), path.clone());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let State::Generating(ref registry) = self.state else {
            panic!("registry creation failed");
        };

        let root_module = package_name;
        for (module, registry) in facet_generate::namespace::split(root_module, registry.clone())
            .map_err(|e| TypeGenError::Generation(e.to_string()))?
        {
            let config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            installer
                .install_module(&config, &registry)
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        // add bincode deserialization for Vec<Request>
        let mut output = File::create(
            path.join("Sources")
                .join(package_name)
                .join("Requests.swift"),
        )?;

        let requests_path = Self::extensions_path("swift/requests.swift");
        let requests_data = fs::read_to_string(requests_path)?;
        write!(output, "{requests_data}")?;

        installer
            .install_manifest(package_name)
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        Ok(())
    }

    /// Generates types for Java (for use with Kotlin)
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.java(
    ///     "com.redbadger.crux_core.shared_types",
    ///     output_root.join("java"),
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    ///
    /// # Panics
    /// Panics if the registry creation fails.
    pub fn java(&mut self, package_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry();

        fs::create_dir_all(&path)?;

        let package_path = package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(path.as_ref().join(&package_path)).unwrap_or(());

        let mut installer = java::Installer::new(path.as_ref().to_path_buf());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let State::Generating(ref mut registry) = self.state else {
            panic!("registry creation failed");
        };

        let root_module = package_name;
        for (module, registry) in facet_generate::namespace::split(root_module, registry.clone())
            .map_err(|e| TypeGenError::Generation(e.to_string()))?
        {
            let this_module = &module.config().module_name;
            let module = if root_module == this_module {
                module
            } else {
                Namespace::new([root_module, this_module].join("."))
            };

            let config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            installer
                .install_module(&config, &registry)
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        let requests_path = Self::extensions_path("java/Requests.java");

        let requests_data = fs::read_to_string(requests_path)?;

        let requests = format!("package {package_name};\n\n{requests_data}");

        fs::write(
            path.as_ref()
                .to_path_buf()
                .join(package_path)
                .join("Requests.java"),
            requests,
        )?;

        Ok(())
    }

    /// Generates types for TypeScript
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.typescript("shared_types", output_root.join("typescript"))?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    /// # Errors
    /// Errors that can occur during type generation.
    ///
    /// # Panics
    /// Panics if the registry creation fails.
    pub fn typescript(&mut self, package_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry();

        fs::create_dir_all(&path)?;
        let output_dir = path.as_ref().to_path_buf();

        let types_dir = output_dir.join("types");
        fs::create_dir_all(&types_dir)?;

        let installer = typescript::Installer::new(output_dir.clone());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let State::Generating(ref mut registry) = self.state else {
            panic!("registry creation failed");
        };

        let root_module = package_name;
        for (module, registry) in facet_generate::namespace::split(root_module, registry.clone())
            .map_err(|e| TypeGenError::Generation(e.to_string()))?
        {
            let config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            let module_name = config.module_name();

            let generator = typescript::CodeGenerator::new(&config);
            let mut source = Vec::new();
            generator.output(&mut source, &registry)?;

            // FIXME fix import paths in generated code which assume running on Deno
            let out = String::from_utf8_lossy(&source)
                .replace(
                    "import { BcsSerializer, BcsDeserializer } from '../bcs/mod.ts';",
                    "",
                )
                .replace(".ts'", "'");

            let mut output = File::create(types_dir.join(format!("{module_name}.ts")))?;
            write!(output, "{out}")?;
        }

        let extensions_dir = Self::extensions_path("typescript");
        copy(extensions_dir, path)?;

        // Install dependencies
        std::process::Command::new("pnpm")
            .current_dir(output_dir.clone())
            .arg("install")
            .status()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => TypeGenError::PnpmNotFound(e),
                _ => TypeGenError::Io(e),
            })?;

        // Build TS code and emit declarations
        std::process::Command::new("pnpm")
            .current_dir(output_dir)
            .arg("exec")
            .arg("tsc")
            .arg("--build")
            .status()
            .map_err(TypeGenError::Io)?;

        Ok(())
    }

    fn ensure_registry(&mut self) {
        if let State::Registering(_) = self.state {
            // replace the current state
            let old_state = mem::replace(&mut self.state, State::Registering(Registry::new()));

            // move the registry
            if let State::Registering(registry) = old_state {
                // replace dummy with registry
                self.state = State::Generating(registry);
            }
        }
    }

    fn extensions_path(path: &str) -> PathBuf {
        let custom = PathBuf::from("./typegen_extensions").join(path);
        let default = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("typegen_extensions")
            .join(path);

        match custom.try_exists() {
            Ok(true) => custom,
            Ok(false) => default,
            Err(e) => {
                println!("cant check typegen extensions override: {e}");
                default
            }
        }
    }
}

fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result {
    fs::create_dir_all(to.as_ref())?;

    let entries = fs::read_dir(from)?;
    for entry in entries {
        let entry = entry?;

        let to = to.as_ref().to_path_buf().join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(entry.path(), to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }

    Ok(())
}
