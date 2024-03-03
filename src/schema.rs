// mod

use crate::parser::utils::ReadError;
use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display, Formatter},
};

pub struct Schema {
    pub hosts: Hosts,
    pub versioning: Versioning,
    pub types: TypeDeclResults,
    pub interfaces: InterfaceDeclResults,
}

impl Display for Schema {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = "Schema {\n".to_string();
        result.push_str(&format!("  hosts = {:?}\n", self.hosts));
        result.push_str(&format!("  versioning = {:?}\n", self.versioning));
        result.push_str(&format!(
            "  types = {}\n",
            self.types
                .iter()
                .map(|t| {
                    match t {
                        Ok(val) => format!("{}\n", val),
                        Err(err) => format!("{}\n", err),
                    }
                })
                .collect::<String>()
        ));
        result.push_str(&format!(
            "  interfaces = {}\n",
            self.interfaces
                .iter()
                .map(|t| {
                    match t {
                        Ok(val) => format!("{}\n", val),
                        Err(err) => format!("{}\n", err),
                    }
                })
                .collect::<String>()
        ));
        f.write_str(&result)
    }
}

#[derive(Debug)]
pub struct Host {
    pub env: String,
    pub address: String,
}

pub type Hosts = Vec<Host>;

#[derive(Debug)]
pub enum VersioningFormat {
    Headers,
}

#[derive(Debug)]
pub struct Versioning {
    pub format: VersioningFormat,
    pub header: Option<String>,
}

pub type TypeDeclResults = Vec<Result<TypeDecl, TypeDeclError>>;

#[derive(PartialEq, Clone, Debug)]
pub struct TypeDecl {
    pub name: String,
    pub property_decls: Vec<PropertyDecl>,
}

impl Display for TypeDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("type `{}` {{ ", self.name);
        for property_decl in &self.property_decls {
            match &property_decl.data_type_decl {
                Ok(data_type_decl) => {
                    result.push_str(&format!("{}: {}; ", property_decl.name, data_type_decl));
                }
                Err(err) => {
                    result.push_str(&format!("{}: {}; ", property_decl.name, err));
                }
            }
        }
        result.push_str(" }");
        f.write_str(&result)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PropertyDecl {
    pub name: String,
    pub data_type_decl: Result<DataTypeDecl, TypeDeclError>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypeDeclError {
    ImportFailure(ImportError),
    UnsupportedTypeDeclaration,
    UnsupportedKeyType,
    EmptyTypeDeclaration,
    SubtypeValuesEmptyDeclaration,
    UnsupportedPrimitive(String),
}

impl TypeDeclError {
    fn default_fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TypeDeclError::ImportFailure(import_error) => {
                write!(f, "Import failed: {}", import_error.to_string())
            }
            TypeDeclError::UnsupportedTypeDeclaration => {
                write!(f, "This type declaration format is not supported.")
            }
            TypeDeclError::UnsupportedKeyType => write!(f, "Key type must be string."),
            TypeDeclError::EmptyTypeDeclaration => write!(f, "Type declaration cannot be empty."),
            TypeDeclError::SubtypeValuesEmptyDeclaration => {
                write!(f, "Subtype declaration cannot be empty.")
            }
            TypeDeclError::UnsupportedPrimitive(value) => {
                write!(f, "Primitive {} not supported.", value)
            }
        }
    }
}

impl Error for TypeDeclError {}

impl Display for TypeDeclError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct DataTypeDecl {
    pub data_type: DataType,
    pub is_required: bool,
}

impl Display for DataTypeDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("{}", self.data_type);
        if !self.is_required {
            result.push_str("?");
        }
        f.write_str(&result)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum DataType {
    Primitive(Primitive),
    Array(Box<DataType>),
    Dict(Primitive, Box<DataType>),
    Object(String),
    ObjectDecl(TypeDecl),
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Primitive(primitive) => f.write_str(&format!("{}", primitive)),
            DataType::Array(data_type) => f.write_str(&format!("array[{}]", data_type)),
            DataType::Dict(key, value) => f.write_str(&format!("dict{{ {}: {} }}", key, value)),
            DataType::Object(ident) => f.write_str(&format!("{}", ident)),
            DataType::ObjectDecl(type_decl) => f.write_str(&format!("{}", type_decl)),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Primitive {
    Int,
    Double,
    Bool,
    Str,
}

impl Display for Primitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Primitive::Int => f.write_str("int"),
            Primitive::Double => f.write_str("double"),
            Primitive::Bool => f.write_str("bool"),
            Primitive::Str => f.write_str("str"),
        }
    }
}

#[derive(Clone)]
pub enum ImportError {
    IOError(ReadError),
    InvalidInputSource,
    InvalidImportValue,
}

impl PartialEq for ImportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ImportError::IOError(lhs), ImportError::IOError(rhs)) => lhs == rhs,
            (ImportError::InvalidInputSource, ImportError::InvalidInputSource) => true,
            (ImportError::InvalidImportValue, ImportError::InvalidImportValue) => true,
            _ => false,
        }
    }
}

// MARK - Interface

pub type InterfaceDeclResults = Vec<Result<InterfaceDecl, InterfaceDeclError>>;

#[derive(PartialEq, Debug)]
pub struct InterfaceDecl {
    pub ident: String,
    pub params: Vec<String>,
    pub spec: InterfaceSpec,
}

impl Display for InterfaceDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let InterfaceSpec::Api(api) = &self.spec;
        let mut result = format!("{:?} /{}\n", api.method, self.ident);
        if let Some(payload) = &api.payload {
            match payload {
                HttpPayload::Body(body) => {
                    result.push_str(&format!("Body: {:?}\n", body));
                }
                HttpPayload::Query(query) => {
                    result.push_str(&format!("Query: {:?}\n", query));
                }
            }
        }
        if let Some(responses) = &api.responses {
            result.push_str(&format!(
                "Responses: {}",
                responses
                    .iter()
                    .map(|(k, v)| { format!("{}: {}\n", k, v) })
                    .collect::<String>()
            ));
        }
        f.write_str(&result)
    }
}

#[derive(PartialEq)]
pub enum InterfaceSpec {
    Api(ApiSpec),
}

impl Debug for InterfaceSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InterfaceSpec::Api(api_spec) => f.write_str(&format!("{}", api_spec)),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct ApiSpec {
    pub method: HttpMethod,
    pub payload: Option<HttpPayload>,
    pub responses: HttpResponses,
}

impl Display for ApiSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("method: {}\n", self.method);
        if let Some(payload) = &self.payload {
            result.push_str(&format!("\t{}\n", payload));
        }
        if let Some(responses) = &self.responses {
            result.push_str(&format!("\t{:?}", responses));
        }
        f.write_str(&result)
    }
}

pub type HttpResponses = Option<HashMap<StatusCode, TypeDecl>>;

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum StatusCode {
    Fixed(u16),
    Prefix(u16),
}

impl StatusCode {
    pub fn as_key(&self) -> String {
        match self {
            StatusCode::Fixed(val) => val.to_string(),
            StatusCode::Prefix(val) => val.to_string() + "xx",
        }
    }
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusCode::Fixed(val) => f.write_str(&val.to_string()),
            StatusCode::Prefix(val) => f.write_str(&(val.to_string() + "xx")),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => f.write_str("GET"),
            HttpMethod::Post => f.write_str("POST"),
            HttpMethod::Put => f.write_str("PUT"),
            HttpMethod::Delete => f.write_str("DELETE"),
            HttpMethod::Patch => f.write_str("PATCH"),
            HttpMethod::Head => f.write_str("HEAD"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum HttpPayload {
    Query(Vec<PropertyDecl>),
    Body(Vec<PropertyDecl>),
}

impl Display for HttpPayload {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpPayload::Query(query) => {
                let mut result = "Query: {\n".to_string();
                for property_decl in query {
                    result.push_str(&format!(
                        "    {}: {:?}\n",
                        property_decl.name, property_decl.data_type_decl
                    ));
                }
                result.push_str("}\n");
                f.write_str(&result)
            }
            HttpPayload::Body(body) => {
                let mut result = "Body: {\n".to_string();
                for property_decl in body {
                    result.push_str(&format!(
                        "    {}: {:?}\n",
                        property_decl.name, property_decl.data_type_decl
                    ));
                }
                result.push_str("}\n");
                f.write_str(&result)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum InterfaceDeclError {
    ImportFailure(ImportError),
    BodyNotAllowed,
    QueryNotAllowed,
    InvalidKey,
    InvalidStatusCode,
    TypeNotFound(String),
    InvalidResponseDeclaration,
    InvalidInterfaceDeclaration,
    InvalidIdent,
    EmptyParam,
    InvalidMethod,
    InvalidQuery,
    InvalidBody,
    InvalidResponseTypeDeclaration,
}

impl Error for InterfaceDeclError {}

impl Display for InterfaceDeclError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => f.write_str("InterfaceDeclError"),
        }
    }
}

pub type TypeUsageMeta = Option<UnknownType>;

#[derive(Debug, PartialEq)]
pub enum UnknownType {
    InTypeDeclaration(usize, usize),
    InPayload(usize),
    InResponse(StatusCode, usize)
}

