use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LLMCouncilRequestSchema {
    pub title: String,
    pub prompt: String,
    pub max_tokens: usize,
    pub flow_control: u8,
}

#[allow(unused)]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LLMCouncilResponse {
    pub id: String,
    pub mapping: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseObject {
    pub contents: String,
    pub process_name: String,
    pub status_code: u16,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseSummary {
    pub documents: Vec<Document>,
    pub summary_result: BTreeMap<String, usize>,
    pub response_mapping: BTreeMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub name: String,
    pub url: String,
}
