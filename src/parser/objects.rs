use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use yaml_rust::Yaml;
use yaml_rust::yaml::Hash;
use crate::parser::imports::detect;
use crate::parser::utils::as_str_or;
use crate::schema::{DataType, DataTypeDecl, ObjectDecl, ObjectDeclResults, ObjectError, Primitive, PropertyDecl};

pub struct ObjectsParser<'a> {
    pub main: &'a Yaml,
    pub parent_path: &'a str
}

impl<'a> ObjectsParser<'a> {
    pub fn parse(&self) -> Result<ObjectDeclResults, ObjectError> {
        let mut object_decl_results = Vec::new();
        let mut sources = Vec::new();
        sources.push(Ok(self.main.clone()));
        if let Ok(imports) = detect(&self.main, self.parent_path) {
            sources.extend(imports);
        }
        for source in sources {
            match source {
                Ok(source) =>
                    self.parse_composed_source(&source, &mut object_decl_results)?,
                Err(err) =>
                    object_decl_results.push(Err(ObjectError::ImportFailure(err)))
            }
        }
        Ok(object_decl_results)
    }

    fn parse_composed_source(
        &self,
        source: &Yaml,
        output: &mut ObjectDeclResults
    ) -> Result<(), ObjectError> {
        let source = source.as_hash().ok_or(ObjectError::UnsupportedTypeDeclaration)?;
        for (key, value) in source {
            let key = &as_str_or(key, ObjectError::UnsupportedKeyType)?;
            if key == "_import" {
                continue;
            }
            let object_parser = ObjectParser { key, value: &value.as_hash().unwrap() };
            let result = object_parser.parse();
            output.push(result);
        }
        Ok(())
    }
}

pub struct ObjectParser<'a> {
    pub key: &'a str,
    pub value: &'a Hash
}

fn type_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

impl<'a> ObjectParser<'a> {
    pub fn parse(&self) -> Result<ObjectDecl, ObjectError> {
        let mut property_decls = Vec::new();
        for (property_name, property_type) in self.value.iter() {
            let property_name = as_str_or(property_name, ObjectError::UnsupportedKeyType)?;
            let data_type_decl = self.make_data_type_decl(property_type, &property_name);
            let property_decl = PropertyDecl {
                name: property_name,
                data_type_decl
            };
            property_decls.push(property_decl);
        }
        Ok(ObjectDecl {
            name: self.key.to_string(),
            property_decls
        })
    }

    fn make_data_type_decl(
        &self,
        raw_type: &Yaml,
        property_name: &str
    ) -> Result<DataTypeDecl, ObjectError> {
        match raw_type {
            Yaml::String(string_value) => self.string_data_type_decl(string_value),
            Yaml::Hash(hash_value) => self.hash_data_type_decl(property_name, hash_value),
            _ => Err(ObjectError::UnsupportedTypeDeclaration)
        }
    }

    fn string_data_type_decl(&self, string_value: &str) -> Result<DataTypeDecl, ObjectError> {
        if string_value.is_empty() {
            return Err(ObjectError::EmptyTypeDeclaration);
        }
        let chars: Vec<char> = string_value.chars().collect();
        let mut last_read_index = 0;
        let mut type_name = String::new();
        while last_read_index < chars.len() && chars[last_read_index].is_alphabetic() {
            type_name.push(chars[last_read_index]);
            last_read_index += 1;
        }
        if type_name.is_empty() {
            return Err(ObjectError::EmptyTypeDeclaration);
        }
        if last_read_index >= chars.len() {
            let data_type = self.make_data_type(&type_name, &Vec::new())?;
            return Ok(DataTypeDecl { data_type, is_required: true });
        }
        let subtypes = self.subtypes(&chars, &mut last_read_index)?;
        let data_type = self.make_data_type(&type_name, &subtypes)?;
        let mut is_required = true;
        if last_read_index >= chars.len() {
            return Ok(DataTypeDecl { data_type, is_required: true });
        }
        if chars[last_read_index] == '?' {
            is_required = false
        }
        Ok(DataTypeDecl { data_type, is_required })
    }

    fn hash_data_type_decl(
        &self,
        property_name: &str,
        hash_value: &Hash
    ) -> Result<DataTypeDecl, ObjectError> {
        if hash_value.is_empty() {
            return Err(ObjectError::EmptyTypeDeclaration);
        }
        let parser = ObjectParser { key: property_name, value: hash_value };
        let object_decl = parser.parse()
            .map(|val| DataTypeDecl { data_type: DataType::ObjectDecl(val), is_required: true })
            .map_err(|_| ObjectError::UnsupportedTypeDeclaration);
        return object_decl;
    }

    fn make_data_type(
        &self,
        type_name: &str,
        subtypes: &Vec<String>
    ) -> Result<DataType, ObjectError> {
        match self.make_primitive(type_name) {
            Ok(primitive) => return Ok(DataType::Primitive(primitive)),
            Err(_) => {}
        };
        match type_name {
            "array" => {
                let contained_type = self.make_data_type(&subtypes[0], &Vec::new())?;
                Ok(DataType::Array(Box::new(contained_type)))
            },
            "dict" => {
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
            },
            other => Ok(DataType::Object(other.to_string()))
        }
    }

    fn subtypes(&self, chars: &Vec<char>, index: &mut usize) -> Result<Vec<String>, ObjectError> {
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
                return Err(ObjectError::SubtypeValuesEmptyDeclaration);
            }
            _i += 1;
        }
        *index = _i;
        Ok(subtypes)
    }

    fn make_primitive(&self, raw: &str) -> Result<Primitive, ObjectError> {
        match raw {
            "str" => Ok(Primitive::Str),
            "bool" => Ok(Primitive::Bool),
            "int" => Ok(Primitive::Int),
            "double" => Ok(Primitive::Double),
            other => Err(ObjectError::UnsupportedPrimitive(other.to_string()))
        }
    }
}

impl ObjectError {
    fn default_fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ObjectError::ImportFailure(import_error) =>
                write!(f, "Import failed: {}", import_error.to_string()),
            ObjectError::UnsupportedTypeDeclaration =>
                write!(f, "This type declaration format is not supported."),
            ObjectError::UnsupportedKeyType =>
                write!(f, "Key type must be string."),
            ObjectError::EmptyTypeDeclaration =>
                write!(f, "Type declaration cannot be empty."),
            ObjectError::SubtypeValuesEmptyDeclaration =>
                write!(f, "Subtype declaration cannot be empty."),
            ObjectError::UnsupportedPrimitive(value) =>
                write!(f, "Primitive {} not supported.", value)
        }
    }
}

impl Error for ObjectError {
}

impl Display for ObjectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}

impl Debug for ObjectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.default_fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use yaml_rust::Yaml;
    use crate::schema::{DataType, DataTypeDecl, ObjectDecl, Primitive, PropertyDecl};

    #[test]
    fn make_data_type_decl_for_str() {
        let key = "key".to_string();
        let value = Yaml::String("str".to_string());
        let parser = crate::parser::ObjectParser { key: &key, value: &yaml_rust::yaml::Hash::new() };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl { data_type: DataType::Primitive(Primitive::Str), is_required: true };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_optional_str() {
        let key = "key".to_string();
        let value = Yaml::String("str?".to_string());
        let parser = crate::parser::ObjectParser { key: &key, value: &yaml_rust::yaml::Hash::new() };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl { data_type: DataType::Primitive(Primitive::Str), is_required: false };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_array() {
        let key = "key".to_string();
        let value = Yaml::String("array[int]".to_string());
        let parser = crate::parser::ObjectParser { key: &key, value: &yaml_rust::yaml::Hash::new() };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Array(Box::new(DataType::Primitive(Primitive::Int))),
            is_required: true
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_optional_array() {
        let key = "key".to_string();
        let value = Yaml::String("array[int]?".to_string());
        let parser = crate::parser::ObjectParser { key: &key, value: &yaml_rust::yaml::Hash::new() };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Array(Box::new(DataType::Primitive(Primitive::Int))),
            is_required: false
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_dict_with_primitives() {
        let key = "key".to_string();
        let value = Yaml::String("dict[int, str]?".to_string());
        let parser = crate::parser::ObjectParser { key: &key, value: &yaml_rust::yaml::Hash::new() };

        let data_type_decl = parser
            .make_data_type_decl(&value, "")
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Dict(Primitive::Int, Box::new(DataType::Primitive(Primitive::Str))),
            is_required: false
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_dict_with_another_data_type() {
        let key = "key".to_string();
        let value = Yaml::String("dict[int, array[user]]?".to_string());
        let parser = crate::parser::ObjectParser { key: &key, value: &yaml_rust::yaml::Hash::new() };

        let data_type_decl = parser
            .make_data_type_decl(&value, &key)
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Dict(
                Primitive::Int,
                Box::new(DataType::Array(Box::new(DataType::Object("user".to_string()))))
            ),
            is_required: false
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_for_optional_object() {
        let key = "created_at".to_string();
        let value = Yaml::String("date?".to_string());
        let parser = crate::parser::ObjectParser { key: &key, value: &yaml_rust::yaml::Hash::new() };

        let data_type_decl = parser
            .make_data_type_decl(&value, &key)
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::Object("date".to_string()),
            is_required: false
        };
        assert_eq!(expected, data_type_decl);
    }

    #[test]
    fn make_data_type_decl_nested_declaration() {
        let key = "nested_object".to_string();
        let mut hash = yaml_rust::yaml::Hash::new();
        hash.insert(Yaml::String("id".to_string()), Yaml::String("str".to_string()));
        hash.insert(Yaml::String("updated_at".to_string()), Yaml::String("date".to_string()));
        hash.insert(Yaml::String("is_active".to_string()), Yaml::String("bool".to_string()));
        let value = Yaml::Hash(hash);
        let parser = crate::parser::ObjectParser { key: &key, value: &value.as_hash().unwrap() };

        let data_type_decl = parser
            .make_data_type_decl(&value, &key)
            .unwrap_or_else(|_| panic!("Expect to have an OK result"));

        let expected = DataTypeDecl {
            data_type: DataType::ObjectDecl(
                ObjectDecl {
                    name: "nested_object".to_string(),
                    property_decls: Vec::from([
                        PropertyDecl {
                            name: "id".to_string(),
                            data_type_decl: Ok(DataTypeDecl {
                                data_type: DataType::Primitive(Primitive::Str),
                                is_required: true
                            })
                        },
                        PropertyDecl {
                            name: "updated_at".to_string(),
                            data_type_decl: Ok(DataTypeDecl {
                                data_type: DataType::Object("date".to_string()),
                                is_required: true
                            })
                        },
                        PropertyDecl {
                            name: "is_active".to_string(),
                            data_type_decl: Ok(DataTypeDecl {
                                data_type: DataType::Primitive(Primitive::Bool),
                                is_required: true
                            })
                        }
                    ])
                }
            ),
            is_required: true
        };
        assert_eq!(expected, data_type_decl);
    }
}
