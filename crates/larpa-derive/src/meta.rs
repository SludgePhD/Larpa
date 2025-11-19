/// Metadata from `Cargo.toml`.
pub struct Metadata {
    pub pkg_version: Option<String>,
    pub pkg_authors: Option<String>,
    pub pkg_name: Option<String>,
    pub pkg_description: Option<String>,
    pub pkg_license: Option<String>,
    pub pkg_homepage: Option<String>,
    pub pkg_repository: Option<String>,
    pub bin_name: Option<String>,
}

impl Metadata {
    pub fn get() -> Self {
        Self {
            pkg_version: env("CARGO_PKG_VERSION"),
            pkg_authors: env("CARGO_PKG_AUTHORS"),
            pkg_name: env("CARGO_PKG_NAME"),
            pkg_description: env("CARGO_PKG_DESCRIPTION"),
            pkg_license: env("CARGO_PKG_LICENSE"),
            pkg_homepage: env("CARGO_PKG_HOMEPAGE"),
            pkg_repository: env("CARGO_PKG_REPOSITORY"),
            bin_name: env("CARGO_BIN_NAME"),
        }
    }

    /// Returns the canonical executable name of the crate being compiled.
    ///
    /// This is used as the canonical name of any commands defined in this crate, unless a custom
    /// name is specified.
    ///
    /// It defaults to the name of the produced binary (for crates that produce a binary), or to
    /// the Cargo package name in other cases.
    ///
    /// Returns [`None`] if Cargo isn't being used and the respective environment variables are
    /// missing.
    /// In that case, manually specifying an executable name is required.
    pub fn canonical_name(&self) -> Option<&str> {
        self.bin_name
            .as_deref()
            .or_else(|| self.pkg_name.as_deref())
    }
}

fn env(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(str) if !str.is_empty() => Some(str),
        _ => None,
    }
}
