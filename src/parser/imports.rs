use crate::parser::utils::{as_str_or, read_yaml, YamlHash};
use crate::schema::ImportError;
use std::fmt::{Debug, Display, Formatter};
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct SourceImport {
    pub key: String,
    pub imported_source: Yaml,
}

pub fn detect(
    source: &Yaml,
    parent_path: &str,
) -> Result<Vec<Result<Yaml, ImportError>>, ImportError> {
    let mut found_imports = Vec::new();
    let inner: Option<&YamlHash> = source.as_hash();
    let inner = inner.ok_or(ImportError::InvalidInputSource)?;
    let import_key = &Yaml::String("_import".to_string());
    let is_import = inner.contains_key(import_key);
    if !is_import {
        return Ok(found_imports);
    }
    match &inner[import_key] {
        Yaml::String(file_path) => {
            let file_path = parent_path.to_string() + "/" + &file_path;
            match read_yaml(&file_path) {
                Ok(imported_yaml) => {
                    found_imports.push(Ok(imported_yaml[0].to_owned()))
                }
                Err(err) => found_imports.push(Err(ImportError::IOError(err))),
            }
        }
        Yaml::Array(file_paths) => {
            for file_path in file_paths {
                match as_str_or(file_path, ImportError::InvalidImportValue) {
                    Ok(file_path) => {
                        let file_path = parent_path.to_string() + "/" + &file_path;
                        match read_yaml(&file_path) {
                            Ok(imported_yaml) => {
                                found_imports.push(Ok(imported_yaml[0].to_owned()))
                            }
                            Err(err) => found_imports.push(Err(ImportError::IOError(err))),
                        }
                    }
                    Err(err) => found_imports.push(Err(err)),
                }
            }
        }
        _ => found_imports.push(Err(ImportError::InvalidImportValue)),
    }
    Ok(found_imports)
}

impl ImportError {
    fn default_fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ImportError::IOError(err) => {
                write!(f, "I/O error while loading imports: {}", err.to_string())
            }
            ImportError::InvalidInputSource => write!(f, "Input source should be a hashmap"),
            ImportError::InvalidImportValue => write!(f, "Import statement should be string"),
        }
    }
}

impl std::error::Error for ImportError {}

impl Display for ImportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}

impl Debug for ImportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}
