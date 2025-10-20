use rust_decimal::{Decimal, MathematicalOps as _};
use serde::Deserialize;
use time::OffsetDateTime;

use super::{PriceFetchError, PriceInfoFromProvider};

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

pub(crate) async fn fetch_pyth(url: &str) -> Result<PriceInfoFromProvider, PriceFetchError> {
    reqwest::get(url)
        .await?
        .json::<PythResult>()
        .await?
        .try_into()
}

#[cfg(test)]
mod tests {
    use super::fetch_pyth;

    const HYPER: &str =
        "https://hermes.pyth.network/v2/updates/price/latest?ids%5B%5D=0x4279e31cc369bbcc2faf022b382b080e32a8e689ff20fbc530d2a603eb6cd98b";

    #[tokio::test]
    async fn can_fetch_price_from_pyth() {
        fetch_pyth(HYPER)
            .await
            .expect("Should be able to fetch price from pyth");
    }
}
