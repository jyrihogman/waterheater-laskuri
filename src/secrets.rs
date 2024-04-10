use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::error::ApplicationError;

lazy_static! {
    static ref API_KEYS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

async fn update_api_keys_cache(api_keys: Vec<String>) {
    let mut api_keys_cache = API_KEYS.lock().unwrap();
    *api_keys_cache = api_keys;
}

async fn cache_api_keys() -> Result<(), ApplicationError> {
    match fetch_api_keys().await {
        Ok(api_keys) => {
            update_api_keys_cache(api_keys).await;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

async fn fetch_api_keys() -> Result<Vec<String>, ApplicationError> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_ssm::Client::new(&config);

    let mut next_token: Option<String> = None;
    let mut api_keys: Vec<String> = Vec::new();

    loop {
        let resp = client
            .get_parameters_by_path()
            .path("/waterheater_calc/api_keys/")
            .recursive(true)
            .with_decryption(true)
            .set_next_token(next_token)
            .send()
            .await?;

        resp.parameters()
            .iter()
            .for_each(|param| api_keys.push(param.value().unwrap().to_string()));

        if resp.next_token().is_none() {
            break;
        }

        next_token = resp.next_token().map(|t| t.to_string());
    }

    Ok(api_keys)
}
