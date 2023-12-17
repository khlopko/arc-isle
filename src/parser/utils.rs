use std::{fs, io};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

pub type YamlHash = yaml_rust::yaml::Hash;

pub type ReadError = either::Either<io::Error, yaml_rust::ScanError>;

pub fn read_yaml(file_path: &str) -> Result<Vec<Yaml>, ReadError> {
    let file_contents = fs::read_to_string(file_path)
        .map_err(|err| either::Either::Left(err))?;
    let yaml: Vec<Yaml> = YamlLoader::load_from_str(&file_contents)
        .map_err(|err| either::Either::Right(err))?;
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
