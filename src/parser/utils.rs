use std::{fs, io};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

pub type YamlHash = yaml_rust::yaml::Hash;

#[derive(Debug)]
pub struct ReadError {
    pub internal_error: either::Either<io::Error, yaml_rust::ScanError>
}

impl std::error::Error for ReadError {
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl PartialEq for ReadError {
    fn eq(&self, other: &ReadError) -> bool {
        match (&self.internal_error, &other.internal_error) {
            (either::Either::Left(_), either::Either::Left(_)) => true,
            (either::Either::Right(lhs), either::Either::Right(rhs)) => lhs == rhs,
            _ => false
        }
    }
}

impl Clone for ReadError {
    fn clone(&self) -> Self {
        let internal_error = match &self.internal_error {
            either::Either::Left(err) => either::Either::Left(io::Error::new(err.kind(), err.to_string())),
            either::Either::Right(err) => either::Either::Right(err.clone())
        };
        Self { internal_error }
    }
}

pub fn read_yaml(file_path: &str) -> Result<Vec<Yaml>, ReadError> {
    let file_contents = fs::read_to_string(file_path)
        .map_err(|err| ReadError { internal_error: either::Either::Left(err) })?;
    let yaml: Vec<Yaml> = YamlLoader::load_from_str(&file_contents)
        .map_err(|err| ReadError { internal_error: either::Either::Right(err) })?;
    Ok(yaml)
}

pub fn serialize_to_string(yaml: &Yaml) -> String {
    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(yaml).unwrap(); // dump the YAML object to a String
    }
    out_str
}

pub fn as_str_or<Err>(yaml: &Yaml, err: Err) -> Result<String, Err> {
    let value: Option<&str> = yaml.as_str();
    let value = value.ok_or(err)?.to_string();
    Ok(value)
}
