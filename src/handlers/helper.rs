use crate::MAP_LOOKUP;
use crate::SEMAPHORE;
use crate::config::load::ModelSchema;

// helper functions

pub fn check_semaphore() -> Result<bool, Box<dyn std::error::Error>> {
    let state = SEMAPHORE.lock()?;
    Ok(*state)
}

pub fn set_semaphore(value: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = SEMAPHORE.lock()?;
    *state = value;
    Ok(())
}

pub fn get_council_chairman() -> Result<ModelSchema, Box<dyn std::error::Error>> {
    let hm_guard = MAP_LOOKUP.lock().map_err(|_| "mutex lock failed")?;
    let res_guard = hm_guard.as_ref();
    let result = match res_guard {
        Some(value) => value.council_chairman.clone(),
        None => {
            return Err(Box::from(
                "[get_council_chairman] retrieving council_chairman parameter",
            ));
        }
    };
    Ok(result)
}

pub fn get_council_members() -> Result<Vec<ModelSchema>, Box<dyn std::error::Error>> {
    let hm_guard = MAP_LOOKUP.lock().map_err(|_| "mutex lock failed")?;
    let res_guard = hm_guard.as_ref();
    let result = match res_guard {
        Some(value) => value.council_members.clone(),
        None => {
            return Err(Box::from(
                "[get_council_members] retrieving council_members parameter",
            ));
        }
    };
    Ok(result)
}

pub fn get_document_store_url() -> Result<String, Box<dyn std::error::Error>> {
    let hm_guard = MAP_LOOKUP.lock().map_err(|_| "mutex lock failed")?;
    let res_guard = hm_guard.as_ref();
    let result = match res_guard {
        Some(value) => value.document_service_url.clone(),
        None => {
            return Err(Box::from(
                "[get_document_store_url] retrieving document_service_url parameter",
            ));
        }
    };
    Ok(result)
}
