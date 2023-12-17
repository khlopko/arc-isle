// mod

use std::fmt::{Debug, Formatter};
use crate::parser::utils::ReadError;

pub struct Schema {
    pub hosts: Hosts,
    pub versioning: Versioning,
    pub object_decl_results: ObjectDeclResults
}

impl Debug for Schema {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = "Schema {\n".to_string();
        result.push_str(&format!("  hosts = {:?}\n", self.hosts));
        result.push_str(&format!("  versioning = {:?}\n", self.versioning));
        result.push_str(&format!("  object_decl_results = {:?}\n", self.object_decl_results));
        f.write_str(&result)
    }
}

#[derive(Debug)]
pub struct Host {
    pub env: String,
    pub address: String
}

pub type Hosts = Vec<Host>;

#[derive(Debug)]
pub enum VersioningFormat {
    Headers,
}

#[derive(Debug)]
pub struct Versioning {
    pub format: VersioningFormat,
    pub header: Option<String>
}

pub type ObjectDeclResults = Vec<Result<ObjectDecl, ObjectError>>;

#[derive(PartialEq)]
pub struct ObjectDecl {
    pub name: String,
    pub property_decls: Vec<PropertyDecl>
}

impl Debug for ObjectDecl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = "ObjectDecl {\n".to_string();
        result.push_str(&format!("    name = {}\n", self.name));
        for property_decl in &self.property_decls {
            result.push_str(&format!("    {} = {:?}\n", property_decl.name, property_decl.data_type_decl));
        }
        f.write_str(&result)
    }
}

#[derive(Debug, PartialEq)]
pub struct PropertyDecl {
    pub name: String,
    pub data_type_decl: Result<DataTypeDecl, ObjectError>
}

#[derive(PartialEq)]
pub enum ObjectError {
    ImportFailure(ImportError),
    UnsupportedTypeDeclaration,
    UnsupportedKeyType,
    EmptyTypeDeclaration,
    SubtypeValuesEmptyDeclaration,
    UnsupportedPrimitive(String)
}

#[derive(Debug, PartialEq)]
pub struct DataTypeDecl {
    pub data_type: DataType,
    pub is_required: bool
}

#[derive(Debug, PartialEq)]
pub enum DataType {
    Primitive(Primitive),
    Array(Box<DataType>),
    Dict(Primitive, Box<DataType>),
    Object(String),
    ObjectDecl(ObjectDecl)
}

#[derive(Debug, PartialEq)]
pub enum Primitive {
    Int,
    Double,
    Bool,
    Str
}

pub enum ImportError {
    IOError(ReadError),
    InvalidInputSource,
    InvalidImportValue
}

impl PartialEq for ImportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                ImportError::IOError(either::Either::Left(_)),
                ImportError::IOError(either::Either::Left(_))
            ) => true,
            (
                ImportError::IOError(either::Either::Right(lhs)),
                ImportError::IOError(either::Either::Right(rhs))
            ) => lhs == rhs,
            (
                ImportError::InvalidInputSource,
                ImportError::InvalidInputSource
            ) => true,
            (
                ImportError::InvalidImportValue,
                ImportError::InvalidImportValue
            ) => true,
            _ => false
        }
    }
}
