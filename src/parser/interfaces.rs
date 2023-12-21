use std::collections::HashMap;

use yaml_rust::yaml::Hash;
use yaml_rust::Yaml;

use crate::schema::{
    ApiSpec, HTTPMethod, HttpPayload, InterfaceDecl, InterfaceDeclError, InterfaceDeclResults,
    InterfaceSpec,
};

use super::{imports::detect, types::TypeParser};

pub struct InterfacesParser<'a> {
    pub main: &'a Yaml,
    pub parent_path: &'a str,
}

impl<'a> InterfacesParser<'a> {
    const FILTER_KEY: &'static str = "_import";

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
            return Ok(source.iter().map(|item| self.read_decl(item)).collect());
        }
        if let Some(source) = source.as_hash() {
            return Ok(self.from_hash(source));
        }
        Err("invalid source".to_string())
    }

    fn from_hash(&self, source: &Hash) -> Vec<Result<Hash, InterfaceDeclError>> {
        let key = Yaml::from_str("declarations");
        source[&key]
            .as_vec()
            .unwrap()
            .iter()
            .map(|item| self.read_decl(item))
            .filter(|item| self.is_import(item))
            .collect()
    }

    fn read_decl(&self, item: &Yaml) -> Result<Hash, InterfaceDeclError> {
        item.as_hash()
            .ok_or(InterfaceDeclError::UnsupportedInterfaceDeclaration)
            .cloned()
    }

    fn is_import(&self, item: &Result<Hash, InterfaceDeclError>) -> bool {
        if item.is_err() {
            return false;
        }
        item.as_ref()
            .is_ok_and(|val| !val.contains_key(&Yaml::from_str(InterfacesParser::FILTER_KEY)))
    }
}

fn parse_declaration(hash: &Hash) -> Result<InterfaceDecl, InterfaceDeclError> {
    let ident = hash[&Yaml::from_str("path")]
        .as_str()
        .ok_or(InterfaceDeclError::UnsupportedInterfaceDeclaration)?
        .to_string();
    let raw_method = hash[&Yaml::from_str("method")]
        .as_str()
        .ok_or(InterfaceDeclError::UnsupportedInterfaceDeclaration)?;
    let method = map_raw_method(raw_method)?;
    let query_key = key_from("query");
    let mut payload = None;
    if hash.contains_key(&query_key) {
        let raw_query = hash[&query_key]
            .as_hash()
            .ok_or(InterfaceDeclError::UnsupportedInterfaceDeclaration)?;
        let parser = TypeParser {
            key: &query_key.as_str().unwrap(),
            value: raw_query,
        };
        let query = parser
            .parse()
            .map_err(|_| InterfaceDeclError::UnsupportedInterfaceDeclaration)?;
        let payload_value = HttpPayload::Query(query.property_decls);
        payload = Some(payload_value);
    }
    let api_spec = ApiSpec { method, payload };
    let spec = InterfaceSpec::Api(api_spec);
    let decl = InterfaceDecl { ident, spec };
    Ok(decl)
}

fn key_from(value: &str) -> Yaml {
    Yaml::from_str(value)
}

fn map_raw_method(raw_method: &str) -> Result<HTTPMethod, InterfaceDeclError> {
    match raw_method {
        "get" => Ok(HTTPMethod::Get),
        "post" => Ok(HTTPMethod::Post),
        "put" => Ok(HTTPMethod::Put),
        "delete" => Ok(HTTPMethod::Delete),
        /*"head" => Ok(HTTPMethod::Head),
        "options" => Ok(HTTPMethod::Options),
        "trace" => Ok(HTTPMethod::Trace),
        "connect" => Ok(HTTPMethod::Connect),*/
        _ => Err(InterfaceDeclError::UnsupportedInterfaceDeclaration),
    }
}

#[cfg(test)]
mod tests {
    use yaml_rust::yaml::Hash;
    use yaml_rust::Yaml;

    use crate::schema::{ApiSpec, HTTPMethod, InterfaceDecl, InterfaceSpec};

    #[test]
    fn make_simplest_get() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("get"));

        let result = super::parse_declaration(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news".to_string(),
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HTTPMethod::Get,
                    payload: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn make_simplest_post() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news/post"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("post"));

        let result = super::parse_declaration(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news/post".to_string(),
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HTTPMethod::Post,
                    payload: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn make_simplest_put() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news/post"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("put"));

        let result = super::parse_declaration(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news/post".to_string(),
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HTTPMethod::Put,
                    payload: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn make_simplest_delete() {
        let mut hash = Hash::new();
        hash.insert(
            Yaml::from_str("path"),
            Yaml::from_str("news/post/{post_id}"),
        );
        hash.insert(Yaml::from_str("method"), Yaml::from_str("delete"));

        let result = super::parse_declaration(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news/post/{post_id}".to_string(),
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HTTPMethod::Delete,
                    payload: None,
                }),
            }),
            result
        );
    }
}
