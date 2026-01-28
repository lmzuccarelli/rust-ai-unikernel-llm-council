use crate::api::schema::ResponseObject;
use crate::config::load::ModelSchema;
use crate::handlers::helper::get_document_store_url;
use custom_logger as log;
use hyper::StatusCode;
use reqwest::Client;
use std::collections::BTreeMap;
use std::fs;

// api calls

pub async fn get_all_documents(
    mut council_members: Vec<ModelSchema>,
    title: String,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut hm_results: BTreeMap<String, String> = BTreeMap::new();
    let base_url = get_document_store_url()?;
    council_members.sort_by_key(|x| x.id);
    for ms in council_members.clone().iter() {
        let doc_url = format!(
            "{}/read?document={}-{}.md",
            base_url,
            ms.name.clone(),
            title
        );
        let response = process_get_call(doc_url).await?;
        hm_results.insert(ms.name.clone(), response);
    }
    Ok(hm_results)
}

pub async fn process_get_call(url: String) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    log::trace!("[process_get_call] {}", url);
    let client_response = client.get(url).send().await?;

    if client_response.status() != StatusCode::OK {
        return Err(Box::from(format!(
            "[process_get_call] error status code {}",
            client_response.status()
        )));
    }
    log::trace!("[process_get_call] status {}", client_response.status());
    let response = client_response.bytes().await?;
    let result = str::from_utf8(&response)?;
    Ok(result.to_string())
}

// this is a complex post as it will call the endpoint
// if successfull will then store the document
pub async fn process_post_call(
    name: String,
    url: String,
    doc_url: String,
    title: String,
    data: String,
) -> Result<ResponseObject, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let client_response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("unikernel-access", "valid")
        .body(data)
        .send()
        .await?;

    let status = client_response.status();
    let response = client_response.bytes().await?;

    let res = match status {
        StatusCode::OK => {
            // only if we have success can we then save the document
            let doc_content = String::from_utf8(response.to_vec())?;
            let doc_data = format!(
                r##"{{ "document": "{}-{}.md", "content" : {} }}"##,
                name, title, doc_content
            );
            fs::write(format!("docs/{}-{}.md", name, title), doc_content.clone())?;
            let doc_url = format!("{}/write", doc_url);
            log::info!("[process_post_call] writing document {}-{}", name, title);
            log::debug!("[process_post_call] contents {}", doc_content);

            let doc_response = client
                .post(doc_url)
                .header("Content-Type", "application/text")
                .header("unikernel-access", "valid")
                .body(doc_data)
                .send()
                .await?;

            ResponseObject {
                status_code: doc_response.status().as_u16(),
                contents: doc_response.text().await?.clone(),
                process_name: name,
            }
        }
        _ => ResponseObject {
            contents: String::from_utf8(response.to_vec())?,
            status_code: status.as_u16(),
            process_name: name,
        },
    };
    log::debug!("[process_post_call] response {:?}", res);
    Ok(res)
}
