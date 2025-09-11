use std::{collections::HashMap, fmt::Display, sync::Arc};

use alloy_primitives::Address;
use fetching::fetch_price;
use parking_lot::Mutex;
pub use price::Price;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shielder_account::Token;
use time::OffsetDateTime;
use tokio::time::Duration;
use tracing::warn;
use utoipa::{schema, ToSchema};

use crate::price::Expiration;

mod fetching;
mod price;

pub type LegacyTokenInfo = TokenInfo<TokenKind>;
pub type SimpleTokenInfo = TokenInfo<SimpleKind>;

#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
pub enum TokenKind {
    #[default]
    Native,
    ERC20 {
        #[schema(value_type = String)]
        address: Address,
        decimals: u32,
    },
}

impl Decimals for TokenKind {
    fn decimals(&self) -> u32 {
        match self {
            TokenKind::Native => 18,
            TokenKind::ERC20 { decimals, .. } => *decimals,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct SimpleKind {
    pub name: String,
    pub decimals: u32,
}

impl Decimals for SimpleKind {
    fn decimals(&self) -> u32 {
        self.decimals
    }
}

impl From<TokenKind> for Token {
    fn from(token_kind: TokenKind) -> Self {
        match token_kind {
            TokenKind::Native => Token::Native,
            TokenKind::ERC20 { address, .. } => Token::ERC20(address),
        }
    }
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            TokenKind::Native => "native".to_string(),
            TokenKind::ERC20 { address, .. } => format!("erc20:{address}"),
        };
        write!(f, "{str}")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum PriceProvider {
    Dia(String),
    Pyth(String),
    Static(Decimal),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TokenInfo<K> {
    pub kind: K,
    pub price_provider: PriceProvider,
}

pub trait Decimals {
    fn decimals(&self) -> u32;
}

impl<K: Decimals> Decimals for TokenInfo<K> {
    fn decimals(&self) -> u32 {
        self.kind.decimals()
    }
}

/// A collection of prices for various coins.
///
/// The underlying structure is behind a mutex, and a process to update it
/// asynchronously can be started with `start_price_feed`.
///
/// The type parameter `K` is the type used to identify tokens.
/// Apart from being `Clone`, `Eq`, `PartialEq` and `Hash`, it must also implement the `Decimals` trait
/// to provide the number of decimals for each token. `TokenKind` is a legacy implementation that works
/// but can only represent one native token. `SimpleKind` is an implementation where a token is identified
/// by both its name and its number of decimals.
#[derive(Clone)]
pub struct Prices<K> {
    validity: time::Duration,
    refresh_interval: Duration,
    tokens: HashMap<K, TokenInfo<K>>,
    inner: HashMap<K, Arc<Mutex<Option<Price>>>>,
}

impl<K: Clone + Eq + PartialEq + std::hash::Hash + Decimals> Prices<K> {
    /// Create a new `Prices` instance for a set of tokens with the given validity and refresh
    /// interval.
    ///
    /// Note that you should realistically set `validity` to at least 5 or 10 minutes - it seems
    /// the API we are using (DIA) updates about 2 or 3 minutes or so.
    pub fn new(tokens: &[TokenInfo<K>], validity: Duration, refresh_interval: Duration) -> Self {
        let validity =
            time::Duration::new(validity.as_secs() as i64, validity.subsec_nanos() as i32);

        let mut token_map = HashMap::new();
        let mut inner = HashMap::new();

        for token in tokens {
            token_map.insert(token.kind.clone(), token.clone());
            let price = match &token.price_provider {
                PriceProvider::Dia(_) => None,
                PriceProvider::Pyth(_) => None,
                PriceProvider::Static(price) => Some(Price::static_price(*price, token.decimals())),
            };
            inner.insert(token.kind.clone(), Arc::new(Mutex::new(price)));
        }

        Self {
            validity,
            refresh_interval,
            tokens: token_map,
            inner,
        }
    }

    /// Gather current price for all the tokens.
    pub fn current_prices(&self) -> HashMap<K, Option<Price>> {
        self.tokens
            .keys()
            .map(|k| (k.clone(), self.price(k)))
            .collect()
    }

    pub fn price_ages(&self) -> HashMap<K, Option<time::Duration>> {
        let now = OffsetDateTime::now_utc();
        self.inner
            .iter()
            .map(|(token, price)| {
                let price = price.lock();
                if price.is_none() {
                    // if the price is None, it means it was never fetched
                    return (token.clone(), None);
                }
                let price = price.as_ref().unwrap();
                match price.expiration {
                    Expiration::Eternal => (token.clone(), Some(time::Duration::ZERO)),
                    Expiration::Timed { fetched, .. } => (token.clone(), Some(now - fetched)),
                }
            })
            .collect()
    }

    /// Get the price of a token or `None` if the price is not available or outdated.
    pub fn price(&self, token: &K) -> Option<Price> {
        self.inner
            .get(token)?
            .lock()
            .clone()?
            .validate(&OffsetDateTime::now_utc())
    }

    async fn update(&self) {
        for token in self.tokens.values() {
            let price_info = fetch_price(&token.price_provider).await;

            if let Err(err) = price_info {
                warn!("Failed to update prices: {err}");
                continue;
            }

            let price =
                Price::from_price_info(price_info.unwrap(), token.decimals(), self.validity);

            self.inner.get(&token.kind).unwrap().lock().replace(price);
        }
    }
}

/// Start a price feed that updates the prices in the given `Prices` instance.
pub async fn start_price_feed<K: Clone + Eq + PartialEq + std::hash::Hash + Decimals>(
    prices: Prices<K>,
) -> Result<(), anyhow::Error> {
    loop {
        prices.update().await;
        tokio::time::sleep(prices.refresh_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_with_static_price() -> TokenInfo<TokenKind> {
        TokenInfo {
            kind: TokenKind::Native,
            price_provider: PriceProvider::Static(Decimal::ONE),
        }
    }

    fn token_with_url_price() -> TokenInfo<TokenKind> {
        TokenInfo {
            kind: TokenKind::Native,
            price_provider: PriceProvider::Dia(
                "https://api.diadata.org/v1/assetQuotation/Ethereum/0x0000000000000000000000000000000000000000".to_string(),
            ),
        }
    }

    #[tokio::test]
    async fn price_available_without_update_when_using_static_provider() {
        let prices = Prices::new(
            &[token_with_static_price()],
            Duration::from_secs(1_000_000),
            Default::default(),
        );
        assert!(prices.price(&TokenKind::Native).is_some());
    }

    #[tokio::test]
    async fn single_update_static_provider() {
        let prices = Prices::new(
            &[token_with_static_price()],
            Duration::from_secs(1_000_000),
            Default::default(),
        );

        prices.update().await;

        assert!(prices.price(&TokenKind::Native).is_some());
    }

    #[tokio::test]
    async fn single_update_url_provider() {
        let prices = Prices::new(
            &[token_with_url_price()],
            Duration::from_secs(1_000_000),
            Default::default(),
        );

        prices.update().await;

        assert!(prices.price(&TokenKind::Native).is_some());
    }

    #[tokio::test]
    async fn with_short_validity_even_after_update_there_is_no_price_available() {
        let prices = Prices::new(
            &[token_with_url_price()],
            Duration::from_millis(1),
            Default::default(),
        );
        prices.update().await;

        assert!(prices.price(&TokenKind::Native).is_none());
    }

    #[tokio::test]
    async fn start_price_feed_works() {
        let prices = Prices::new(
            &[token_with_url_price()],
            Duration::from_secs(1_000_000),
            Duration::from_secs(1),
        );
        tokio::spawn(start_price_feed(prices.clone()));

        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(prices.price(&TokenKind::Native).is_some());
    }
}
