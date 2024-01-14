use crate::parser::imports::detect;
use crate::parser::utils::as_str_or;
use crate::schema::{
    DataType, DataTypeDecl, Primitive, PropertyDecl, TypeDecl, TypeDeclError, TypeDeclResults,
};
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use yaml_rust::yaml::Hash;
use yaml_rust::Yaml;

pub struct TypesParser<'a> {
    pub main: &'a Yaml,
    pub parent_path: &'a str,
    pub known_types: &'a mut HashSet<String>
}

impl<'a> TypesParser<'a> {
    pub fn parse(&mut self) -> Result<TypeDeclResults, TypeDeclError> {
        let mut results = Vec::new();
        let mut sources = Vec::new();
        sources.push(Ok(self.main.clone()));
        if let Ok(imports) = detect(&self.main, self.parent_path) {
            sources.extend(imports);
        }
        for source in sources {
            match source {
                Ok(source) => self.parse_composed_source(&source, &mut results)?,
                Err(err) => results.push(Err(TypeDeclError::ImportFailure(err))),
            }
        }
        Ok(results)
    }

    fn parse_composed_source(
        &mut self,
        source: &Yaml,
        output: &mut TypeDeclResults,
    ) -> Result<(), TypeDeclError> {
        let source = source
            .as_hash()
            .ok_or(TypeDeclError::UnsupportedTypeDeclaration)?;
        for (key, value) in source {
            let key = as_str_or(key, TypeDeclError::UnsupportedKeyType)?;
            if key == "_import" {
                continue;
            }
            self.known_types.insert(key.clone());
            let object_parser = TypeParser {
                key: &key,
                value: &value.as_hash().unwrap(),
                known_types: &self.known_types
            };
            let result = object_parser.parse();
            output.push(result);
        }
        Ok(())
    }
}

pub struct TypeParser<'a> {
    pub key: &'a str,
    pub value: &'a Hash,
    pub known_types: &'a HashSet<String>
}

impl<'a> TypeParser<'a> {
    pub fn parse(&self) -> Result<TypeDecl, TypeDeclError> {
        let mut property_decls = Vec::new();
        for (property_name, property_type) in self.value.iter() {
            let property_name = as_str_or(property_name, TypeDeclError::UnsupportedKeyType)?;
            let data_type_decl = self.make_data_type_decl(property_type, &property_name);
            let property_decl = PropertyDecl {
                name: property_name,
                data_type_decl,
            };
            property_decls.push(property_decl);
        }
        Ok(TypeDecl {
            name: self.key.to_string(),
            property_decls,
        })
    }

    fn make_data_type_decl(
        &self,
        raw_type: &Yaml,
        property_name: &str,
    ) -> Result<DataTypeDecl, TypeDeclError> {
        match raw_type {
            Yaml::String(string_value) => self.string_data_type_decl(string_value),
            Yaml::Hash(hash_value) => self.hash_data_type_decl(property_name, hash_value),
            _ => Err(TypeDeclError::UnsupportedTypeDeclaration),
        }
    }

    fn string_data_type_decl(&self, string_value: &str) -> Result<DataTypeDecl, TypeDeclError> {
        if string_value.is_empty() {
            return Err(TypeDeclError::EmptyTypeDeclaration);
        }
        let chars: Vec<char> = string_value.chars().collect();
        let mut last_read_index = 0;
        let mut type_name = String::new();
        while last_read_index < chars.len() && chars[last_read_index].is_alphabetic() {
            type_name.push(chars[last_read_index]);
            last_read_index += 1;
        }
        if type_name.is_empty() {
            return Err(TypeDeclError::EmptyTypeDeclaration);
        }
        if last_read_index >= chars.len() {
            let data_type = self.make_data_type(&type_name, &Vec::new())?;
            return Ok(DataTypeDecl {
                data_type,
                is_required: true,
            });
        }
        let subtypes = self.subtypes(&chars, &mut last_read_index)?;
        let data_type = self.make_data_type(&type_name, &subtypes)?;
        let mut is_required = true;
        if last_read_index >= chars.len() {
            return Ok(DataTypeDecl {
                data_type,
                is_required: true,
            });
        }
        if chars[last_read_index] == '?' {
            is_required = false
        }
        Ok(DataTypeDecl {
            data_type,
            is_required,
        })
    }

    fn hash_data_type_decl(
        &self,
        property_name: &str,
        hash_value: &Hash,
    ) -> Result<DataTypeDecl, TypeDeclError> {
        if hash_value.is_empty() {
            return Err(TypeDeclError::EmptyTypeDeclaration);
        }
        let parser = TypeParser {
            key: property_name,
            value: hash_value,
            known_types: self.known_types
        };
        let object_decl = parser
            .parse()
            .map(|val| DataTypeDecl {
                data_type: DataType::ObjectDecl(val),
                is_required: true,
            })
            .map_err(|_| TypeDeclError::UnsupportedTypeDeclaration);
        return object_decl;
    }

    fn make_data_type(
        &self,
        type_name: &str,
        subtypes: &Vec<String>,
    ) -> Result<DataType, TypeDeclError> {
        match self.make_primitive(type_name) {
            Ok(primitive) => return Ok(DataType::Primitive(primitive)),
            Err(_) => {}
        };
        match type_name {
            "array" => {
                let contained_type = self.make_data_type(&subtypes[0], &Vec::new())?;
                Ok(DataType::Array(Box::new(contained_type)))
            }
            "dict" => self.make_dict_data_type(subtypes),
            other => Ok(DataType::Object(other.to_string())),
        }
    }

    fn make_dict_data_type(&self, subtypes: &Vec<String>) -> Result<DataType, TypeDeclError> {
        let key = self.make_primitive(&subtypes[0])?;
        let mut value_type_name: &str = &subtypes[1];
        let value_subtypes: Vec<String>;
        if let Some(mut start_index) = value_type_name.find("[") {
            value_type_name = &value_type_name[..start_index];
            value_subtypes = self.subtypes(&subtypes[1].chars().collect(), &mut start_index)?;
        } else {
            value_subtypes = Vec::new();
        }
        let value = self.make_data_type(value_type_name, &value_subtypes)?;
        Ok(DataType::Dict(key, Box::new(value)))
    }

    fn subtypes(&self, chars: &Vec<char>, index: &mut usize) -> Result<Vec<String>, TypeDeclError> {
        let mut _i = *index;
        let mut subtypes: Vec<String> = Vec::new();
        if chars[_i] == '[' {
            let mut n_open_braces = 1;
            _i += 1; // advance over opening brace
            let mut subtype_value = String::new();
            while _i < chars.len() {
                if chars[_i] == ']' {
                    if n_open_braces == 1 {
                        subtypes.push(subtype_value.clone());
                        break;
                    } else {
                        n_open_braces -= 1;
                    }
                }
                if chars[_i] == ',' {
                    subtypes.push(subtype_value.clone());
                    subtype_value = String::new();
                    _i += 2;
                }
                if chars[_i] == '[' {
                    n_open_braces += 1;
                }
                subtype_value.push(chars[_i]);
                _i += 1;
            }
            if !subtypes.iter().all(|e| !e.is_empty()) {
                return Err(TypeDeclError::SubtypeValuesEmptyDeclaration);
            }
            _i += 1;
        }
        *index = _i;
        Ok(subtypes)
    }

    fn make_primitive(&self, raw: &str) -> Result<Primitive, TypeDeclError> {
        match raw {
            "str" => Ok(Primitive::Str),
            "bool" => Ok(Primitive::Bool),
            "int" => Ok(Primitive::Int),
            "double" => Ok(Primitive::Double),
            other => Err(TypeDeclError::UnsupportedPrimitive(other.to_string())),
        }
    }
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{
        parser::types::TypeParser,
        schema::{DataType, DataTypeDecl, Primitive, PropertyDecl, TypeDecl},
    };
    use yaml_rust::Yaml;

    #[test]
    fn make_data_type_decl_for_str() {
        let key = "key".to_string();
        let value = Yaml::String("str".to_string());
        let parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Primitive(Primitive::Str),
            is_required: true,
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_optional_str() {
        let key = "key".to_string();
        let value = Yaml::String("str?".to_string());
        let parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            known_types: &HashSet::new()
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Primitive(Primitive::Str),
            is_required: false,
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_array() {
        let key = "key".to_string();
        let value = Yaml::String("array[int]".to_string());
        let parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            known_types: &HashSet::new()
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Array(Box::new(DataType::Primitive(Primitive::Int))),
            is_required: true,
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_optional_array() {
        let key = "key".to_string();
        let value = Yaml::String("array[int]?".to_string());
        let parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            known_types: &HashSet::new()
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Array(Box::new(DataType::Primitive(Primitive::Int))),
            is_required: false,
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_dict_with_primitives() {
        let key = "key".to_string();
        let value = Yaml::String("dict[int, str]?".to_string());
        let parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            known_types: &HashSet::new()
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Dict(
                Primitive::Int,
                Box::new(DataType::Primitive(Primitive::Str)),
            ),
            is_required: false,
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_dict_with_another_data_type() {
        let key = "key".to_string();
        let value = Yaml::String("dict[int, array[user]]?".to_string());
        let parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            known_types: &HashSet::new()
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, &key)
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Dict(
                Primitive::Int,
                Box::new(DataType::Array(Box::new(DataType::Object(
                    "user".to_string(),
                )))),
            ),
            is_required: false,
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_optional_object() {
        let key = "created_at".to_string();
        let value = Yaml::String("date?".to_string());
        let parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            known_types: &HashSet::new()
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, &key)
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Object("date".to_string()),
            is_required: false,
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_nested_declaration() {
        let key = "nested_object".to_string();
        let mut hash = yaml_rust::yaml::Hash::new();
        hash.insert(
            Yaml::String("id".to_string()),
            Yaml::String("str".to_string()),
        );
        hash.insert(
            Yaml::String("updated_at".to_string()),
            Yaml::String("date".to_string()),
        );
        hash.insert(
            Yaml::String("is_active".to_string()),
            Yaml::String("bool".to_string()),
        );
        let value = Yaml::Hash(hash);
        let parser = TypeParser {
            key: &key,
            value: &value.as_hash().unwrap(),
            known_types: &HashSet::new()
        };

        let data_type_decl = parser
            .make_data_type_decl(&value, &key)
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::ObjectDecl(TypeDecl {
                name: "nested_object".to_string(),
                property_decls: Vec::from([
                    PropertyDecl {
                        name: "id".to_string(),
                        data_type_decl: Ok(DataTypeDecl {
                            data_type: DataType::Primitive(Primitive::Str),
                            is_required: true,
                        }),
                    },
                    PropertyDecl {
                        name: "updated_at".to_string(),
                        data_type_decl: Ok(DataTypeDecl {
                            data_type: DataType::Object("date".to_string()),
                            is_required: true,
                        }),
                    },
                    PropertyDecl {
                        name: "is_active".to_string(),
                        data_type_decl: Ok(DataTypeDecl {
                            data_type: DataType::Primitive(Primitive::Bool),
                            is_required: true,
                        }),
                    },
                ]),
            }),
            is_required: true,
        };
        assert_eq!(expected, data_type_decl);
    }
}
