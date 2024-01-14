use std::collections::{HashMap, HashSet};

use yaml_rust::yaml::Hash;
use yaml_rust::Yaml;

use crate::schema::{
    ApiSpec, HttpMethod, HttpPayload, HttpResponses, InterfaceDecl, InterfaceDeclError,
    InterfaceDeclResults, InterfaceSpec, StatusCode, TypeDecl, TypeDeclResults,
};

use super::{imports::detect, types::TypeParser};

type KnownTypes<'a> = HashMap<&'a String, &'a TypeDecl>;

pub struct InterfacesParser<'a> {
    main: &'a Yaml,
    parent_path: &'a str,
    known_types_set: &'a HashSet<String>,
    known_types: KnownTypes<'a>,
}

impl<'a> InterfacesParser<'a> {
    pub fn new(
        main: &'a Yaml,
        parent_path: &'a str,
        known_types: &'a HashSet<String>,
        type_decls: &'a TypeDeclResults,
    ) -> InterfacesParser<'a> {
        let pairs = type_decls.iter().filter_map(|item| {
            if let Ok(item) = item {
                return Some((&item.name, item));
            }
            None
        });
        InterfacesParser {
            main,
            parent_path,
            known_types_set: known_types,
            known_types: HashMap::from_iter(pairs),
        }
    }

    pub fn parse(&self) -> InterfaceDeclResults {
        let mut sources = Vec::new();
        sources.push(Ok(self.main.clone()));
        if let Ok(imports) = detect(&self.main, self.parent_path) {
            sources.extend(imports);
        }
        let mut results = Vec::new();
        let interface_parser = InterfaceParser {
            knows_types_set: &self.known_types_set,
            known_types: &self.known_types,
        };
        for source in sources {
            match source {
                Ok(source) => {
                    let raw = from_file(&source).unwrap();
                    for item in raw {
                        match item {
                            Ok(item) => {
                                if item.contains_key(&key_from("_import")) {
                                    continue;
                                }
                                let decl = interface_parser.parse(&item);
                                results.push(decl);
                            }
                            Err(err) => results.push(Err(err)),
                        }
                    }
                }
                Err(err) => results.push(Err(InterfaceDeclError::ImportFailure(err))),
            }
        }
        results
    }
}

struct InterfaceParser<'a> {
    knows_types_set: &'a HashSet<String>,
    known_types: &'a KnownTypes<'a>,
}

impl<'a> InterfaceParser<'a> {
    fn parse(&self, hash: &Hash) -> Result<InterfaceDecl, InterfaceDeclError> {
        let ident = get_ident(hash)?;
        let params = get_params(&ident)?;
        let method = get_method(hash)?;
        let payload = self.get_payload(&method, &hash)?;
        let responses = self.get_response(&hash)?;
        let api_spec = ApiSpec {
            method,
            payload,
            responses,
        };
        let spec = InterfaceSpec::Api(api_spec);
        let decl = InterfaceDecl {
            ident,
            params,
            spec,
        };
        Ok(decl)
    }

    fn get_response(&self, hash: &Hash) -> Result<HttpResponses, InterfaceDeclError> {
        let response_key = key_from("response");
        if !hash.contains_key(&response_key) {
            return Ok(None);
        }
        match &hash[&response_key] {
            Yaml::Hash(val) => self.responses_from(val),
            Yaml::String(name) => {
                let type_decl = self.known_types.get(name);
                match type_decl {
                    Some(type_decl) => {
                        let type_decl = TypeDecl {
                            name: name.clone(),
                            property_decls: type_decl.property_decls.clone(),
                        };
                        Ok(Some(HashMap::from([(StatusCode::Fixed(200), type_decl)])))
                    }
                    None => Err(InterfaceDeclError::TypeNotFound(name.clone())),
                }
            }
            _ => Err(InterfaceDeclError::InvalidResponseDeclaration),
        }
    }

    fn responses_from(&self, hash: &Hash) -> Result<HttpResponses, InterfaceDeclError> {
        if self.has_custom_response_codes(hash) {
            return self.custom_responses(hash);
        }
        let status_code = StatusCode::Fixed(200);
        let key = &status_code.as_key();
        let single_response = HashMap::from([(status_code, self.parse_response(key, hash)?)]);
        Ok(Some(single_response))
    }

    fn has_custom_response_codes(&self, hash: &Hash) -> bool {
        hash.keys()
            .find(|key| {
                key.as_str().map_or(false, |key| {
                    key.chars().next().map_or(false, |x| x.is_digit(10))
                })
            })
            .is_some()
    }

    fn custom_responses(&self, hash: &Hash) -> Result<HttpResponses, InterfaceDeclError> {
        let mut responses = HashMap::new();
        for (key, value) in hash {
            let key = match key {
                Yaml::String(val) => Ok(val.to_string()),
                Yaml::Integer(val) => Ok(val.to_string()),
                _ => Err(InterfaceDeclError::InvalidKey),
            }?;
            let fixed_code: Result<u16, _> = key.parse();
            let status_code = match fixed_code {
                Ok(code) => StatusCode::Fixed(code),
                Err(_) => self.as_status_code_pattern(&key)?,
            };
            let type_decl = self.response_type_decl(value)?;
            responses.insert(status_code, type_decl);
        }
        Ok(Some(responses))
    }

    fn as_status_code_pattern(&self, key: &str) -> Result<StatusCode, InterfaceDeclError> {
        let first = key.chars().next();
        let val = first.ok_or(InterfaceDeclError::InvalidStatusCode)?;
        let num = val
            .to_digit(10)
            .ok_or(InterfaceDeclError::InvalidStatusCode)?;
        let num: u16 = num
            .try_into()
            .map_err(|_| InterfaceDeclError::InvalidStatusCode)?;
        Ok(StatusCode::Prefix(num))
    }

    fn response_type_decl(&self, hash: &Yaml) -> Result<TypeDecl, InterfaceDeclError> {
        match hash {
            Yaml::Hash(val) => self.parse_response("200", val),
            Yaml::String(name) => {
                let type_decl = self.known_types.get(name);
                match type_decl {
                    Some(type_decl) => {
                        let type_decl = TypeDecl {
                            name: name.clone(),
                            property_decls: type_decl.property_decls.clone(),
                        };
                        Ok(type_decl)
                    }
                    None => Err(InterfaceDeclError::TypeNotFound(name.clone())),
                }
            }
            _ => Err(InterfaceDeclError::InvalidResponseDeclaration),
        }
    }

    fn parse_response(&self, key: &str, hash: &Hash) -> Result<TypeDecl, InterfaceDeclError> {
        TypeParser {
            key,
            value: hash,
            known_types: &self.knows_types_set,
        }
        .parse()
        .map_err(|_| InterfaceDeclError::InvalidResponseTypeDeclaration)
    }

    fn get_payload(
        &self,
        method: &HttpMethod,
        hash: &Hash,
    ) -> Result<Option<HttpPayload>, InterfaceDeclError> {
        match method {
            HttpMethod::Get | HttpMethod::Head => {
                if hash.contains_key(&key_from("body")) {
                    return Err(InterfaceDeclError::BodyNotAllowed);
                }
                return self.get_query_if_has(hash);
            }
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => {
                if hash.contains_key(&key_from("query")) {
                    return Err(InterfaceDeclError::QueryNotAllowed);
                }
                return self.get_body_if_has(hash);
            }
            HttpMethod::Delete => {
                if hash.contains_key(&key_from("query")) {
                    return Err(InterfaceDeclError::QueryNotAllowed);
                }
                if hash.contains_key(&key_from("body")) {
                    return Err(InterfaceDeclError::BodyNotAllowed);
                }
                return Ok(None);
            }
        }
    }

    fn get_query_if_has(&self, hash: &Hash) -> Result<Option<HttpPayload>, InterfaceDeclError> {
        let query_key = key_from("query");
        if !hash.contains_key(&query_key) {
            return Ok(None);
        }
        let raw_query = hash[&query_key]
            .as_hash()
            .ok_or(InterfaceDeclError::InvalidQuery)?;
        let parser = TypeParser {
            key: &query_key.as_str().unwrap(),
            value: raw_query,
            known_types: &self.knows_types_set,
        };
        let query = parser
            .parse()
            .map_err(|_| InterfaceDeclError::InvalidQuery)?;
        let payload_value = HttpPayload::Query(query.property_decls);
        Ok(Some(payload_value))
    }

    fn get_body_if_has(&self, hash: &Hash) -> Result<Option<HttpPayload>, InterfaceDeclError> {
        let body_key = key_from("body");
        if !hash.contains_key(&body_key) {
            return Ok(None);
        }
        let raw_body = hash[&body_key]
            .as_hash()
            .ok_or(InterfaceDeclError::InvalidBody)?;
        let parser = TypeParser {
            key: &body_key.as_str().unwrap(),
            value: raw_body,
            known_types: &self.knows_types_set,
        };
        let body = parser
            .parse()
            .map_err(|_| InterfaceDeclError::InvalidBody)?;
        let payload_value = HttpPayload::Body(body.property_decls);
        Ok(Some(payload_value))
    }
}

fn from_file(source: &Yaml) -> Result<Vec<Result<Hash, InterfaceDeclError>>, String> {
    if let Some(source) = source.as_vec() {
        return Ok(source.iter().map(|item| read_decl(item)).collect());
    }
    if let Some(source) = source.as_hash() {
        return Ok(from_hash(source));
    }
    Err("invalid source".to_string())
}

fn from_hash(source: &Hash) -> Vec<Result<Hash, InterfaceDeclError>> {
    let key = Yaml::from_str("declarations");
    source[&key]
        .as_vec()
        .unwrap()
        .iter()
        .map(|item| read_decl(item))
        .filter(|item| is_import(item))
        .collect()
}

fn read_decl(item: &Yaml) -> Result<Hash, InterfaceDeclError> {
    item.as_hash()
        .ok_or(InterfaceDeclError::InvalidInterfaceDeclaration)
        .cloned()
}

fn is_import(item: &Result<Hash, InterfaceDeclError>) -> bool {
    if item.is_err() {
        return false;
    }
    item.as_ref()
        .is_ok_and(|val| !val.contains_key(&Yaml::from_str("_import")))
}

fn get_ident(hash: &Hash) -> Result<String, InterfaceDeclError> {
    Ok(hash[&Yaml::from_str("path")]
        .as_str()
        .ok_or(InterfaceDeclError::InvalidIdent)?
        .to_string())
}

fn get_params(ident: &str) -> Result<Vec<String>, InterfaceDeclError> {
    let mut params = Vec::new();
    let mut param = String::new();
    let mut reading_param = false;
    for c in ident.chars() {
        if c == '{' {
            reading_param = true;
            continue;
        }
        if c == '}' {
            reading_param = false;
            if param.is_empty() {
                return Err(InterfaceDeclError::EmptyParam);
            }
            params.push(param.clone());
            param.clear();
            continue;
        }
        if reading_param {
            param.push(c);
        }
    }
    Ok(params)
}

fn get_method(hash: &Hash) -> Result<HttpMethod, InterfaceDeclError> {
    let raw_method = hash[&Yaml::from_str("method")]
        .as_str()
        .ok_or(InterfaceDeclError::InvalidMethod)?;
    match raw_method {
        "get" => Ok(HttpMethod::Get),
        "post" => Ok(HttpMethod::Post),
        "put" => Ok(HttpMethod::Put),
        "delete" => Ok(HttpMethod::Delete),
        "head" => Ok(HttpMethod::Head),
        "patch" => Ok(HttpMethod::Patch),
        /*"options" => Ok(HttpMethod::Options),
        "trace" => Ok(HttpMethod::Trace),
        "connect" => Ok(HttpMethod::Connect),*/
        _ => Err(InterfaceDeclError::InvalidMethod),
    }
}

fn key_from(value: &str) -> Yaml {
    Yaml::from_str(value)
}

#[cfg(test)]
mod tests {
    use yaml_rust::yaml::Hash;
    use yaml_rust::Yaml;

    use crate::{
        parser::interfaces::InterfaceParser,
        schema::{ApiSpec, HttpMethod, InterfaceDecl, InterfaceSpec, PropertyDecl},
    };

    #[test]
    fn minimal_get() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("get"));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news".to_string(),
                params: vec![],
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HttpMethod::Get,
                    payload: None,
                    responses: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn get_with_query() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("get"));
        let mut query = Hash::new();
        query.insert(Yaml::from_str("page"), Yaml::from_str("int"));
        query.insert(Yaml::from_str("limit"), Yaml::from_str("int?"));
        hash.insert(Yaml::from_str("query"), Yaml::Hash(query));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news".to_string(),
                params: vec![],
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HttpMethod::Get,
                    payload: Some(super::HttpPayload::Query(vec![
                        PropertyDecl {
                            name: "page".to_string(),
                            data_type_decl: Ok(crate::schema::DataTypeDecl {
                                data_type: crate::schema::DataType::Primitive(
                                    crate::schema::Primitive::Int
                                ),
                                is_required: true
                            })
                        },
                        PropertyDecl {
                            name: "limit".to_string(),
                            data_type_decl: Ok(crate::schema::DataTypeDecl {
                                data_type: crate::schema::DataType::Primitive(
                                    crate::schema::Primitive::Int
                                ),
                                is_required: false
                            })
                        }
                    ])),
                    responses: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn body_prohibited_on_get() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("get"));
        let mut body = Hash::new();
        body.insert(Yaml::from_str("title"), Yaml::from_str("str"));
        hash.insert(Yaml::from_str("body"), Yaml::Hash(body));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Err(crate::schema::InterfaceDeclError::BodyNotAllowed),
            result
        );
    }

    #[test]
    fn make_simplest_post() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news/post"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("post"));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news/post".to_string(),
                params: vec![],
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HttpMethod::Post,
                    payload: None,
                    responses: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn post_with_body() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news/post"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("post"));
        let mut body = Hash::new();
        body.insert(Yaml::from_str("title"), Yaml::from_str("str"));
        hash.insert(Yaml::from_str("body"), Yaml::Hash(body));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news/post".to_string(),
                params: vec![],
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HttpMethod::Post,
                    payload: Some(super::HttpPayload::Body(vec![PropertyDecl {
                        name: "title".to_string(),
                        data_type_decl: Ok(crate::schema::DataTypeDecl {
                            data_type: crate::schema::DataType::Primitive(
                                crate::schema::Primitive::Str
                            ),
                            is_required: true
                        })
                    }])),
                    responses: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn not_allowed_post_with_query() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news/post"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("post"));
        let mut query = Hash::new();
        query.insert(Yaml::from_str("page"), Yaml::from_str("int"));
        query.insert(Yaml::from_str("limit"), Yaml::from_str("int?"));
        hash.insert(Yaml::from_str("query"), Yaml::Hash(query));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Err(crate::schema::InterfaceDeclError::QueryNotAllowed),
            result
        );
    }

    #[test]
    fn make_simplest_put() {
        let mut hash = Hash::new();
        hash.insert(Yaml::from_str("path"), Yaml::from_str("news/post"));
        hash.insert(Yaml::from_str("method"), Yaml::from_str("put"));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news/post".to_string(),
                params: vec![],
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HttpMethod::Put,
                    payload: None,
                    responses: None,
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
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Ok(InterfaceDecl {
                ident: "news/post/{post_id}".to_string(),
                params: vec!["post_id".to_string()],
                spec: InterfaceSpec::Api(ApiSpec {
                    method: HttpMethod::Delete,
                    payload: None,
                    responses: None,
                }),
            }),
            result
        );
    }

    #[test]
    fn not_allowed_delete_with_query() {
        let mut hash = Hash::new();
        hash.insert(
            Yaml::from_str("path"),
            Yaml::from_str("news/post/{post_id}"),
        );
        hash.insert(Yaml::from_str("method"), Yaml::from_str("delete"));
        let mut query = Hash::new();
        query.insert(Yaml::from_str("page"), Yaml::from_str("int"));
        query.insert(Yaml::from_str("limit"), Yaml::from_str("int?"));
        hash.insert(Yaml::from_str("query"), Yaml::Hash(query));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Err(crate::schema::InterfaceDeclError::QueryNotAllowed),
            result
        );
    }

    #[test]
    fn not_allowed_delete_with_body() {
        let mut hash = Hash::new();
        hash.insert(
            Yaml::from_str("path"),
            Yaml::from_str("news/post/{post_id}"),
        );
        hash.insert(Yaml::from_str("method"), Yaml::from_str("delete"));
        let mut body = Hash::new();
        body.insert(Yaml::from_str("title"), Yaml::from_str("str"));
        hash.insert(Yaml::from_str("body"), Yaml::Hash(body));
        let parser = InterfaceParser {
            known_types: &std::collections::HashMap::new(),
        };

        let result = parser.parse(&hash);

        assert_eq!(
            Err(crate::schema::InterfaceDeclError::BodyNotAllowed),
            result
        );
    }
}
