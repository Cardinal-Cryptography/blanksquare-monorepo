use super::{PriceFetchError, PriceInfoFromProvider};

pub(crate) async fn fetch_dia(url: &str) -> Result<PriceInfoFromProvider, PriceFetchError> {
    Ok(reqwest::get(url)
        .await?
        .json::<PriceInfoFromProvider>()
        .await?)
}

#[cfg(test)]
mod tests {
    use super::fetch_dia;

    const ETH: &str =
        "https://api.diadata.org/v1/assetQuotation/Ethereum/0x0000000000000000000000000000000000000000";
    const USDT: &str =
        "https://api.diadata.org/v1/assetQuotation/Ethereum/0xdAC17F958D2ee523a2206206994597C13D831ec7";

    #[tokio::test]
    async fn can_fetch_price_from_dia() {
        for url in &[ETH, USDT] {
            fetch_dia(url)
                .await
                .expect("Should connect to the feed and get price");
        }
    }
}
