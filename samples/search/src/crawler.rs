use serde::Deserialize;
use serde_json::{Value};
use std::{cmp::min, error::Error};

// Used to represent values in prefix path
#[derive(Clone, PartialEq)]
pub enum PrefixValue {
    ObjectItemAny,
    ArrayItemAny,
    Key(String),
}

impl PrefixValue {
    pub fn is_key(&self) -> bool {
        return matches!(self, PrefixValue::Key(..));
    }
}

pub struct Prefix {
    prefix: Vec<PrefixValue>,
}

impl Prefix {
    pub fn get(&self, i: usize) -> &PrefixValue {
        return &self.prefix[i];
    }

    pub fn from_jpath(jpath: &String) -> Prefix {
        let mut prefix: Vec<PrefixValue> = Vec::new();
        let otokens: Vec<&str> = jpath.split("/").collect();
        let mut start = 0;
        if (*otokens.get(start).unwrap()).eq("") {
            start = 1;
        }
        for &otokens in &otokens[start..] {
            let ltokens: Vec<&str> = otokens.split("[").collect();
            if ltokens[0] == "*" {
                prefix.push(PrefixValue::ObjectItemAny)
            } else if ltokens[0] != "" {
                prefix.push(PrefixValue::Key(ltokens[0].to_string()));
            }

            for &ltoken in &ltokens[1..] {
                assert!(ltoken.chars().last().unwrap() == ']');

                let mut ltoken_slice = ltoken.chars();
                ltoken_slice.next_back();
                if ltoken_slice.as_str() != "*" {
                    prefix.push(PrefixValue::Key(ltoken_slice.as_str().to_string()));
                } else {
                    prefix.push(PrefixValue::ArrayItemAny)
                }
            }
        }
        Prefix { prefix }
    }

    pub fn to_jpath(&self) -> String {
        let mut str_elems: Vec<String> = Vec::new();
        for elem in &self.prefix {
            match elem {
                PrefixValue::Key(value) => str_elems.push(format!("/{}", value)),
                PrefixValue::ObjectItemAny => str_elems.push("/*".to_string()),
                PrefixValue::ArrayItemAny => str_elems.push("[*]".to_string()),
            }
        }
        if str_elems.is_empty() {
            return "/".to_string();
        }
        str_elems.join("")
    }

    pub fn common_ancestor(prefix_1: Prefix, prefix_2: Prefix) -> Prefix {
        let mut common_prefix: Vec<PrefixValue> = Vec::new();
        let min_length = min(prefix_1.prefix.len(), prefix_2.prefix.len());
        for i in 0..min_length {
            let mut is_match = false;
            let mut elem: Option<PrefixValue> = None;

            if *prefix_1.get(i) == *prefix_2.get(i) {
                is_match = true;
                elem = Some(prefix_1.get(i).clone());
            } else {
                if *prefix_1.get(i) == PrefixValue::ObjectItemAny && prefix_2.get(i).is_key() {
                    is_match = true;
                    elem = Some(prefix_2.get(i).clone());
                } else if *prefix_2.get(i) == PrefixValue::ObjectItemAny && prefix_1.get(i).is_key()
                {
                    is_match = true;
                    elem = Some(prefix_1.get(i).clone());
                }
            }
            if !is_match {
                break;
            }
            common_prefix.push(elem.unwrap());
        }
        Prefix {
            prefix: common_prefix,
        }
    }
}

#[derive(Deserialize)]
pub struct RootConfig {
    pub(crate) content: String,
    pub(crate) library: String,
}

#[derive(Deserialize)]
pub struct FabricConfig {
    pub(crate) policy: Value,
    pub(crate) root: RootConfig,
}

#[derive(Deserialize)]
pub struct IndexerConfig {
    #[serde(rename = "type")]
    pub(crate) indexer_type: String,
    pub(crate) document: Value,
    pub(crate) fields: Vec<FieldConfig>,
}

impl IndexerConfig {

    /**
     * Given a string representing the JSON index config, returns
     * and IndexerConfig value with proper fields filled out.
     */
    pub fn parse_index_config(
        config_value: &Value,
    ) -> Result<IndexerConfig, Box<dyn Error + Send + Sync>> {
        // Read config string as serde_json Value

        // Parse config into IndexerConfig
        let indexer_config_val: &Value = &config_value["indexer"];
        let indexer_arguments_val = &indexer_config_val["arguments"];
        let mut field_configs: Vec<FieldConfig> = Vec::new();
        for (field_name, field_value) in indexer_arguments_val["fields"].as_object().unwrap() {
            field_configs.push(FieldConfig {
                name: field_name.to_string(),
                options: field_value["options"].clone(),
                field_type: serde_json::from_value(field_value["type"].clone())?,
                paths: serde_json::from_value(field_value["paths"].clone())?,
            });
        }
        Ok(IndexerConfig {
            indexer_type: serde_json::from_value(indexer_config_val["type"].clone())?,
            document: indexer_arguments_val["document"].clone(),
            fields: field_configs,
        })
    }
}

#[derive(Deserialize)]
pub struct FieldConfig {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) field_type: String,
    options: Value,
    pub(crate) paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use test_utils::test_metadata::INDEX_CONFIG;

    #[test]
    fn test_parse_index_config() -> () {
        let index_object_meta: Value = serde_json::from_str(INDEX_CONFIG)
            .expect("Could not read index object into json value.");
        let config_value: &Value = &index_object_meta["indexer"]["config"];
        let indexer_config: IndexerConfig = IndexerConfig::parse_index_config(config_value)
            .expect("Could not parse indexer config.");

        /* Assert that indexer_config fields are correctly filled out. */
        assert_eq!(22, indexer_config.fields.len());
        assert_eq!("metadata-text", indexer_config.indexer_type);
        assert!(config_value["indexer"]["arguments"]["document"] == indexer_config.document);
    }
}
