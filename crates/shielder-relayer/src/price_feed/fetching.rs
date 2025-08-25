use rust_decimal::{Decimal, MathematicalOps as _};
use serde::Deserialize;
use shielder_relayer::PriceProvider;
use time::OffsetDateTime;

/// This is the struct that we expect to receive at `https://api.diadata.org/v1/assetQuotation/`.
#[derive(Clone, Debug, Deserialize)]
pub struct PriceInfoFromProvider {
    #[serde(rename = "Price")]
    pub token_price: Decimal,
    #[serde(
        rename = "Time",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    pub time: OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize)]
struct PythResult {
    parsed: Vec<PythItem>,
}

#[derive(Clone, Debug, Deserialize)]
struct PythItem {
    price: PythPrice,
}

#[derive(Clone, Debug, Deserialize)]
struct PythPrice {
    price: String,
    #[serde(rename = "expo")]
    exponent: i8,
    publish_time: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum PriceFetchError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Unexpected pyth response")]
    UnexpectedPythResponse,
}

impl TryFrom<PythResult> for PriceInfoFromProvider {
    type Error = PriceFetchError;

    fn try_from(value: PythResult) -> Result<Self, Self::Error> {
        if let [item] = &value.parsed[..] {
            let price = item
                .price
                .price
                .parse::<Decimal>()
                .map_err(|_| PriceFetchError::UnexpectedPythResponse)?;
            let exponent = item.price.exponent;
            let token_price = price * Decimal::new(10, 0).powd(exponent.into());
            let time = OffsetDateTime::from_unix_timestamp(item.price.publish_time as i64)
                .map_err(|_| PriceFetchError::UnexpectedPythResponse)?;

            return Ok(PriceInfoFromProvider { token_price, time });
        }

        Err(PriceFetchError::UnexpectedPythResponse)
    }
}

pub async fn fetch_price(
    provider: &PriceProvider,
) -> Result<PriceInfoFromProvider, PriceFetchError> {
    match provider {
        PriceProvider::Dia(url) => fetch_dia(url).await,
        PriceProvider::Pyth(url) => fetch_pyth(url).await,
        PriceProvider::Static(price) => Ok(PriceInfoFromProvider {
            token_price: *price,
            time: OffsetDateTime::now_utc(),
        }),
    }
}

async fn fetch_pyth(url: &str) -> Result<PriceInfoFromProvider, PriceFetchError> {
    reqwest::get(url)
        .await?
        .json::<PythResult>()
        .await?
        .try_into()
}

async fn fetch_dia(url: &str) -> Result<PriceInfoFromProvider, PriceFetchError> {
    Ok(reqwest::get(url)
        .await?
        .json::<PriceInfoFromProvider>()
        .await?)
}

#[cfg(test)]
mod tests {
    use shielder_relayer::PriceProvider;

    use super::fetch_price;

    const ETH: &str =
        "https://api.diadata.org/v1/assetQuotation/Ethereum/0x0000000000000000000000000000000000000000";
    const USDT: &str =
        "https://api.diadata.org/v1/assetQuotation/Ethereum/0xdAC17F958D2ee523a2206206994597C13D831ec7";

    const HYPER: &str =
        "https://hermes.pyth.network/v2/updates/price/latest?ids%5B%5D=0x4279e31cc369bbcc2faf022b382b080e32a8e689ff20fbc530d2a603eb6cd98b";

    #[tokio::test]
    async fn can_fetch_price_from_dia() {
        for token in &[ETH, USDT] {
            fetch_price(&PriceProvider::Dia(token.to_string()))
                .await
                .expect("Should connect to the feed and get price");
        }
    }

    #[tokio::test]
    async fn can_fetch_price_from_pyth() {
        fetch_price(&PriceProvider::Pyth(HYPER.to_string()))
            .await
            .expect("Should be able to fetch price from pyth");
    }
}
