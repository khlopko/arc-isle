use yaml_rust::yaml::Hash;
use yaml_rust::Yaml;

use crate::schema::{InterfaceDeclError, InterfaceDeclResults};

use super::imports::detect;

pub struct InterfacesParser<'a> {
    pub main: &'a Yaml,
    pub parent_path: &'a str,
}

impl<'a> InterfacesParser<'a> {
    pub fn parse(&self) -> InterfaceDeclResults {
        let mut sources = Vec::new();
        sources.push(Ok(self.main.clone()));
        if let Ok(imports) = detect(&self.main, self.parent_path) {
            sources.extend(imports);
        }
        let mut raw_decls: Vec<Result<Hash, InterfaceDeclError>> = Vec::new();
        let mut results = Vec::new();
        for source in sources {
            match source {
                Ok(source) => raw_decls.extend(self.from_file(&source).unwrap()),
                Err(err) => results.push(Err(InterfaceDeclError::ImportFailure(err))),
            }
        }
        println!("raw_decls = {:?}", raw_decls);
        results
    }

    fn from_file(&self, source: &Yaml) -> Result<Vec<Result<Hash, InterfaceDeclError>>, String> {
        if let Some(source) = source.as_vec() {
            return Ok(source
                .iter()
                .map(|item| self.read_decl(item))
                .collect());
        }
        if let Some(source) = source.as_hash() {
            return Ok(self.from_hash(source));
        }
        return Err("invalid source".to_string());
    }

    fn from_hash(&self, source: &Hash) -> Vec<Result<Hash, InterfaceDeclError>> {
        let key = Yaml::from_str("declarations");
        let filter_key = Yaml::from_str("_import");
        source[&key]
            .as_vec()
            .unwrap()
            .iter()
            .map(|item| self.read_decl(item))
            .filter(|item| {
                item.is_err()
                    || item
                        .as_ref()
                        .is_ok_and(|val| !val.contains_key(&filter_key))
            })
            .collect()
    }

    fn read_decl(&self, item: &Yaml) -> Result<Hash, InterfaceDeclError> {
        item.as_hash()
            .ok_or(InterfaceDeclError::UnsupportedInterfaceDeclaration)
            .cloned()
    }
}

pub struct InterfaceParser {}

#[cfg(test)]
mod tests {
    #[test]
    fn make_simples_get() {}
}
