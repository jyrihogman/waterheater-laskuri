use chrono::{Days, TimeZone};
use reqwest::Client;
use strum::IntoEnumIterator;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::error;
use url::form_urlencoded;

use wh_core::types::BiddingZone;

use crate::types::{EnergyChartApiResponse, WorkerError};

static BASE_URL: &str = "https://api.energy-charts.info/price";

pub async fn get_pricing_data(
    client: &Client,
    tx: mpsc::Sender<(BiddingZone, EnergyChartApiResponse)>,
) -> Result<(), WorkerError> {
    let mut handles: Vec<JoinHandle<()>> = vec![];

    for zone in BiddingZone::iter() {
        let client = client.clone();
        let sender = tx.clone();

        let handle = tokio::spawn(async move {
            match fetch_pricing(&client, &zone).await {
                Ok(data) => {
                    if sender.send((zone, data)).await.is_err() {
                        error!("Failed to send data through channel");
                    }
                }
                Err(e) => error!("Failed to fetch data for zone {:?}: {:?}", zone, e),
            }
        });
        handles.push(handle);
    }

    drop(tx);

    for handle in handles {
        if let Err(e) = handle.await {
            error!("Task failed: {:?}", e);
        }
    }

    Ok(())
}

pub async fn fetch_pricing(
    client: &Client,
    timezone: &BiddingZone,
) -> Result<EnergyChartApiResponse, reqwest::Error> {
    let start_date = timezone
        .to_tz()
        .from_utc_datetime(&chrono::Utc::now().naive_utc());

    let end_date = match start_date.checked_add_days(Days::new(1)) {
        Some(d) => d,
        None => start_date,
    };

    let url: String = format!(
        "{}?bzn={}&start={}&end={}",
        BASE_URL,
        timezone,
        form_urlencoded::byte_serialize(
            start_date
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                .as_bytes()
        )
        .collect::<String>(),
        form_urlencoded::byte_serialize(
            end_date
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                .as_bytes()
        )
        .collect::<String>()
    );

    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<EnergyChartApiResponse>()
        .await
}
