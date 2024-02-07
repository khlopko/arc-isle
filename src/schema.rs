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

impl Debug for Schema {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = "Schema {\n".to_string();
        result.push_str(&format!("  hosts = {:?}\n", self.hosts));
        result.push_str(&format!("  versioning = {:?}\n", self.versioning));
        result.push_str(&format!("  types = {:?}\n", self.types));
        result.push_str(&format!("  interfaces = {:?}\n", self.interfaces));
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

#[derive(PartialEq, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub property_decls: Vec<PropertyDecl>,
}

impl Debug for TypeDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("type {} {{\n", self.name);
        for property_decl in &self.property_decls {
            result.push_str(&format!(
                "    {}: {:?}\n",
                property_decl.name, property_decl.data_type_decl
            ));
        }
        result.push_str("}\n");
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

#[derive(PartialEq, Clone)]
pub struct DataTypeDecl {
    pub data_type: DataType,
    pub is_required: bool,
}

impl Debug for DataTypeDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("{:?}", self.data_type);
        if !self.is_required {
            result.push_str("?");
        }
        f.write_str(&result)
    }
}

#[derive(PartialEq, Clone)]
pub enum DataType {
    Primitive(Primitive),
    Array(Box<DataType>),
    Dict(Primitive, Box<DataType>),
    Object(String),
    ObjectDecl(TypeDecl),
}

impl Debug for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Primitive(primitive) => f.write_str(&format!("{:?}", primitive)),
            DataType::Array(data_type) => f.write_str(&format!("array[{:?}]", data_type)),
            DataType::Dict(key, value) => f.write_str(&format!("dict{{ {:?}: {:?} }}", key, value)),
            DataType::Object(ident) => f.write_str(&format!("{}", ident)),
            DataType::ObjectDecl(type_decl) => f.write_str(&format!("{:?}", type_decl)),
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

#[derive(PartialEq)]
pub struct InterfaceDecl {
    pub ident: String,
    pub params: Vec<String>,
    pub spec: InterfaceSpec,
}

impl Debug for InterfaceDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("{} {{\n", self.ident);
        result.push_str(&format!("\t{:?}\n", self.spec));
        result.push_str("}\n");
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
            InterfaceSpec::Api(api_spec) => f.write_str(&format!("{:?}", api_spec)),
        }
    }
}

#[derive(PartialEq)]
pub struct ApiSpec {
    pub method: HttpMethod,
    pub payload: Option<HttpPayload>,
    pub responses: HttpResponses,
}

impl Debug for ApiSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = format!("method: {:?}\n", self.method);
        if let Some(payload) = &self.payload {
            result.push_str(&format!("\t{:?}\n", payload));
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
            StatusCode::Prefix(val) => val.to_string() + "xx"
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

#[derive(Debug, PartialEq)]
pub enum HttpPayload {
    Query(Vec<PropertyDecl>),
    Body(Vec<PropertyDecl>),
}

#[derive(Debug, PartialEq)]
pub enum InterfaceDeclError {
    ImportFailure(ImportError),
    UnsupportedInterfaceDeclaration,
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
    InvalidResponseValue,
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
