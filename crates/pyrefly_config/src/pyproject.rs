/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::path::Path;

use anyhow::Context as _;
use pyrefly_util::fs_anyhow;
use serde::Deserialize;
use serde::Serialize;

use crate::config::ConfigFile;

/// Wrapper used to (de)serialize pyrefly configs from pyproject.toml files.
#[derive(Debug, Serialize, Deserialize)]
struct Tool {
    #[serde(default)]
    pyrefly: Option<ConfigFile>,
}

/// Wrapper used to (de)serialize pyrefly configs from pyproject.toml files.
#[derive(Debug, Serialize, Deserialize)]
pub struct PyProject {
    #[serde(default)]
    tool: Option<Tool>,
}

impl PyProject {
    /// Wrap the given ConfigFile in a `PyProject { Tool { ... }}`
    pub fn new(cfg: ConfigFile) -> Self {
        Self {
            tool: Some(Tool { pyrefly: Some(cfg) }),
        }
    }

    pub(crate) fn pyrefly(self) -> Option<ConfigFile> {
        self.tool.and_then(|t| t.pyrefly)
    }

    pub fn update(pyproject_path: &Path, config: ConfigFile) -> anyhow::Result<()> {
        const ERR_WRITE_CONFIG: &str =
            "While trying to write Pyrefly config to pyproject.toml file";
        let config_pyproject = PyProject::new(config);
        if pyproject_path.exists() {
            let original_content = fs_anyhow::read_to_string(pyproject_path)?;
            let mut doc = original_content
                .parse::<toml_edit::DocumentMut>()
                .with_context(|| {
                    format!(
                        "Failed to parse {} as TOML document",
                        pyproject_path.display()
                    )
                })?;
            let toml_string = toml::to_string_pretty(&config_pyproject)?;
            let config_doc = toml_string.parse::<toml_edit::DocumentMut>()?;
            if let Some(tool_table) = config_doc.get("tool")
                && let Some(pyrefly_table) = tool_table.get("pyrefly")
            {
                let tool_entry = doc
                    .entry("tool")
                    .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
                if let Some(tool_table_mut) = tool_entry.as_table_mut() {
                    tool_table_mut.remove("pyrefly");
                    let max_tool_pos = tool_table_mut
                        .iter()
                        .filter_map(|(_, v)| v.as_table().and_then(|t| t.position()))
                        .max()
                        .unwrap_or(0);
                    tool_table_mut.insert("pyrefly", pyrefly_table.clone());
                    if let Some(pyrefly_item) = tool_table_mut.get_mut("pyrefly")
                        && let Some(pyrefly_table_mut) = pyrefly_item.as_table_mut()
                    {
                        pyrefly_table_mut.decor_mut().set_prefix("\n");
                        pyrefly_table_mut.set_position(max_tool_pos + 1);
                    }
                }
            }
            fs_anyhow::write(pyproject_path, doc.to_string()).with_context(|| ERR_WRITE_CONFIG)
        } else {
            let mut serialized_toml = toml::to_string_pretty(&config_pyproject)?;
            if !serialized_toml.contains("[tool.pyrefly]") {
                serialized_toml = String::from("[tool.pyrefly]\n");
            }
            fs_anyhow::write(pyproject_path, serialized_toml).with_context(|| ERR_WRITE_CONFIG)
        }
    }
}

#[cfg(test)]
mod tests {
    use pyrefly_util::globs::Globs;

    use super::*;

    #[test]
    fn test_replace_existing_pyrefly_config() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let pyproject_path = tmp.path().join("pyproject.toml");

        let existing_content = r#"[project]
name = "test-project"
version = "0.1.0"

[tool.poetry]
dependencies = { python = "^3.8" }

[tool.pyrefly]
project_includes = ["old/path/**/*.py"]
project_excludes = ["should/be/removed.py"]

[tool.black]
line-length = 88
"#;
        fs_anyhow::write(&pyproject_path, existing_content)?;

        let config = ConfigFile {
            project_includes: Globs::new(vec!["new/path/**/*.py".to_owned()]),
            ..Default::default()
        };
        PyProject::update(&pyproject_path, config)?;

        let updated_content = fs_anyhow::read_to_string(&pyproject_path)?;

        assert!(updated_content.contains("[project]"));
        assert!(updated_content.contains("name = \"test-project\""));
        assert!(updated_content.contains("[tool.poetry]"));
        assert!(updated_content.contains("[tool.black]"));

        assert!(updated_content.contains("[tool.pyrefly]"));
        assert!(updated_content.contains("project-includes = [\"new/path/**/*.py\"]"));
        assert!(!updated_content.contains("project_includes = [\"old/path/**/*.py\"]"));
        assert!(!updated_content.contains("project_excludes"));

        Ok(())
    }

    #[test]
    fn test_pyrefly_section_ordering() -> anyhow::Result<()> {
        let tmp = tempfile::tempdir()?;
        let ordering_path = tmp.path().join("ordering_test.toml");
        let existing_content = r#"[project]
name = "test-project"
version = "0.1.0"

# Comment before tool section
[tool]
# Comment within tool section

[tool.black]
line-length = 88

[tool.pytest]
testpaths = ["tests"]

[tool.ruff]
line-length = 88

[build-system]
requires = ["setuptools"]
build-backend = "setuptools.build_meta"
"#;
        fs_anyhow::write(&ordering_path, existing_content)?;

        let config = ConfigFile {
            project_includes: Globs::new(vec!["ordering_test.py".to_owned()]),
            ..Default::default()
        };
        PyProject::update(&ordering_path, config)?;

        let toml_content = fs_anyhow::read_to_string(&ordering_path)?;

        let toml_expected = concat!(
            "[project]\n",
            "name = \"test-project\"\n",
            "version = \"0.1.0\"\n",
            "\n",
            "# Comment before tool section\n",
            "[tool]\n",
            "# Comment within tool section\n",
            "\n",
            "[tool.black]\n",
            "line-length = 88\n",
            "\n",
            "[tool.pytest]\n",
            "testpaths = [\"tests\"]\n",
            "\n",
            "[tool.ruff]\n",
            "line-length = 88\n",
            "\n",
            "[tool.pyrefly]\n",
            "project-includes = [\"ordering_test.py\"]\n",
            "\n",
            "[build-system]\n",
            "requires = [\"setuptools\"]\n",
            "build-backend = \"setuptools.build_meta\"\n",
        );

        assert_eq!(toml_content.trim(), toml_expected.trim());

        Ok(())
    }
}
