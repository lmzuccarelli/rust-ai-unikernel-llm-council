use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LLMCouncilRequestSchema {
    pub title: String,
    pub prompt: String,
    pub max_tokens: usize,
    pub cache: Option<bool>,
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
