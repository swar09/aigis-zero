use std::{collections::HashMap, path::Path};

use tracing::{info, warn};

use crate::error::AppError;

pub struct TypedRuleCompiler;

impl TypedRuleCompiler {
    /// Compiles YARA rules from subdirectories of the rules directory.
    /// Each subdirectory name (process, network, file, auth) maps to an event type.
    /// Returns a map from event_type string to compiled yara_x::Rules.
    pub fn compile_all(rules_dir: &Path) -> Result<HashMap<String, yara_x::Rules>, AppError> {
        let mut rule_sets = HashMap::new();

        if !rules_dir.exists() {
            warn!(path = %rules_dir.display(), "Rules directory does not exist; returning empty rule set");
            return Ok(rule_sets);
        }

        for entry in std::fs::read_dir(rules_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let event_type = entry.file_name().to_string_lossy().to_string();
                if event_type == "mitre" {
                    continue;
                }
                let rules = Self::compile_directory(&path)?;
                info!(event_type = %event_type, path = %path.display(), "Compiled YARA rule set");
                rule_sets.insert(event_type, rules);
            }
        }

        Ok(rule_sets)
    }

    pub fn compile_directory(dir: &Path) -> Result<yara_x::Rules, AppError> {
        let mut compiler = yara_x::Compiler::new();
        let mut file_count = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && (ext == "yar" || ext == "yara")
            {
                let source = std::fs::read_to_string(&path)?;
                compiler
                    .add_source(source.as_str())
                    .map_err(|e| AppError::RuleCompilation {
                        source_file: path.display().to_string(),
                        line: 0,
                        message: e.to_string(),
                    })?;
                file_count += 1;
            }
        }

        info!(dir = %dir.display(), file_count, "Built YARA compiler source set");
        Ok(compiler.build())
    }

    pub fn compile_source(name: &str, source: &str) -> Result<yara_x::Rules, AppError> {
        let mut compiler = yara_x::Compiler::new();
        compiler.add_source(source).map_err(|e| AppError::RuleCompilation {
            source_file: name.to_string(),
            line: 0,
            message: e.to_string(),
        })?;
        Ok(compiler.build())
    }
}
