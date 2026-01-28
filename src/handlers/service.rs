use crate::handlers::controller::{all_health, flow_control};
use crate::handlers::helper::set_semaphore;
use custom_logger as log;
use http::{Method, Request, Response, StatusCode};
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};

pub async fn endpoints(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut response = Response::new(Full::default());
    let request = req.uri().path();
    log::debug!("{}", request);
    match *req.method() {
        Method::POST => match request {
            x if x.contains("/v1/chat/completions") => {
                let data = req.into_body().collect().await?.to_bytes();
                let result = flow_control("/v1/chat/completions".to_owned(), data).await;
                match result {
                    Ok(_) => {
                        *response.body_mut() = Full::from("[endpoints] flow_control completed\n");
                    }
                    Err(err) => {
                        if err.to_string().contains("still processing") {
                            *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                            *response.body_mut() = Full::from(
                                "[endpoints] still processing - try again later\n".to_string(),
                            );
                        } else {
                            let _ = set_semaphore(false);
                            log::error!("[endpoints] {}", err);
                            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                            *response.body_mut() =
                                Full::from(format!("[endpoints] error : {}\n", err));
                        }
                    }
                }
            }
            _ => {
                log::error!("[endpoints] method/endpoint not implemented");
                *response.body_mut() = Full::from("[endpoints] method/endpoint not implmented\n");
                *response.status_mut() = StatusCode::NOT_FOUND;
            }
        },
        Method::GET => match request {
            x if x.contains("/v1/health") => {
                let res = all_health().await;
                match res {
                    Ok(value) => {
                        let content = format!("[endpoints] all_health\n{}", value);
                        *response.body_mut() = Full::from(content);
                    }
                    Err(err) => {
                        log::error!("[endpoints] {}", err);
                        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        *response.body_mut() = Full::from(format!("error : {:?}\n", err.source()));
                    }
                }
            }
            x if x.contains("/v1/is-alive") => {
                let content = format!(
                    r##"{{ "status":"ok", "appplication": "{}", "version": "{}" }}"##,
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                );
                *response.body_mut() = Full::from(content);
            }
            &_ => {}
        },
        _ => {
            log::error!("[endpoints] method/endpoint not implemented");
            *response.body_mut() = Full::from("[endpoints] method/endpoint not implmented\n");
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };
    Ok(response)
}
