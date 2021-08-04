use anyhow::Result;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{blocking::Client, Proxy, StatusCode};
use std::thread;
use std::time::Duration;


#[derive(Clone)]
pub struct Validator {
    clients: Vec<Client>,
    offset: usize,
    current: usize,
}

impl Validator {
    pub fn new() -> Validator {
        let clients = vec![Client::new()];

        Validator { 
            clients,
            offset: 0,
            current: 0,
        }
    }
    pub fn from(proxies: Vec<String>) -> Result<Validator> {
        let mut clients = Vec::new();

        for proxy in proxies {
            let client = Client::builder().proxy(Proxy::http(proxy)?).build()?;

            clients.push(client);
        }

        Ok(Validator {
            clients,
            offset: 0,
            current: 0,
        })
    }

    pub fn set_client_offset(&mut self, o: usize) {
        self.offset = o;
    }

    pub fn next_client(&mut self) -> &Client {
        let client_idx = self.offset + self.current;
        self.current += 1;
        debug!("using client {}", client_idx);
        &self.clients[client_idx % self.clients.len()]
    }

    pub fn validate(&mut self, tok: &str) -> bool {
        const URL: &str = "https://discord.com/api/v9/users/@me/library";
        let client = self.next_client();

        // generate the headers for hte request
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(AUTHORIZATION, tok.parse().unwrap());
        // we unwrap the value here
        // that's fine, this will only fail in *rare* circumstances
        let mut res: Option<reqwest::blocking::Response> = None;
        for i in 0..10 {
            match client.get(URL).headers(headers.clone()).send() {
                Ok(resp) => {
                    res = Some(resp);
                    break;
                }
                Err(e) => {
                    warn!(
                        "error in checking token [{}]: {}\nretry [{}]",
                        tok,
                        e,
                        i + 1
                    );
                }
            };
        }

        if res.is_none() {
            warn!("failed to check token: [{}]", tok);
            return false;
        }
        let res = res.unwrap();

        // if disord gives us an OK then the token is valid
        let status = res.status();
        debug!("status code: {}", status);
        match status {
            StatusCode::OK => {
                debug!("validated token: [{}]", tok);
                true
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let wait = res
                    .headers()
                    .get("Retry-After")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();

                warn!("rate limited, waiting [{}s]", wait);
                thread::sleep(Duration::from_secs(wait));
                self.validate(tok)
            }
            _ => {
                debug!("invalidated token: [{}]", tok);
                false
            }
        }
    }
}
