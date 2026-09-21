//! Naming the FFI bindings the generated `Core` should bridge to.
//!
//! By default the generated shell API stops at [`CoreBridge`][bridge]: the
//! shell writes the handful of lines that carry bytes between `CoreBridge` and
//! whatever binding generator it uses. [`BoltFfi`] removes even that, for the
//! one binding generator Crux ships with — tell type generation the module,
//! package or namespace `BoltFFI` put `CoreFfi` in and it emits an `FfiBridge`
//! over it, plus a one-argument `Core` constructor that uses it.
//!
//! It is opt-in per language, because only the shell knows where the bindings
//! ended up: a language left unconfigured is generated exactly as it was
//! before.
//!
//! [bridge]: crate::type_generation::facet::CodeGenerator

use facet_generate::generation::PackageLocation;

/// The class `BoltFFI` exports when nothing else is said.
const DEFAULT_CLASS: &str = "CoreFfi";

/// Which `BoltFFI` bindings the generated `FfiBridge` should call, per language.
///
/// Build one with [`BoltFfi::new`] and name each language you want a bridge
/// for; see [`CodeGenerator::boltffi`](crate::type_generation::facet::CodeGenerator::boltffi)
/// for a worked example.
#[derive(Debug, Clone, Default)]
pub struct BoltFfi {
    class: Option<String>,
    swift: Option<SwiftFfi>,
    kotlin: Option<KotlinFfi>,
    typescript: Option<TypeScriptFfi>,
    csharp: Option<CSharpFfi>,
}

/// Where the Swift bindings live: a separate SPM package the generated one has
/// to depend on.
#[derive(Debug, Clone)]
pub struct SwiftFfi {
    /// The module `CoreFfi` is imported from, e.g. `Shared`.
    pub module: String,
    /// The library product that vends the module.
    pub product: String,
    /// The path to the package, relative to the generated one.
    pub package_path: String,
}

/// Where the Kotlin bindings live. `None` is the generated package itself,
/// which is how `boltffi pack android` lays things out when the two are
/// generated into the same source root.
#[derive(Debug, Clone)]
pub struct KotlinFfi {
    /// The package `CoreFfi` is declared in, or `None` for the generated one.
    pub package: Option<String>,
}

/// Where the TypeScript bindings live: an npm package the generated one has to
/// depend on.
#[derive(Debug, Clone)]
pub struct TypeScriptFfi {
    /// The npm package name, e.g. `shared`.
    pub package: String,
    /// Where npm should resolve it from.
    pub location: PackageLocation,
    /// The version range, for a published package. Ignored for a path.
    pub version: Option<String>,
}

/// Where the C# bindings live. `None` is the generated namespace itself.
#[derive(Debug, Clone)]
pub struct CSharpFfi {
    /// The namespace `CoreFfi` is declared in, or `None` for the generated one.
    pub namespace: Option<String>,
}

impl BoltFfi {
    /// A configuration that names no language yet — add one per language you
    /// want an `FfiBridge` for.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The class `BoltFFI` exports, if it is not called `CoreFfi`.
    ///
    /// The name is used in every language, since it is the Rust type's name
    /// that `BoltFFI` carries across.
    #[must_use]
    pub fn class(mut self, name: &str) -> Self {
        self.class = Some(name.to_string());
        self
    }

    /// Bridge Swift to `CoreFfi` in `module`, assuming the conventional
    /// layout: a package of the same name, one directory up from the generated
    /// one.
    #[must_use]
    pub fn swift(self, module: &str) -> Self {
        let package_path = format!("../{module}");
        self.swift_package(module, module, &package_path)
    }

    /// [`swift`](Self::swift), for a package that is laid out differently.
    #[must_use]
    pub fn swift_package(mut self, module: &str, product: &str, package_path: &str) -> Self {
        self.swift = Some(SwiftFfi {
            module: module.to_string(),
            product: product.to_string(),
            package_path: package_path.to_string(),
        });
        self
    }

    /// Bridge Kotlin to `CoreFfi` in the generated package.
    #[must_use]
    pub fn kotlin(mut self) -> Self {
        self.kotlin = Some(KotlinFfi { package: None });
        self
    }

    /// [`kotlin`](Self::kotlin), for bindings generated into another package.
    #[must_use]
    pub fn kotlin_package(mut self, package: &str) -> Self {
        self.kotlin = Some(KotlinFfi {
            package: Some(package.to_string()),
        });
        self
    }

    /// Bridge TypeScript to `CoreFfi` from the npm package `package`, which is
    /// added to the generated package's dependencies.
    #[must_use]
    pub fn typescript(self, package: &str, location: PackageLocation) -> Self {
        self.typescript_package(package, location, None)
    }

    /// [`typescript`](Self::typescript), pinning a published package to a
    /// version range.
    #[must_use]
    pub fn typescript_package(
        mut self,
        package: &str,
        location: PackageLocation,
        version: Option<&str>,
    ) -> Self {
        self.typescript = Some(TypeScriptFfi {
            package: package.to_string(),
            location,
            version: version.map(ToString::to_string),
        });
        self
    }

    /// Bridge C# to `CoreFfi` in the generated namespace.
    #[must_use]
    pub fn csharp(mut self) -> Self {
        self.csharp = Some(CSharpFfi { namespace: None });
        self
    }

    /// [`csharp`](Self::csharp), for bindings generated into another namespace.
    #[must_use]
    pub fn csharp_namespace(mut self, ns: &str) -> Self {
        self.csharp = Some(CSharpFfi {
            namespace: Some(ns.to_string()),
        });
        self
    }

    /// The name of the `BoltFFI` class, defaulting to `CoreFfi`.
    #[must_use]
    pub fn class_name(&self) -> &str {
        self.class.as_deref().unwrap_or(DEFAULT_CLASS)
    }

    /// The Swift bindings, if Swift was named.
    #[must_use]
    pub const fn swift_ffi(&self) -> Option<&SwiftFfi> {
        self.swift.as_ref()
    }

    /// The Kotlin bindings, if Kotlin was named.
    #[must_use]
    pub const fn kotlin_ffi(&self) -> Option<&KotlinFfi> {
        self.kotlin.as_ref()
    }

    /// The TypeScript bindings, if TypeScript was named.
    #[must_use]
    pub const fn typescript_ffi(&self) -> Option<&TypeScriptFfi> {
        self.typescript.as_ref()
    }

    /// The C# bindings, if C# was named.
    #[must_use]
    pub const fn csharp_ffi(&self) -> Option<&CSharpFfi> {
        self.csharp.as_ref()
    }
}

impl TypeScriptFfi {
    /// The `package.json` dependency pair the generated package needs, in the
    /// shape facet-generate's TypeScript installer merges — `"shared":
    /// "file:../pkg"`.
    #[must_use]
    pub fn manifest_dependency(&self) -> String {
        let version = match &self.location {
            PackageLocation::Path(path) => format!("file:{path}"),
            PackageLocation::Url(url) => self.version.clone().unwrap_or_else(|| url.clone()),
        };
        format!(r#""{}": "{version}""#, self.package)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_class_defaults_to_core_ffi() {
        assert_eq!(BoltFfi::new().class_name(), "CoreFfi");
        assert_eq!(BoltFfi::new().class("MyCore").class_name(), "MyCore");
    }

    #[test]
    fn swift_assumes_a_sibling_package_of_the_same_name() {
        let ffi = BoltFfi::new().swift("Shared");
        let swift = ffi.swift_ffi().expect("swift should be configured");

        assert_eq!(swift.module, "Shared");
        assert_eq!(swift.product, "Shared");
        assert_eq!(swift.package_path, "../Shared");
    }

    #[test]
    fn a_swift_package_can_be_spelled_out() {
        let ffi = BoltFfi::new().swift_package("Shared", "SharedLib", "../../ffi/Shared");
        let swift = ffi.swift_ffi().expect("swift should be configured");

        assert_eq!(swift.product, "SharedLib");
        assert_eq!(swift.package_path, "../../ffi/Shared");
    }

    #[test]
    fn kotlin_and_csharp_default_to_the_generated_package() {
        let ffi = BoltFfi::new().kotlin().csharp();

        assert_eq!(ffi.kotlin_ffi().expect("kotlin").package, None);
        assert_eq!(ffi.csharp_ffi().expect("csharp").namespace, None);

        let ffi = BoltFfi::new()
            .kotlin_package("com.example.ffi")
            .csharp_namespace("Example.Ffi");

        assert_eq!(
            ffi.kotlin_ffi().expect("kotlin").package.as_deref(),
            Some("com.example.ffi")
        );
        assert_eq!(
            ffi.csharp_ffi().expect("csharp").namespace.as_deref(),
            Some("Example.Ffi")
        );
    }

    #[test]
    fn a_typescript_path_becomes_a_file_dependency() {
        let ffi = BoltFfi::new().typescript("shared", PackageLocation::Path("../pkg".to_string()));

        assert_eq!(
            ffi.typescript_ffi()
                .expect("typescript")
                .manifest_dependency(),
            r#""shared": "file:../pkg""#
        );
    }

    #[test]
    fn a_published_typescript_package_uses_its_version() {
        let ffi = BoltFfi::new().typescript_package(
            "shared",
            PackageLocation::Url("https://npmjs.com/shared".to_string()),
            Some("^1.2.3"),
        );

        assert_eq!(
            ffi.typescript_ffi()
                .expect("typescript")
                .manifest_dependency(),
            r#""shared": "^1.2.3""#
        );
    }

    /// An unnamed language keeps today's output, so the absence has to survive
    /// the builder.
    #[test]
    fn a_language_that_is_not_named_stays_unconfigured() {
        let ffi = BoltFfi::new().swift("Shared");

        assert!(ffi.kotlin_ffi().is_none());
        assert!(ffi.typescript_ffi().is_none());
        assert!(ffi.csharp_ffi().is_none());
    }
}
