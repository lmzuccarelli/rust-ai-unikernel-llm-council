use crate::api::schema::LLMCouncilRequestSchema;
use crate::config::load::ModelSchema;
use crate::handlers::api_calls::*;
use crate::handlers::helper::*;
use custom_logger as log;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use hyper::body::Bytes;
use std::collections::BTreeMap;

pub async fn flow_control(
    end_point: String,
    data: Bytes,
) -> Result<(), Box<dyn std::error::Error>> {
    let req: LLMCouncilRequestSchema = serde_json::from_slice(&data)?;
    let cm = get_council_members()?;
    if check_semaphore()? {
        return Err(Box::from("[flow_control] still processing"));
    } else {
        let cache = req.cache.unwrap_or(false);

        // flow is as follows
        //
        // 1. collect initial response from user prompt to all council members
        // 2. create a ranking prompt from the responses obtained in step 1
        // 3. collect the ranking results from all council members using the ranking prompt
        // 4. create a summary prompt for the council chairman
        // 5. collect the council chariman's summary and final rating

        set_semaphore(true)?;

        // start flow
        log::info!("[flow_control] triggered flow_control");

        // 1.
        if !cache {
            log::info!("[flow_control] executing collect initial responses");
            collect_initial_responses(
                end_point.clone(),
                cm.clone(),
                req.prompt.clone(),
                req.title.clone(),
                req.max_tokens,
            )
            .await?;
            log::info!("[flow_control] completed collect initial responses");
        }

        // 2.
        let hm_ir = get_all_documents(cm.clone(), format!("initial-{}", req.title)).await?;
        let (initial_merged_responses, label_mapping) = format_initial_responses(hm_ir);

        // 3.
        if !cache {
            log::info!("[flow_control] executing collect ranking responses");
            collect_ranking_responses(
                end_point.clone(),
                cm.clone(),
                req.prompt.clone(),
                req.title.clone(),
                initial_merged_responses.clone(),
            )
            .await?;
            log::info!("[flow_control] completed collect ranking responses");
        }

        // 4.
        let hm_ranking = get_all_documents(cm.clone(), format!("ranking-{}", req.title)).await?;
        let ranking_merged_responses = format_ranking_responses(hm_ranking.clone());

        // 5.
        if !cache {
            log::info!("[flow_control] executing chairman council analysis");
            chairman_council_analysis(
                end_point,
                req.prompt,
                req.title.clone(),
                initial_merged_responses,
                ranking_merged_responses,
            )
            .await?;
            log::info!("[flow_control] completed chairman council analysis");
        }

        log::info!("[flow_control] label mapping {:?}", label_mapping);

        // all good set semaphore to false
        set_semaphore(false)?;
    }
    Ok(())
}

pub async fn all_health() -> Result<String, Box<dyn std::error::Error>> {
    let mut result: String = String::new();
    let council_members = get_council_members()?;
    let cm = council_members.clone();
    for ms in cm.iter() {
        let name = ms.name.clone();
        let base_url = ms.url.clone();
        let handle = tokio::spawn(async move {
            let response = process_get_call(format!("{}/v1/health", base_url)).await;
            match response {
                Ok(content) => {
                    log::debug!("[all_health] mapping member {} {}", name, content);
                    content
                }
                Err(e) => {
                    log::error!("[all_health] response {} {}", name, e);
                    e.to_string()
                }
            }
        });
        match handle.await {
            Ok(res) => {
                result.push_str(&format!("{}: {}", ms.name, res));
            }
            Err(e) => {
                log::error!("[all_health] spawn {}", e);
            }
        }
    }
    Ok(result)
}

async fn collect_initial_responses(
    end_point: String,
    council_members: Vec<ModelSchema>,
    prompt: String,
    title: String,
    max_tokens: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let cm = council_members.clone();
    let doc_url = get_document_store_url()?;
    let mut futs = FuturesUnordered::new();
    // call all services in parallel
    for ms in cm.iter() {
        let name = ms.name.clone();
        let url = ms.url.clone();
        let message = format!(
            r##"{{ "model": "{}", "messages": [{{"role": "user", "content": "{}" }}], "max_tokens": {} }}"##,
            ms.model, prompt, max_tokens
        );
        let updated_url = format!("{}{}", url, end_point);
        let updated_title = format!("initial-{}", title);
        futs.push(process_post_call(
            name,
            updated_url,
            doc_url.clone(),
            updated_title,
            message.clone(),
        ));
    }
    // wait for all posts to complete
    while let Some(response) = futs.next().await {
        match response {
            Ok(contents) => {
                log::info!(
                    "[collect_initial_responses] {}  {}",
                    contents.process_name,
                    contents.status_code
                )
            }
            Err(e) => {
                return Err(Box::from(format!("[collect_initial_responses] {}", e)));
            }
        }
    }
    Ok(())
}

async fn collect_ranking_responses(
    end_point: String,
    council_members: Vec<ModelSchema>,
    prompt: String,
    title: String,
    initial_responses_merged: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut futs = FuturesUnordered::new();
    let mut stage_prompt = format!(
        r##"
        You are evaluating different responses to the following question:

        Question {:?} 
            
        Here are the responses from different models (anonymized):
        
        {:?} 
    "##,
        prompt, initial_responses_merged
    );

    stage_prompt.push_str(
        r#"
        
        Your task:
        1. First, evaluate each response individually. For each response, explain what it does well and what it does poorly.
        2. Then, at the very end of your response, provide a final ranking.

        IMPORTANT: Your final ranking MUST be formatted EXACTLY as follows:
        - Start with the line "FINAL RANKING:" (all caps, with colon)
        - Then list the responses from best to worst as a numbered list
        - Each line should be: number, period, space, then ONLY the response label (e.g., "1. Response A")
        - Do not add any other text or explanations in the ranking section

        Example of the correct format for your ENTIRE response:

        Response A provides good detail on X but misses Y...
        Response B is accurate but lacks depth on Z...
        Response C offers the most comprehensive answer...

        FINAL RANKING:
        1. Response C
        2. Response A
        3. Response B

        Now provide your evaluation and ranking:
        "#,
        );

    let doc_url = get_document_store_url()?;
    // call all services in parallel
    for ms in council_members.clone().iter() {
        let updated_url = format!("{}{}", ms.url, end_point.clone());
        let updated_title = format!("ranking-{}", title);
        let message = format!(
            r##"{{ "model": "{}", "messages": [{{"role": "user", "content": {:?} }}], "max_tokens": {} }}"##,
            ms.model, stage_prompt, 16384
        );
        futs.push(process_post_call(
            ms.name.clone(),
            updated_url,
            doc_url.clone(),
            updated_title.clone(),
            message.clone(),
        ));
    }
    // wait for all posts to complete
    while let Some(response) = futs.next().await {
        match response {
            Ok(contents) => {
                log::info!(
                    "[collect_ranking_responses] {}  {}",
                    contents.process_name,
                    contents.status_code
                )
            }
            Err(e) => {
                return Err(Box::from(format!("[collect_ranking_responses] {}", e)));
            }
        }
    }
    Ok(())
}

async fn chairman_council_analysis(
    end_point: String,
    prompt: String,
    title: String,
    initial_responses_merged: String,
    ranking_responses_merged: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let stage_prompt = format!(
        r##"
        You are the Chairman of an LLM Council. Multiple AI models have provided responses to a user's question, and then ranked each other's responses.

        Original Question {:?} 

        STAGE 1 - Individual Responses:
        {:?}

        STAGE 2 - Peer Rankings:
        {:?}

        Your task as Chairman is to synthesize all of this information into a single, comprehensive, accurate answer to the user's original question. Consider:
        - The individual responses and their insights
        - The peer rankings and what they reveal about response quality
        - Any patterns of agreement or disagreement

        Provide a clear, well-reasoned final answer that represents the council's collective wisdom:

        "##,
        prompt, initial_responses_merged, ranking_responses_merged
    );

    let chairman = get_council_chairman()?;
    let chairman_url = chairman.url;
    let updated_url = format!("{}{}", chairman_url, end_point);
    let doc_url = get_document_store_url()?;
    let updated_title = format!("chairman-summary-{}", title);
    let message = format!(
        r##"{{ "model": "{}", "messages": [{{"role": "user", "content": {:?} }}] }}"##,
        chairman.model, stage_prompt
    );
    let response = process_post_call(
        chairman.name.clone(),
        updated_url,
        doc_url,
        updated_title,
        message,
    )
    .await?;
    match response.status_code {
        200 => Ok(()),
        _ => Err(Box::from(format!(
            "[council_analysis] {} : {}",
            chairman.name, response.contents
        ))),
    }
}

fn format_initial_responses(
    initial_responses: BTreeMap<String, String>,
) -> (String, BTreeMap<String, String>) {
    let mut stage_prompt = String::new();
    let mut label_model: BTreeMap<String, String> = BTreeMap::new();
    for (count, (k, v)) in initial_responses.clone().iter().enumerate() {
        let label = format!("Response {}", (65 + count as u8) as char);
        let model_response = format!(r#"{}:{}"#, label, v);
        label_model.insert(k.to_owned(), label);
        stage_prompt.push_str(&model_response);
    }
    (stage_prompt, label_model)
}

fn format_ranking_responses(ranking_responses: BTreeMap<String, String>) -> String {
    let mut stage_prompt = String::new();
    for (count, (_k, v)) in ranking_responses.clone().iter().enumerate() {
        let label = format!("Response {}", (65 + count as u8) as char);
        let model_response = format!(r#"{}:{}"#, label, v);
        stage_prompt.push_str(&model_response);
    }
    stage_prompt
}

// TDD - initial phase

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::collections::BTreeMap;

    #[test]
    fn test_rank_parsing_kgo() {
        let result = r#"
        Response A provides good detail on X but misses Y...
        Response B is accurate but lacks depth on Z...
        Response C offers the most comprehensive answer...

        FINAL RANKING:
        1. Response C
        2. Response A
        3. Response B

        "#;

        let vec_body: Vec<&str> = result.split("FINAL RANKING:").collect();
        assert_eq!(vec_body.len(), 2);

        let re = Regex::new(r"\d+\.\s*Response [A-Z]").unwrap();
        let all = re.captures_iter(result);
        println!();
        for s in all {
            println!("{}", s.get_match().as_str());
        }
    }

    #[test]
    fn test_rank_parsing_missing_final() {
        let result = r#"
        Response A provides good detail on X but misses Y...
        Response B is accurate but lacks depth on Z...
        Response C offers the most comprehensive answer...

        1. Response C
        2. Response A
        3. Response B

        "#;

        let vec_body: Vec<&str> = result.split("FINAL RANKING:").collect();
        assert_eq!(vec_body.len(), 1);

        let re = Regex::new(r"Response [A-Z]").unwrap();
        let all = re.captures_iter(result);
        println!();
        for s in all {
            println!("{}", s.get_match().as_str());
        }
    }

    #[test]
    fn calculate_aggregate_rankings() {
        println!();
        let mut bt_result: BTreeMap<usize, &str> = BTreeMap::new();
        let mut vec_all: Vec<Vec<&str>> = Vec::new();
        let vec_lookup = ["Response A", "Response B", "Response C", "Response D"];
        vec_all.push(vec!["Response C", "Response A", "Response B", "Response D"]);
        vec_all.push(vec!["Response C", "Response B", "Response D", "Response A"]);
        vec_all.push(vec!["Response D", "Response A", "Response B", "Response C"]);
        vec_all.push(vec!["Response C", "Response A", "Response B", "Response D"]);
        for k in vec_lookup.iter() {
            let mut total = 1;
            for item in vec_all.iter() {
                total = total + item.iter().position(|x| x == k).map_or(100, |x| x) + 1;
            }
            bt_result.insert(total, k);
        }
        println!("result {:?}", bt_result);
    }
}
