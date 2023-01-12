use bytes::Bytes;
use http::{
    header::{HeaderName, HeaderValue},
    StatusCode,
};
use std::net::{IpAddr, SocketAddr};
use std::{convert::TryFrom, time::Duration};

#[derive(Debug)]
pub struct TestClient {
    client: reqwest::Client,
    addr: SocketAddr,
}

impl TestClient {
    pub async fn new(svc: axum::Router) -> Self {
        let free_port = port_check::free_local_port().unwrap();
        let ip = "0.0.0.0".parse::<IpAddr>().unwrap();
        let socket = SocketAddr::new(ip, free_port);

        let svc = svc.clone();

        tokio::spawn(async move {
            axum::Server::bind(&socket)
                .serve(svc.into_make_service())
                .await
        });

        println!("TestClient Router listening on {}", socket);

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let test_client = TestClient {
            client,
            addr: socket,
        };

        // It takes time for `Axum` to spin up our `Router`. Let's wait until
        // we can at least get some form of response from the server.
        test_client.wait_until_server_started().await;

        test_client
    }

    #[allow(dead_code)]
    pub fn get(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.get(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn head(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.head(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn post(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.post(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn put(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.put(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn patch(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.patch(format!("http://{}{}", self.addr, url)),
        }
    }

    #[allow(dead_code)]
    pub fn delete(&self, url: &str) -> RequestBuilder {
        RequestBuilder {
            builder: self.client.delete(format!("http://{}{}", self.addr, url)),
        }
    }

    async fn wait_until_server_started(&self) {
        loop {
            let res = self.get("/auth/signin").send().await;
            if res.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

#[allow(dead_code)]
pub struct RequestBuilder {
    builder: reqwest::RequestBuilder,
}

impl RequestBuilder {
    #[allow(dead_code)]
    pub async fn send(self) -> Result<TestResponse, reqwest::Error> {
        Ok(TestResponse {
            response: self.builder.send().await?,
        })
    }

    #[allow(dead_code)]
    pub fn body(mut self, body: impl Into<reqwest::Body>) -> Self {
        self.builder = self.builder.body(body);
        self
    }

    #[allow(dead_code)]
    pub fn json<T>(mut self, json: &T) -> Self
    where
        T: serde::Serialize,
    {
        self.builder = self.builder.json(json);
        self
    }

    #[allow(dead_code)]
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.builder = self.builder.header(key, value);
        self
    }

    #[allow(dead_code)]
    pub fn multipart(mut self, form: reqwest::multipart::Form) -> Self {
        self.builder = self.builder.multipart(form);
        self
    }
}

#[allow(dead_code)]
pub struct TestResponse {
    response: reqwest::Response,
}

impl TestResponse {
    #[allow(dead_code)]
    pub async fn text(self) -> Result<String, reqwest::Error> {
        self.response.text().await
    }

    #[allow(dead_code)]
    pub async fn bytes(self) -> Result<Bytes, reqwest::Error> {
        self.response.bytes().await
    }

    #[allow(dead_code)]
    pub async fn json<T>(self) -> Result<T, reqwest::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.response.json().await
    }

    #[allow(dead_code)]
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    #[allow(dead_code)]
    pub fn headers(&self) -> &http::HeaderMap {
        self.response.headers()
    }

    #[allow(dead_code)]
    pub async fn chunk(&mut self) -> Result<Option<Bytes>, reqwest::Error> {
        self.response.chunk().await
    }

    #[allow(dead_code)]
    pub async fn chunk_text(&mut self) -> Result<Option<String>, reqwest::Error> {
        let chunk = self.chunk().await?;
        if let Some(chunk) = chunk {
            let chunk = String::from_utf8(chunk.to_vec());
            match chunk {
                Ok(chunk) => return Ok(Some(chunk)),
                Err(_) => return Ok(Option::None),
            }
        } else {
            Ok(Option::None)
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::routing::get;
    use axum::Router;
    use http::StatusCode;

    #[tokio::test]
    async fn test_get_request() {
        let app = Router::new().route("/", get(|| async {}));
        let client = super::TestClient::new(app).await;
        let res = client.get("/").send().await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
