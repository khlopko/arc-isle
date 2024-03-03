use crate::parser::imports::detect;
use crate::parser::utils::as_str_or;
use crate::schema::{
    DataType, DataTypeDecl, ImportError, Primitive, PropertyDecl, StatusCode, TypeDecl,
    TypeDeclError, TypeDeclResults, TypeUsageMeta, UnknownType,
};
use std::collections::HashMap;
use yaml_rust::Yaml;

use crate::parser::utils::YamlHash;

pub struct TypesParser<'a> {
    pub parent_path: &'a str,
    pub types_usage: &'a mut HashMap<String, TypeUsageMeta>,
}

impl<'a> TypesParser<'a> {
    pub fn parse(&mut self, main: Yaml) -> Result<TypeDeclResults, TypeDeclError> {
        let mut results = Vec::new();
        let mut sources = Vec::new();
        let inner: Option<&YamlHash> = main.as_hash();
        let inner = inner.ok_or(TypeDeclError::ImportFailure(
            ImportError::InvalidInputSource,
        ))?;
        let imports = detect(inner, self.parent_path);
        for i in imports {
            sources.push(i);
        }
        sources.insert(0, Ok(main));
        for source in sources {
            match source {
                Ok(source) => self.parse_composed_source(&source, &mut results)?,
                Err(err) => results.push(Err(TypeDeclError::ImportFailure(err.clone()))),
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
        for (i, e) in source.iter().enumerate() {
            let (key, value) = e;
            let key = as_str_or(key, TypeDeclError::UnsupportedKeyType)?;
            if key == "_import" {
                continue;
            }
            let mut object_parser = TypeParser {
                key: &key,
                value: &value.as_hash().unwrap(),
                types_usage: &mut self.types_usage,
                source: TypeDeclSource::Type(i),
            };
            let result = object_parser.parse();
            output.push(result);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypeDeclSource {
    Type(usize),
    InterfaceInput(usize),
    InterfaceOutput(usize, StatusCode),
}

pub struct TypeParser<'a> {
    pub key: &'a str,
    pub value: &'a YamlHash,
    pub types_usage: &'a mut HashMap<String, TypeUsageMeta>,
    pub source: TypeDeclSource,
}

impl<'a> TypeParser<'a> {
    pub fn parse(&mut self) -> Result<TypeDecl, TypeDeclError> {
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
        self.types_usage.insert(self.key.to_string(), None);
        Ok(TypeDecl {
            name: self.key.to_string(),
            property_decls,
        })
    }

    fn make_data_type_decl(
        &mut self,
        raw_type: &Yaml,
        property_name: &str,
    ) -> Result<DataTypeDecl, TypeDeclError> {
        match raw_type {
            Yaml::String(string_value) => self.string_data_type_decl(string_value),
            Yaml::Hash(hash_value) => self.hash_data_type_decl(property_name, hash_value),
            _ => Err(TypeDeclError::UnsupportedTypeDeclaration),
        }
    }

    fn string_data_type_decl(&mut self, string_value: &str) -> Result<DataTypeDecl, TypeDeclError> {
        if string_value.is_empty() {
            return Err(TypeDeclError::EmptyTypeDeclaration);
        }
        let chars: Vec<char> = string_value.chars().collect();
        let mut last_read_index = 0;
        let mut type_name = String::new();
        while last_read_index < chars.len()
            && (chars[last_read_index].is_alphabetic()
                || chars[last_read_index] == '_'
                || last_read_index > 0 && chars[last_read_index].is_numeric())
        {
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
        &mut self,
        property_name: &str,
        hash_value: &YamlHash,
    ) -> Result<DataTypeDecl, TypeDeclError> {
        if hash_value.is_empty() {
            return Err(TypeDeclError::EmptyTypeDeclaration);
        }
        let mut parser = TypeParser {
            key: property_name,
            value: hash_value,
            types_usage: self.types_usage,
            source: self.source.clone(),
        };
        let object_decl = parser
            .parse()
            .map(|val| DataTypeDecl {
                data_type: DataType::ObjectDecl(val),
                is_required: true,
            })
            .map_err(|_| TypeDeclError::UnsupportedTypeDeclaration);
        object_decl
    }

    fn make_data_type(
        &mut self,
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
            "date_iso8601" => Ok(DataType::Primitive(Primitive::Str)),
            "url" => Ok(DataType::Primitive(Primitive::Str)),
            "timestamp" => Ok(DataType::Primitive(Primitive::Int)),
            "uuid" => Ok(DataType::Primitive(Primitive::Str)),
            other => {
                self.handle_if_unknown_type(other);
                Ok(DataType::Object(other.to_string()))
            }
        }
    }

    fn handle_if_unknown_type(&mut self, type_name: &str) {
        let meta = self.types_usage.get_mut(type_name);
        let make_unknown = || match &self.source {
            TypeDeclSource::Type(i) => UnknownType::InTypeDeclaration(*i, 0),
            TypeDeclSource::InterfaceInput(i) => UnknownType::InPayload(*i, 0),
            TypeDeclSource::InterfaceOutput(i, code) => UnknownType::InResponse(*i, code.clone(), 0),
        };
        match meta {
            Some(val) => match val {
                Some(val) => {
                    val.push(make_unknown());
                }
                None => {}
            },
            None => {
                self.types_usage.insert(type_name.to_string(), Some(vec![make_unknown()]));
            }
        }
    }

    fn make_dict_data_type(&mut self, subtypes: &Vec<String>) -> Result<DataType, TypeDeclError> {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        parser::types::{TypeDeclSource, TypeParser},
        schema::{DataType, DataTypeDecl, Primitive, PropertyDecl, TypeDecl},
    };
    use yaml_rust::Yaml;

    #[test]
    fn make_data_type_decl_for_str() {
        let key = "key".to_string();
        let value = Yaml::String("str".to_string());
        let mut parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
        let mut parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
        let mut parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
        let mut parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
        let mut parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
        let mut parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
        let mut parser = TypeParser {
            key: &key,
            value: &yaml_rust::yaml::Hash::new(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
        let mut parser = TypeParser {
            key: &key,
            value: &value.as_hash().unwrap(),
            types_usage: &mut HashMap::new(),
            source: TypeDeclSource::Type(0),
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
