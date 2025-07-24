/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::ffi::OsStr;
use std::fmt;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use itertools::Itertools;
use pyrefly_python::module_name::ModuleName;
use pyrefly_python::module_path::ModulePath;
use pyrefly_python::sys_info::PythonPlatform;
use pyrefly_python::sys_info::PythonVersion;
use pyrefly_python::sys_info::SysInfo;
use pyrefly_util::absolutize::Absolutize as _;
use pyrefly_util::fs_anyhow;
use pyrefly_util::globs::FilteredGlobs;
use pyrefly_util::globs::Glob;
use pyrefly_util::globs::Globs;
use pyrefly_util::prelude::VecExt;
use serde::Deserialize;
use serde::Serialize;
use starlark_map::small_map::SmallMap;
use tracing::debug;

use crate::base::ConfigBase;
use crate::base::UntypedDefBehavior;
use crate::environment::environment::PythonEnvironment;
use crate::environment::interpreters::Interpreters;
use crate::error::ErrorConfig;
use crate::error::ErrorDisplayConfig;
use crate::finder::ConfigError;
use crate::pyproject::PyProject;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct SubConfig {
    pub matches: Glob,
    #[serde(flatten)]
    pub settings: ConfigBase,
}

impl SubConfig {
    fn rewrite_with_path_to_config(&mut self, config_root: &Path) {
        self.matches = self.matches.clone().from_root(config_root);
    }
}

/// Where did this config come from?
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum ConfigSource {
    /// This config was read from a file
    File(PathBuf),
    /// This config was synthesized with path-specific defaults, based on the location of a
    /// "marker" file that contains no pyrefly configuration but marks a project root (e.g., a
    /// `pyproject.toml` file with no `[tool.pyrefly]` section)
    Marker(PathBuf),
    #[default]
    Synthetic,
}

impl ConfigSource {
    pub fn root(&self) -> Option<&Path> {
        match &self {
            Self::File(path) | Self::Marker(path) => path.parent(),
            Self::Synthetic => None,
        }
    }
}

/// Where the importable Python code in a project lives. There are two common Python project layouts, src and flat.
/// See: https://packaging.python.org/en/latest/discussions/src-layout-vs-flat-layout/#src-layout-vs-flat-layout
#[derive(Default)]
pub enum ProjectLayout {
    /// Python packages live directly in the project root
    #[default]
    Flat,
    /// Python packages live in a src/ subdirectory
    Src,
}

impl ProjectLayout {
    pub fn new(project_root: &Path) -> Self {
        let src_subdir = project_root.join("src");
        match src_subdir.try_exists() {
            Ok(true) => Self::Src,
            Ok(false) => Self::Flat,
            Err(e) => {
                debug!(
                    "Error checking for existence of path {}: {}",
                    src_subdir.display(),
                    e
                );
                Self::default()
            }
        }
    }

    fn get_import_root(&self, project_root: &Path) -> PathBuf {
        match self {
            Self::Flat => project_root.to_path_buf(),
            Self::Src => project_root.join("src"),
        }
    }
}

pub enum ImportLookupPathPart<'a> {
    SearchPathFromArgs(&'a [PathBuf]),
    SearchPathFromFile(&'a [PathBuf]),
    ImportRoot(Option<&'a PathBuf>),
    FallbackSearchPath(&'a [PathBuf]),
    SitePackagePath(&'a [PathBuf]),
    InterpreterSitePackagePath(&'a [PathBuf]),
}

impl Display for ImportLookupPathPart<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SearchPathFromArgs(paths) => {
                write!(f, "Search path override (from command line): {paths:?}")
            }
            Self::SearchPathFromFile(paths) => {
                write!(f, "Search path (from config file): {paths:?}")
            }
            Self::ImportRoot(Some(root)) => {
                write!(f, "Import root (inferred from project layout): {root:?}")
            }
            Self::ImportRoot(None) => write!(f, "Import root (inferred from project layout): None"),
            Self::FallbackSearchPath(paths) => write!(
                f,
                "Fallback search path (guessed from project_includes): {paths:?}"
            ),
            Self::SitePackagePath(paths) => {
                write!(f, "Site package path from user: {paths:?}")
            }
            Self::InterpreterSitePackagePath(paths) => {
                write!(f, "Site package path queried from interpreter: {paths:?}")
            }
        }
    }
}

impl ImportLookupPathPart<'_> {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::SearchPathFromArgs(paths)
            | Self::SearchPathFromFile(paths)
            | Self::FallbackSearchPath(paths)
            | Self::SitePackagePath(paths)
            | Self::InterpreterSitePackagePath(paths) => paths.is_empty(),
            Self::ImportRoot(root) => root.is_none(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigFile {
    #[serde(skip)]
    pub source: ConfigSource,

    /// Files that should be counted as sources (e.g. user-space code).
    /// NOTE: unlike other args, this is never replaced with CLI arg overrides
    /// in this config, but may be overridden by CLI args where used.
    #[serde(
         default = "ConfigFile::default_project_includes",
         skip_serializing_if = "Globs::is_empty",
         // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
         // alias while we migrate existing fields from snake case to kebab case.
         alias = "project_includes",
     )]
    pub project_includes: Globs,

    /// Files that should be excluded as sources (e.g. user-space code). These take
    /// precedence over `project_includes`.
    /// NOTE: unlike other configs, this is never replaced with CLI arg overrides
    /// in this config, but may be overridden by CLI args where used.
    #[serde(
             default = "ConfigFile::default_project_excludes",
             skip_serializing_if = "Globs::is_empty",
             // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
             // alias while we migrate existing fields from snake case to kebab case.
             alias = "project_excludes",
         )]
    pub project_excludes: Globs,

    #[serde(skip)]
    pub search_path_from_args: Vec<PathBuf>,

    /// The list of directories where imports are
    /// imported from, including type checked files.
    /// Does not include command-line overrides or the import root!
    /// Use ConfigFile::search_path() to get the full search path.
    #[serde(
             default,
             skip_serializing_if = "Vec::is_empty",
             rename = "search-path",
             // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
             // alias while we migrate existing fields from snake case to kebab case.
             alias = "search_path"
         )]
    pub search_path_from_file: Vec<PathBuf>,

    /// The automatically inferred subdirectory that importable Python packages live in.
    #[serde(skip)]
    pub import_root: Option<PathBuf>,

    /// Not exposed to the user. When we aren't able to determine the root of a
    /// project, we guess some fallback search paths that are checked after
    /// typeshed (so we don't clobber the stdlib) and before site_package_path.
    #[serde(
             default,
             skip_serializing_if = "Vec::is_empty",
             // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
             // alias while we migrate existing fields from snake case to kebab case.
             alias = "fallback_search_path"
         )]
    pub fallback_search_path: Vec<PathBuf>,

    /// Disable Pyrefly default heuristics, specifically those around
    /// constructing a modified search path. Setting this flag will instruct
    /// Pyrefly to use the exact `search_path` you give it through your config
    /// file and CLI args.
    #[serde(default, skip_serializing_if = "crate::util::skip_default_false")]
    pub disable_search_path_heuristics: bool,

    /// Override the bundled typeshed with a custom path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typeshed_path: Option<PathBuf>,

    /// Pyrefly's configurations around interpreter querying/finding.
    #[serde(flatten)]
    pub interpreters: Interpreters,

    /// Values representing the environment of the Python interpreter
    /// (which platform, Python version, ...). When we parse, these values
    /// are default to false so we know to query the `python_interpreter` before falling
    /// back to Pyrefly's defaults.
    #[serde(flatten)]
    pub python_environment: PythonEnvironment,

    /// The `ConfigBase` values for the whole project.
    #[serde(default, flatten)]
    pub root: ConfigBase,

    /// Sub-configs that can override specific `ConfigBase` settings
    /// based on path matching.
    #[serde(
                 default,
                 rename = "sub-config",
                 skip_serializing_if = "Vec::is_empty",
                 // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
                 // alias while we migrate existing fields from snake case to kebab case.
                 alias = "sub_config"
             )]
    pub sub_configs: Vec<SubConfig>,

    /// Skips any `py.typed` checks we do when resolving `site_package_path` imports.
    #[serde(
                     default = "ConfigFile::default_true",
                     skip_serializing_if = "crate::util::skip_default_true",
                     // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
                     // alias while we migrate existing fields from snake case to kebab case.
                     alias = "use_untyped_imports",
                 )]
    pub use_untyped_imports: bool,

    /// Completely custom module to path mappings. Currently not exposed to the user.
    #[serde(skip)]
    pub custom_module_paths: SmallMap<ModuleName, ModulePath>,

    /// Skips the check to ensure any `-stubs` `site_package_path` entries have an
    /// installed non-stubs package.
    #[serde(
                     default = "ConfigFile::default_true",
                     skip_serializing_if = "crate::util::skip_default_true",
                     // TODO(connernilsen): DON'T COPY THIS TO NEW FIELDS. This is a temporary
                     // alias while we migrate existing fields from snake case to kebab case.
                     alias = "ignore_missing_source",
                 )]
    pub ignore_missing_source: bool,
}

impl Default for ConfigFile {
    /// An empty `ConfigFile`
    fn default() -> Self {
        ConfigFile {
            source: ConfigSource::Synthetic,
            project_includes: Default::default(),
            project_excludes: Default::default(),
            interpreters: Interpreters {
                python_interpreter: None,
                conda_environment: None,
                skip_interpreter_query: false,
            },
            search_path_from_args: Vec::new(),
            search_path_from_file: Vec::new(),
            disable_search_path_heuristics: false,
            import_root: None,
            fallback_search_path: Vec::new(),
            python_environment: Default::default(),
            root: Default::default(),
            sub_configs: Default::default(),
            custom_module_paths: Default::default(),
            use_untyped_imports: true,
            ignore_missing_source: true,
            typeshed_path: None,
        }
    }
}

impl ConfigFile {
    /// Gets a ConfigFile for a project directory.
    pub fn init_at_root(root: &Path, layout: &ProjectLayout) -> Self {
        let mut result = Self {
            project_includes: Self::default_project_includes(),
            project_excludes: Self::default_project_excludes(),
            // Note that rewrite_with_path_to_config() converts "" to the config file's containing directory.
            import_root: Some(layout.get_import_root(Path::new(""))),
            ..Default::default()
        };
        result.rewrite_with_path_to_config(root);
        result
    }

    /// Gets a [`FilteredGlobs`] from the optional `custom_excludes` or this
    /// [`ConfigFile`]s `project_excludes`, adding all `site_package_path` entries
    /// as extra exclude items.
    pub fn get_filtered_globs(&self, custom_excludes: Option<Globs>) -> FilteredGlobs {
        let mut project_excludes = custom_excludes.unwrap_or_else(|| self.project_excludes.clone());
        project_excludes.append(
            &self
                .site_package_path()
                // filter out project directory when editable installs add project path to PYTHONPATH
                .filter(|p| self.import_root.as_ref().is_none_or(|r| !r.starts_with(p)))
                .map(|pattern| Glob::new(pattern.to_string_lossy().to_string()))
                .collect::<Vec<_>>(),
        );
        FilteredGlobs::new(self.project_includes.clone(), project_excludes)
    }
}

impl ConfigFile {
    pub const PYREFLY_FILE_NAME: &str = "pyrefly.toml";
    pub const PYPROJECT_FILE_NAME: &str = "pyproject.toml";
    pub const CONFIG_FILE_NAMES: &[&str] = &[Self::PYREFLY_FILE_NAME, Self::PYPROJECT_FILE_NAME];

    /// Writes the configuration to a file in the specified directory.
    /// TODO(connernilsen) relative these paths to the config file's directory
    pub fn write_to_toml_in_directory(&self, directory: &Path) -> Result<()> {
        let config_str =
            toml::to_string_pretty(&self).context("Failed to serialize config to TOML")?;

        fs_anyhow::write(&directory.join("pyrefly.toml"), config_str)
            .with_context(|| format!("Failed to write config to {}", directory.display()))?;

        Ok(())
    }
    /// Files that don't contain pyrefly-specific config information but indicate that we're at the
    /// root of a Python project, which should be added to the search path.
    pub const ADDITIONAL_ROOT_FILE_NAMES: &[&str] = &["setup.py", "mypy.ini", "pyrightconfig.json"];

    pub fn default_project_includes() -> Globs {
        Globs::new(vec!["**/*".to_owned()])
    }

    pub fn default_project_excludes() -> Globs {
        Globs::new(vec![
            // match any `.venv` or `venv` directory
            "**/*venv/**".to_owned(),
            // Note: dot files are now excluded at the Glob::files() level
        ])
    }

    pub fn default_true() -> bool {
        true
    }

    pub fn python_version(&self) -> PythonVersion {
        // we can use unwrap here, because the value in the root config must
        // be set in `ConfigFile::configure()`.
        self.python_environment.python_version.unwrap()
    }

    pub fn python_platform(&self) -> &PythonPlatform {
        // we can use unwrap here, because the value in the root config must
        // be set in `ConfigFile::configure()`.
        self.python_environment.python_platform.as_ref().unwrap()
    }

    pub fn search_path(&self) -> impl Iterator<Item = &PathBuf> + Clone {
        self.search_path_from_args
            .iter()
            .chain(self.search_path_from_file.iter())
            .chain(if self.disable_search_path_heuristics {
                None.iter()
            } else {
                self.import_root.iter()
            })
    }

    pub fn site_package_path(&self) -> impl Iterator<Item = &PathBuf> + Clone {
        // we can use unwrap here, because the value in the root config must
        // be set in `ConfigFile::configure()`.
        self.python_environment
            .site_package_path
            .as_ref()
            .unwrap()
            .iter()
            .chain(self.python_environment.interpreter_site_package_path.iter())
    }

    /// Gets the full, ordered path used for import lookup. Used for pretty-printing.
    pub fn structured_import_lookup_path(&self) -> Vec<ImportLookupPathPart> {
        let mut result = vec![
            ImportLookupPathPart::SearchPathFromArgs(&self.search_path_from_args),
            ImportLookupPathPart::SearchPathFromFile(&self.search_path_from_file),
        ];
        if !self.disable_search_path_heuristics {
            result.push(ImportLookupPathPart::ImportRoot(self.import_root.as_ref()));
            result.push(ImportLookupPathPart::FallbackSearchPath(
                &self.fallback_search_path,
            ));
        }
        result.push(ImportLookupPathPart::SitePackagePath(
            self.python_environment.site_package_path.as_ref().unwrap(),
        ));
        result.push(ImportLookupPathPart::InterpreterSitePackagePath(
            &self.python_environment.interpreter_site_package_path,
        ));
        result
    }

    pub fn get_sys_info(&self) -> SysInfo {
        SysInfo::new(self.python_version(), self.python_platform().clone())
    }

    pub fn errors(&self, path: &Path) -> &ErrorDisplayConfig {
        self.get_from_sub_configs(ConfigBase::get_errors, path)
            .unwrap_or_else(||
                 // we can use unwrap here, because the value in the root config must
                 // be set in `ConfigFile::configure()`.
                 self.root.errors.as_ref().unwrap())
    }

    pub fn replace_imports_with_any(&self, path: Option<&Path>, module: ModuleName) -> bool {
        let wildcards = path
            .and_then(|path| {
                self.get_from_sub_configs(ConfigBase::get_replace_imports_with_any, path)
            })
            .unwrap_or_else(||
             // we can use unwrap here, because the value in the root config must
             // be set in `ConfigFile::configure()`.
             self.root.replace_imports_with_any.as_deref().unwrap());
        wildcards.iter().any(|p| p.matches(module))
    }

    pub fn ignore_missing_imports(&self, path: Option<&Path>, module: ModuleName) -> bool {
        let wildcards = path
            .and_then(|path| {
                self.get_from_sub_configs(ConfigBase::get_ignore_missing_imports, path)
            })
            .unwrap_or_else(||
             // we can use unwrap here, because the value in the root config must
             // be set in `ConfigFile::configure()`.
             self.root.ignore_missing_imports.as_deref().unwrap());
        wildcards.iter().any(|p| p.matches(module))
    }

    pub fn untyped_def_behavior(&self, path: &Path) -> UntypedDefBehavior {
        self.get_from_sub_configs(ConfigBase::get_untyped_def_behavior, path)
            .unwrap_or_else(||
                 // we can use unwrap here, because the value in the root config must
                 // be set in `ConfigFile::configure()`.
                 self.root.untyped_def_behavior.unwrap())
    }

    fn ignore_errors_in_generated_code(&self, path: &Path) -> bool {
        self.get_from_sub_configs(ConfigBase::get_ignore_errors_in_generated_code, path)
            .unwrap_or_else(||
                 // we can use unwrap here, because the value in the root config must
                 // be set in `ConfigFile::configure()`.
                 self.root.ignore_errors_in_generated_code.unwrap())
    }

    pub fn permissive_ignores(&self, path: &Path) -> bool {
        self.get_from_sub_configs(|x| x.permissive_ignores, path)
            .unwrap_or_else(||
                // we can use unwrap here, because the value in the root config must
                // be set in `ConfigFile::configure()`.
                self.root.permissive_ignores.unwrap())
    }

    pub fn get_error_config(&self, path: &Path) -> ErrorConfig {
        ErrorConfig::new(
            self.errors(path),
            self.ignore_errors_in_generated_code(path),
            self.permissive_ignores(path),
        )
    }

    /// Filter to sub configs whose matches succeed for the given `path`,
    /// then return the first non-None value the getter returns, or None
    /// if a non-empty value can't be found.
    fn get_from_sub_configs<'a, T>(
        &'a self,
        getter: impl Fn(&'a ConfigBase) -> Option<T>,
        path: &Path,
    ) -> Option<T> {
        self.sub_configs.iter().find_map(|c| {
            if c.matches.matches(path).ok()? {
                return getter(&c.settings);
            }
            None
        })
    }

    /// Configures values that must be updated *after* overwriting with CLI flag values,
    /// which should probably be everything except for `PathBuf` or `Globs` types.
    pub fn configure(&mut self) -> Vec<ConfigError> {
        let mut configure_errors = Vec::new();

        if self.interpreters.skip_interpreter_query {
            self.python_environment.set_empty_to_default();
        } else {
            match self.interpreters.find_interpreter(self.source.root()) {
                Ok(interpreter) => {
                    let (env, error) = PythonEnvironment::get_interpreter_env(&interpreter);
                    self.python_environment.override_empty(env);
                    self.interpreters.python_interpreter = Some(interpreter);
                    if let Some(error) = error {
                        configure_errors.push(error);
                    }
                }
                Err(error) => {
                    self.python_environment.set_empty_to_default();
                    configure_errors.push(error.context("While finding Python interpreter."));
                }
            }
        }

        if self.root.errors.is_none() {
            self.root.errors = Some(Default::default());
        }

        if self.root.replace_imports_with_any.is_none() {
            self.root.replace_imports_with_any = Some(Default::default());
        }

        if self.root.ignore_missing_imports.is_none() {
            self.root.ignore_missing_imports = Some(Default::default());
        }

        if self.root.untyped_def_behavior.is_none() {
            self.root.untyped_def_behavior = Some(Default::default());
        }

        if self.root.ignore_errors_in_generated_code.is_none() {
            self.root.ignore_errors_in_generated_code = Some(Default::default());
        }

        if self.root.permissive_ignores.is_none() {
            self.root.permissive_ignores = Some(false);
        }

        fn validate<'a>(
            paths: &'a [PathBuf],
            field: &'a str,
        ) -> impl Iterator<Item = anyhow::Error> + 'a {
            paths.iter().filter_map(move |p| {
                validate_path(p)
                    .err()
                    .map(|err| err.context(format!("Invalid {field}")))
            })
        }
        if let Some(site_package_path) = &self.python_environment.site_package_path {
            configure_errors.extend(validate(site_package_path.as_ref(), "site_package_path"));
        }
        configure_errors.extend(validate(&self.search_path_from_file, "search_path"));

        if self.interpreters.python_interpreter.is_some()
            && self.interpreters.conda_environment.is_some()
        {
            configure_errors.push(anyhow::anyhow!(
                     "Cannot use both `python-interpreter` and `conda-environment`. Finding environment info using `python-interpreter`.",
             ));
        }

        if let ConfigSource::File(path) = &self.source {
            configure_errors
                .into_map(|e| ConfigError::warn(e.context(format!("{}", path.display()))))
        } else {
            configure_errors.into_map(ConfigError::warn)
        }
    }

    /// Rewrites any config values that must be updated *before* applying CLI flag values, namely
    /// rewriting any `PathBuf`s and `Globs` to be relative to `config_root`.
    /// We do this as a step separate from `configure()` because CLI args may override some of these
    /// values, but CLI args will always be relative to CWD, whereas config values should be relative
    /// to the config root.
    fn rewrite_with_path_to_config(&mut self, config_root: &Path) {
        self.project_includes = self.project_includes.clone().from_root(config_root);
        self.project_excludes = self.project_excludes.clone().from_root(config_root);
        self.search_path_from_file
            .iter_mut()
            .for_each(|search_root| {
                *search_root = search_root.absolutize_from(config_root);
            });
        if let Some(import_root) = &self.import_root {
            self.import_root = Some(import_root.absolutize_from(config_root));
        }
        self.python_environment
            .site_package_path
            .iter_mut()
            .for_each(|v| {
                v.iter_mut().for_each(|site_package_path| {
                    *site_package_path = site_package_path.absolutize_from(config_root);
                });
            });
        self.interpreters.python_interpreter = self
            .interpreters
            .python_interpreter
            .take()
            .map(|s| s.map(|i| i.absolutize_from(config_root)));
        self.sub_configs
            .iter_mut()
            .for_each(|c| c.rewrite_with_path_to_config(config_root));
    }

    pub fn from_file(config_path: &Path) -> (ConfigFile, Vec<ConfigError>) {
        fn read_path(config_path: &Path) -> anyhow::Result<Option<ConfigFile>> {
            let config_str = fs_anyhow::read_to_string(config_path)?;
            if config_path.file_name() == Some(OsStr::new(ConfigFile::PYPROJECT_FILE_NAME)) {
                Ok(ConfigFile::parse_pyproject_toml(&config_str)?)
            } else if config_path.file_name().is_some_and(|fi| {
                fi.to_str()
                    .is_some_and(|fi| ConfigFile::ADDITIONAL_ROOT_FILE_NAMES.contains(&fi))
            }) {
                // We'll create a file with default options but treat config_root as the project root.
                Ok(None)
            } else {
                Ok(Some(ConfigFile::parse_config(&config_str)?))
            }
        }
        fn f(config_path: &Path) -> (ConfigFile, Vec<ConfigError>) {
            let mut errors = Vec::new();
            let (maybe_config, config_source) = match read_path(config_path) {
                Ok(Some(config)) => (Some(config), ConfigSource::File(config_path.to_path_buf())),
                Ok(None) => (None, ConfigSource::Marker(config_path.to_path_buf())),
                Err(e) => {
                    errors.push(ConfigError::error(e));
                    (None, ConfigSource::File(config_path.to_path_buf()))
                }
            };
            let mut config = match config_path.parent() {
                Some(config_root) => {
                    let layout = ProjectLayout::new(config_root);
                    if let Some(mut config) = maybe_config {
                        config.rewrite_with_path_to_config(config_root);
                        config.import_root = Some(layout.get_import_root(config_root));
                        config
                    } else {
                        ConfigFile::init_at_root(config_root, &layout)
                    }
                }
                None => {
                    errors.push(ConfigError::error(anyhow!(
                        "Could not find parent of path `{}`",
                        config_path.display()
                    )));
                    maybe_config.unwrap_or_else(ConfigFile::default)
                }
            };
            config.source = config_source;

            if !config.root.extras.0.is_empty() {
                let extra_keys = config.root.extras.0.keys().join(", ");
                errors.push(ConfigError::warn(anyhow!(
                    "Extra keys found in config: {extra_keys}"
                )));
            }
            for sub_config in &config.sub_configs {
                if !sub_config.settings.extras.0.is_empty() {
                    let extra_keys = sub_config.settings.extras.0.keys().join(", ");
                    errors.push(ConfigError::warn(anyhow!(
                        "Extra keys found in sub config matching {}: {extra_keys}",
                        sub_config.matches
                    )));
                }
            }
            (config, errors)
        }
        let config_path = config_path.absolutize();
        let (config, errors) = f(&config_path);
        let errors = errors.into_map(|err| err.context(format!("{}", config_path.display())));
        (config, errors)
    }

    fn parse_config(config_str: &str) -> anyhow::Result<ConfigFile> {
        toml::from_str::<ConfigFile>(config_str).map_err(|err| anyhow::Error::msg(err.to_string()))
    }

    fn parse_pyproject_toml(config_str: &str) -> anyhow::Result<Option<ConfigFile>> {
        Ok(toml::from_str::<PyProject>(config_str)
            .map_err(|err| anyhow::Error::msg(err.to_string()))?
            .pyrefly())
    }
}

impl Display for ConfigFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{source: {:?}, project_includes: {}, project_excludes: {}, search_path: [{}], python_interpreter: {:?}, python_environment: {}, replace_imports_with_any: [{}], ignore_missing_imports: [{}]}}",
            self.source,
            self.project_includes,
            self.project_excludes,
            self.search_path().map(|p| p.display()).join(", "),
            self.interpreters.python_interpreter,
            self.python_environment,
            self.root
                .replace_imports_with_any
                .as_ref()
                .map(|r| { r.iter().map(|p| p.as_str()).join(", ") })
                .unwrap_or_default(),
            self.root
                .ignore_missing_imports
                .as_ref()
                .map(|r| { r.iter().map(|p| p.as_str()).join(", ") })
                .unwrap_or_default(),
        )
    }
}

/// Returns an error if the path is definitely invalid.
pub fn validate_path(path: &Path) -> anyhow::Result<()> {
    match path.try_exists() {
        Ok(true) => Ok(()),
        Err(err) => {
            debug!(
                "Error checking for existence of path {}: {}",
                path.display(),
                err
            );
            Ok(())
        }
        Ok(false) => Err(anyhow!("`{}` does not exist", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path;

    use pretty_assertions::assert_eq;
    use tempfile::TempDir;
    use toml::Table;
    use toml::Value;

    use super::*;
    use crate::error_kind::ErrorKind;
    use crate::error_kind::Severity;
    use crate::module_wildcard::ModuleWildcard;
    use crate::util::ConfigOrigin;

    #[test]
    fn deserialize_pyrefly_config() {
        let config_str = r#"
             project-includes = ["tests", "./implementation"]
             project-excludes = ["tests/untyped/**"]
             untyped-def-behavior = "check-and-infer-return-type"
             search-path = ["../.."]
             python-platform = "darwin"
             python-version = "1.2.3"
             site-package-path = ["venv/lib/python1.2.3/site-packages"]
             python-interpreter = "venv/my/python"
             replace-imports-with-any = ["fibonacci"]
             ignore-missing-imports = ["sprout"]
             ignore-errors-in-generated-code = true
             use-untyped-imports = true
             ignore-missing-source = true

             [errors]
             assert-type = true
             bad-return = false

             [[sub-config]]
             matches = "sub/project/**"

             untyped-def-behavior = "check-and-infer-return-any"
             replace-imports-with-any = []
             ignore-missing-imports = []
             ignore-errors-in-generated-code = false
             [sub-config.errors]
             assert-type = false
             invalid-yield = false
        "#;
        let config = ConfigFile::parse_config(config_str).unwrap();
        assert_eq!(
            config,
            ConfigFile {
                source: ConfigSource::Synthetic,
                project_includes: Globs::new(vec![
                    "tests".to_owned(),
                    "./implementation".to_owned()
                ]),
                project_excludes: Globs::new(vec!["tests/untyped/**".to_owned()]),
                search_path_from_args: Vec::new(),
                search_path_from_file: vec![PathBuf::from("../..")],
                disable_search_path_heuristics: false,
                import_root: None,
                fallback_search_path: Vec::new(),
                python_environment: PythonEnvironment {
                    python_platform: Some(PythonPlatform::mac()),
                    python_version: Some(PythonVersion::new(1, 2, 3)),
                    site_package_path: Some(vec![PathBuf::from(
                        "venv/lib/python1.2.3/site-packages"
                    )]),
                    interpreter_site_package_path: config
                        .python_environment
                        .interpreter_site_package_path
                        .clone(),
                },
                interpreters: Interpreters {
                    python_interpreter: Some(ConfigOrigin::config(PathBuf::from("venv/my/python"))),
                    conda_environment: None,
                    skip_interpreter_query: false,
                },
                root: ConfigBase {
                    extras: Default::default(),
                    errors: Some(ErrorDisplayConfig::new(HashMap::from_iter([
                        (ErrorKind::BadReturn, Severity::Ignore),
                        (ErrorKind::AssertType, Severity::Error),
                    ]))),
                    ignore_errors_in_generated_code: Some(true),
                    replace_imports_with_any: Some(vec![ModuleWildcard::new("fibonacci").unwrap()]),
                    ignore_missing_imports: Some(vec![ModuleWildcard::new("sprout").unwrap()]),
                    untyped_def_behavior: Some(UntypedDefBehavior::CheckAndInferReturnType),
                    permissive_ignores: None,
                },
                custom_module_paths: Default::default(),
                sub_configs: vec![SubConfig {
                    matches: Glob::new("sub/project/**".to_owned()),
                    settings: ConfigBase {
                        extras: Default::default(),
                        errors: Some(ErrorDisplayConfig::new(HashMap::from_iter([
                            (ErrorKind::InvalidYield, Severity::Ignore),
                            (ErrorKind::AssertType, Severity::Ignore),
                        ]))),
                        ignore_errors_in_generated_code: Some(false),
                        replace_imports_with_any: Some(Vec::new()),
                        ignore_missing_imports: Some(Vec::new()),
                        untyped_def_behavior: Some(UntypedDefBehavior::CheckAndInferReturnAny),
                        permissive_ignores: None,
                    }
                }],
                use_untyped_imports: true,
                ignore_missing_source: true,
                typeshed_path: None,
            }
        );
    }

    #[test]
    fn deserialize_pyrefly_config_snake_case() {
        let config_str = r#"
             project_includes = ["tests", "./implementation"]
             project_excludes = ["tests/untyped/**"]
             untyped_def_behavior = "check-and-infer-return-type"
             search_path = ["../.."]
             python_platform = "darwin"
             python_version = "1.2.3"
             site_package_path = ["venv/lib/python1.2.3/site-packages"]
             python_interpreter = "venv/my/python"
             replace_imports_with_any = ["fibonacci"]
             ignore_errors_in_generated_code = true
             use_untyped_imports = true
             ignore_missing_source = true

             [errors]
             assert-type = "error"
             bad-return = "ignore"

             [[sub_config]]
             matches = "sub/project/**"

             untyped_def_behavior = "check-and-infer-return-any"
             replace_imports_with_any = []
             ignore_errors_in_generated_code = false
             [sub_config.errors]
             assert-type = "warn"
             invalid-yield = "ignore"
        "#;
        let config = ConfigFile::parse_config(config_str).unwrap();
        assert!(config.root.extras.0.is_empty());
        assert!(
            config
                .sub_configs
                .iter()
                .all(|c| c.settings.extras.0.is_empty())
        );
    }

    #[test]
    fn deserialize_pyrefly_config_defaults() {
        let config_str = "";
        let config = ConfigFile::parse_config(config_str).unwrap();
        assert_eq!(
            config,
            ConfigFile {
                project_includes: ConfigFile::default_project_includes(),
                project_excludes: ConfigFile::default_project_excludes(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn deserialize_pyrefly_config_with_unknown() {
        let config_str = r#"
             laszewo = "good kids"
             python_platform = "windows"

             [[sub_config]]
             matches = "abcd"

                 atliens = 1
                 "#;
        let config = ConfigFile::parse_config(config_str).unwrap();
        assert_eq!(
            config.root.extras.0,
            Table::from_iter([("laszewo".to_owned(), Value::String("good kids".to_owned())),])
        );
        assert_eq!(
            config.sub_configs[0].settings.extras.0,
            Table::from_iter([("atliens".to_owned(), Value::Integer(1))])
        );
    }

    #[test]
    fn deserialize_pyproject_toml() {
        let config_str = r#"
             [tool.pyrefly]
             project_includes = ["./tests", "./implementation"]
                 python_platform = "darwin"
                 python_version = "1.2.3"
                 "#;
        let config = ConfigFile::parse_pyproject_toml(config_str)
            .unwrap()
            .unwrap();
        assert_eq!(
            config,
            ConfigFile {
                project_includes: Globs::new(vec![
                    "./tests".to_owned(),
                    "./implementation".to_owned()
                ]),
                project_excludes: ConfigFile::default_project_excludes(),
                python_environment: PythonEnvironment {
                    python_platform: Some(PythonPlatform::mac()),
                    python_version: Some(PythonVersion::new(1, 2, 3)),
                    site_package_path: None,
                    interpreter_site_package_path: config
                        .python_environment
                        .interpreter_site_package_path
                        .clone(),
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn deserialize_pyproject_toml_defaults() {
        let config_str = "";
        let config = ConfigFile::parse_pyproject_toml(config_str).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn deserialize_pyproject_toml_with_unknown() {
        let config_str = r#"
            top_level = 1
            [table1]
            table1_value = 2
            [tool.pysa]
            pysa_value = 2
            [tool.pyrefly]
            python_version = "1.2.3"
        "#;
        let config = ConfigFile::parse_pyproject_toml(config_str)
            .unwrap()
            .unwrap();
        assert_eq!(
            config,
            ConfigFile {
                project_includes: ConfigFile::default_project_includes(),
                project_excludes: ConfigFile::default_project_excludes(),
                python_environment: PythonEnvironment {
                    python_version: Some(PythonVersion::new(1, 2, 3)),
                    python_platform: None,
                    site_package_path: None,
                    interpreter_site_package_path: config
                        .python_environment
                        .interpreter_site_package_path
                        .clone(),
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn deserialize_pyproject_toml_without_pyrefly() {
        let config_str = "
             top_level = 1
             [table1]
             table1_value = 2
                 [tool.pysa]
                 pysa_value = 2
                     ";
        let config = ConfigFile::parse_pyproject_toml(config_str).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn deserialize_pyproject_toml_with_unknown_in_pyrefly() {
        let config_str = r#"
             top_level = 1
             [table1]
             table1_value = 2
                 [tool.pysa]
                 pysa_value = 2
                     [tool.pyrefly]
                     python_version = "1.2.3"
                         inzo = "overthinker"
                         "#;
        let config = ConfigFile::parse_pyproject_toml(config_str)
            .unwrap()
            .unwrap();
        assert_eq!(
            config.root.extras.0,
            Table::from_iter([("inzo".to_owned(), Value::String("overthinker".to_owned()))])
        );
    }

    #[test]
    fn test_rewrite_with_path_to_config() {
        fn with_sep(s: &str) -> String {
            s.replace("/", path::MAIN_SEPARATOR_STR)
        }
        let mut python_environment = PythonEnvironment {
            site_package_path: Some(vec![PathBuf::from("venv/lib/python1.2.3/site-packages")]),
            ..PythonEnvironment::default()
        };
        let interpreter = "venv/bin/python3".to_owned();
        let mut config = ConfigFile {
            source: ConfigSource::Synthetic,
            project_includes: Globs::new(vec!["path1/**".to_owned(), "path2/path3".to_owned()]),
            project_excludes: Globs::new(vec!["tests/untyped/**".to_owned()]),
            search_path_from_args: Vec::new(),
            search_path_from_file: vec![PathBuf::from("../..")],
            disable_search_path_heuristics: false,
            import_root: None,
            fallback_search_path: Vec::new(),
            python_environment: python_environment.clone(),
            interpreters: Interpreters {
                python_interpreter: Some(ConfigOrigin::config(PathBuf::from(interpreter.clone()))),
                conda_environment: None,
                skip_interpreter_query: false,
            },
            root: Default::default(),
            custom_module_paths: Default::default(),
            sub_configs: vec![SubConfig {
                matches: Glob::new("sub/project/**".to_owned()),
                settings: Default::default(),
            }],
            use_untyped_imports: false,
            ignore_missing_source: false,
            typeshed_path: None,
        };

        let current_dir = std::env::current_dir().unwrap();
        let path_str = with_sep("path/to/my/config");
        let test_path = current_dir.join(&path_str);

        let project_includes_vec = vec![
            test_path.join("path1/**").to_string_lossy().into_owned(),
            test_path.join("path2/path3").to_string_lossy().into_owned(),
        ];
        let project_excludes_vec = vec![
            test_path
                .join("tests/untyped/**")
                .to_string_lossy()
                .into_owned(),
        ];
        let search_path = vec![test_path.parent().unwrap().parent().unwrap().to_path_buf()];
        python_environment.site_package_path =
            Some(vec![test_path.join("venv/lib/python1.2.3/site-packages")]);

        let sub_config_matches = Glob::new(
            test_path
                .join("sub/project/**")
                .to_string_lossy()
                .into_owned(),
        );

        config.rewrite_with_path_to_config(&test_path);

        let expected_config = ConfigFile {
            source: ConfigSource::Synthetic,
            project_includes: Globs::new(project_includes_vec),
            project_excludes: Globs::new(project_excludes_vec),
            interpreters: Interpreters {
                python_interpreter: Some(ConfigOrigin::config(test_path.join(interpreter))),
                conda_environment: None,
                skip_interpreter_query: false,
            },
            search_path_from_args: Vec::new(),
            search_path_from_file: search_path,
            disable_search_path_heuristics: false,
            import_root: None,
            fallback_search_path: Vec::new(),
            python_environment,
            root: Default::default(),
            custom_module_paths: Default::default(),
            sub_configs: vec![SubConfig {
                matches: sub_config_matches,
                settings: Default::default(),
            }],
            use_untyped_imports: false,
            ignore_missing_source: false,
            typeshed_path: None,
        };
        assert_eq!(config, expected_config);
    }

    #[test]
    fn test_deserializing_unknown_error_errors() {
        let config_str = "
             [errors]
             subtronics = true
                 zeds_dead = false
                 GRiZ = true
                 ";
        let err = ConfigFile::parse_config(config_str).unwrap_err();
        assert!(err.to_string().contains("unknown variant"));
    }

    #[test]
    fn test_deserializing_sub_config_missing_matches() {
        let config_str = r#"
             [[sub_config]]
             search_path = ["../../.."]
                 "#;
        let err = ConfigFile::parse_config(config_str).unwrap_err();
        assert!(err.to_string().contains("missing field `matches`"));
    }

    #[test]
    fn test_expect_all_fields_set_in_root_config() {
        let root = TempDir::new().unwrap();
        let mut config = ConfigFile::init_at_root(root.path(), &ProjectLayout::default());
        config.configure();

        let table: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();

        let ignore_keys: Vec<String> = vec![
            // top level configs, where null values (if possible), should be allowed
            "project_includes",
            "project_excludes",
            "python_interpreter",
            // values we won't be getting
            "extras",
            // values that must be Some (if flattened, their contents will be checked)
            "python_environment",
        ]
        .into_iter()
        .map(|k| k.to_owned())
        .collect();

        table.keys().for_each(|k| {
            if ignore_keys.contains(k) {
                return;
            }

            assert!(
                table.get(k).is_some_and(|v| !v.is_null()),
                "Value for {k} is None after ConfigFile::configure()"
            );
        });
    }

    #[test]
    fn test_get_from_sub_configs() {
        let config = ConfigFile {
            root: ConfigBase {
                errors: Some(Default::default()),
                replace_imports_with_any: Some(vec![ModuleWildcard::new("root").unwrap()]),
                ignore_missing_imports: None,
                untyped_def_behavior: Some(UntypedDefBehavior::CheckAndInferReturnType),
                ignore_errors_in_generated_code: Some(false),
                extras: Default::default(),
                permissive_ignores: Some(false),
            },
            sub_configs: vec![
                SubConfig {
                    matches: Glob::new("**/highest/**".to_owned()),
                    settings: ConfigBase {
                        replace_imports_with_any: Some(vec![
                            ModuleWildcard::new("highest").unwrap(),
                        ]),
                        ignore_errors_in_generated_code: None,
                        ..Default::default()
                    },
                },
                SubConfig {
                    matches: Glob::new("**/priority*".to_owned()),
                    settings: ConfigBase {
                        replace_imports_with_any: Some(vec![
                            ModuleWildcard::new("second").unwrap(),
                        ]),
                        ignore_errors_in_generated_code: Some(true),
                        ..Default::default()
                    },
                },
            ],
            ..Default::default()
        };

        // test precedence (two configs match, one higher priority)
        assert!(config.replace_imports_with_any(
            Some(Path::new("this/is/highest/priority")),
            ModuleName::from_str("highest")
        ));

        // test find fallback match
        assert!(config.replace_imports_with_any(
            Some(Path::new("this/is/second/priority")),
            ModuleName::from_str("second")
        ));

        // test empty value falls back to next
        assert!(config.ignore_errors_in_generated_code(Path::new("this/is/highest/priority")));

        // test no pattern match
        assert!(config.replace_imports_with_any(
            Some(Path::new("this/does/not/match/any")),
            ModuleName::from_str("root")
        ));

        // test replace_imports_with_any special case None path
        assert!(config.replace_imports_with_any(None, ModuleName::from_str("root")));
    }

    #[test]
    fn test_default_search_path() {
        let tempdir = TempDir::new().unwrap();
        let config = ConfigFile::init_at_root(tempdir.path(), &ProjectLayout::default());
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![tempdir.path().to_path_buf()]
        );
    }

    #[test]
    fn test_pyproject_toml_search_path() {
        let root = TempDir::new().unwrap();
        let path = root.path().join(ConfigFile::PYPROJECT_FILE_NAME);
        fs::write(&path, "[tool.pyrefly]").unwrap();
        let config = ConfigFile::from_file(&path).0;
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![root.path().to_path_buf()]
        );
    }

    fn create_empty_file_and_parse_config(root: &TempDir, name: &str) -> ConfigFile {
        let path = root.path().join(name);
        fs::write(&path, "").unwrap();
        ConfigFile::from_file(&path).0
    }

    #[test]
    fn test_pyproject_toml_no_pyrefly_search_path() {
        let root = TempDir::new().unwrap();
        let config = create_empty_file_and_parse_config(&root, ConfigFile::PYPROJECT_FILE_NAME);
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![root.path().to_path_buf()]
        );
    }

    #[test]
    fn test_setup_py_search_path() {
        let root = TempDir::new().unwrap();
        let config = create_empty_file_and_parse_config(&root, "setup.py");
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![root.path().to_path_buf()]
        );
    }

    #[test]
    fn test_mypy_config_search_path() {
        let root = TempDir::new().unwrap();
        let config = create_empty_file_and_parse_config(&root, "mypy.ini");
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![root.path().to_path_buf()]
        );
    }

    #[test]
    fn test_pyright_config_search_path() {
        let root = TempDir::new().unwrap();
        let config = create_empty_file_and_parse_config(&root, "pyrightconfig.json");
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![root.path().to_path_buf()]
        );
    }

    #[test]
    fn test_src_layout_default_config() {
        // root/
        // - pyproject.toml (empty)
        // - src/
        // - my_amazing_scripts/
        //   - foo.py
        let root = TempDir::new().unwrap();
        let src_dir = root.path().join("src");
        let scripts_dir = root.path().join("my_amazing_scripts");
        let python_file = scripts_dir.join("foo.py");
        fs::create_dir(&src_dir).unwrap();
        fs::create_dir(&scripts_dir).unwrap();
        fs::write(&python_file, "").unwrap();
        let config = create_empty_file_and_parse_config(&root, ConfigFile::PYPROJECT_FILE_NAME);
        // We should still find Python files (commonly scripts and tests) outside src/.
        assert_eq!(config.project_includes.files().unwrap(), vec![python_file]);
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![src_dir]
        );
    }

    #[test]
    fn test_src_layout_with_config() {
        // root/
        // - pyrefly.toml
        // - src/
        let root = TempDir::new().unwrap();
        let src_dir = root.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        let pyrefly_path = root.path().join(ConfigFile::PYREFLY_FILE_NAME);
        fs::write(&pyrefly_path, "project_includes = [\"**/*\"]").unwrap();
        let config = ConfigFile::from_file(&pyrefly_path).0;
        // File contents should still be relative to the location of the config file, not src/.
        assert_eq!(
            config.project_includes,
            Globs::new(vec![root.path().join("**/*").to_string_lossy().to_string()]),
        );
        assert_eq!(
            config.search_path().cloned().collect::<Vec<_>>(),
            vec![src_dir]
        );
    }

    #[test]
    fn test_get_filtered_globs() {
        let mut config = ConfigFile::default();
        let site_package_path = vec![
            "venv/site_packages".to_owned(),
            "system/site_packages".to_owned(),
        ];
        config.python_environment.site_package_path = Some(
            site_package_path
                .iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>(),
        );
        config.project_excludes = ConfigFile::default_project_excludes();

        assert_eq!(
            config.get_filtered_globs(None),
            FilteredGlobs::new(
                config.project_includes.clone(),
                Globs::new(
                    vec!["**/*venv/**".to_owned()]
                        .into_iter()
                        .chain(site_package_path.clone())
                        .collect::<Vec<_>>()
                ),
            )
        );
        assert_eq!(
            config.get_filtered_globs(Some(Globs::new(vec!["custom_excludes".to_owned()]))),
            FilteredGlobs::new(
                config.project_includes.clone(),
                Globs::new(
                    vec!["custom_excludes".to_owned()]
                        .into_iter()
                        .chain(site_package_path)
                        .collect::<Vec<_>>()
                ),
            )
        );
    }

    #[test]
    fn test_python_interpreter_conda_environment() {
        let mut config = ConfigFile {
            interpreters: Interpreters {
                python_interpreter: Some(ConfigOrigin::config(PathBuf::new())),
                conda_environment: Some(ConfigOrigin::config("".to_owned())),
                skip_interpreter_query: false,
            },
            ..Default::default()
        };

        let validation_errors = config.configure();

        assert!(
             validation_errors.iter().any(|e| {
                 e.get_message() == "Cannot use both `python-interpreter` and `conda-environment`. Finding environment info using `python-interpreter`."
             })
         );
    }

    #[test]
    fn test_interpreter_not_queried_with_skip_interpreter_query() {
        let mut config = ConfigFile {
            interpreters: Interpreters {
                skip_interpreter_query: true,
                ..Default::default()
            },
            ..Default::default()
        };

        config.configure();
        assert!(config.interpreters.python_interpreter.is_none());
        assert!(config.interpreters.conda_environment.is_none());
    }

    #[test]
    fn test_serializing_config_origins() {
        let mut config = ConfigFile {
            interpreters: Interpreters {
                python_interpreter: Some(ConfigOrigin::config(PathBuf::from("abcd"))),
                conda_environment: None,
                skip_interpreter_query: false,
            },
            project_includes: ConfigFile::default_project_includes(),
            project_excludes: ConfigFile::default_project_excludes(),
            ..Default::default()
        };
        let reparsed = ConfigFile::parse_config(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(reparsed, config);

        config.interpreters.python_interpreter = Some(ConfigOrigin::auto(PathBuf::from("abcd")));
        let reparsed = ConfigFile::parse_config(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(reparsed.interpreters.python_interpreter, None);

        config.interpreters.python_interpreter = Some(ConfigOrigin::cli(PathBuf::from("abcd")));
        let reparsed = ConfigFile::parse_config(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(reparsed.interpreters.python_interpreter, None);
    }
}
