use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt};
use std::{
    collections::HashMap,
    result::Result,
    sync::{Arc, Mutex},
};

type City = String;
type Temperature = u64;

#[async_trait]
pub trait Api: Send + Sync + 'static {
    async fn fetch(&self) -> Result<HashMap<City, Temperature>, String>;
    async fn subscribe(&self) -> BoxStream<Result<(City, Temperature), String>>;
}

pub struct StreamCache {
    results: Arc<Mutex<HashMap<String, u64>>>,
}

impl StreamCache {
    pub fn new(api: impl Api) -> Self {
        let instance = Self {
            results: Arc::new(Mutex::new(HashMap::new())),
        };
        instance.update_in_background(api);
        instance
    }

    pub fn get(&self, key: &str) -> Option<u64> {
        let results = self.results.lock().expect("poisoned");
        results.get(key).copied()
    }

    pub fn update_in_background(&self, api: impl Api) {
        let result = self.results.clone();
        let api = Arc::new(api);
        let api_sub = api.clone();
        let api_fetch = api.clone();

        tokio::spawn(async move {
            let mut stream = api_sub.subscribe().await;

            while let Some(update) = stream.next().await {
                if let Ok((city, temp)) = update {
                    let mut cache = result.lock().expect("poisoned");

                    cache
                        .entry(city)
                        .and_modify(|old_temp| *old_temp = temp)
                        .or_insert(temp);
                }
            }
        });

        let result = self.results.clone();
        tokio::spawn(async move {
            match api_fetch.fetch().await {
                Ok(initial_data) => {
                    let mut cache = result.lock().expect("poisoned");
                    for (city, temp) in initial_data {
                        cache.entry(city).or_insert(temp);
                    }
                }
                Err(err) => {
                    println!("Error fetch(): {}", err);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use tokio::sync::Notify;
    use tokio::time;

    use futures::{future, stream::select, FutureExt, StreamExt};
    use maplit::hashmap;

    use super::*;

    #[derive(Default)]
    struct TestApi {
        signal: Arc<Notify>,
    }

    #[async_trait]
    impl Api for TestApi {
        async fn fetch(&self) -> Result<HashMap<City, Temperature>, String> {
            // fetch is slow an may get delayed until after we receive the first updates
            self.signal.notified().await;

            Ok(hashmap! {
                "Rabat".to_string() => 29,
                "Seoul".to_string() => 31,
            })
        }
        async fn subscribe(&self) -> BoxStream<Result<(City, Temperature), String>> {
            let results = vec![
                Ok(("London".to_string(), 27)),
                Ok(("Seoul".to_string(), 32)),
            ];

            select(
                futures::stream::iter(results),
                async {
                    self.signal.notify_one();
                    future::pending().await
                }
                .into_stream(),
            )
            .boxed()
        }
    }
    #[tokio::test]
    async fn works() {
        let cache = StreamCache::new(TestApi::default());

        time::sleep(Duration::from_millis(1000)).await;

        assert_eq!(cache.get("Rabat"), Some(29));
        assert_eq!(cache.get("London"), Some(27));
        assert_eq!(cache.get("Seoul"), Some(32));
    }
}
