pub struct HttpsClient {
    pub client: reqwest::Client,
    pub url: String,
    uid_field_name: String,
    spoof_check_secret: String
}

impl HttpsClient {
    pub fn new(url: String, uid_field_name: String, spoof_check_secret: String)-> HttpsClient {
        HttpsClient {
            client: reqwest::Client::new(),
            url: url,
            uid_field_name: uid_field_name,
            spoof_check_secret: spoof_check_secret
        }
    }

    pub async fn send_auth_request(&self, uid: &str, spoof_check_secret: &str) -> Result<reqwest::Response, reqwest::Error> {
        let request = reqwest::Client::new()
            .post(&self.url)
            .header("Shib-Spoof-Check", spoof_check_secret)
            .header(self.uid_field_name.clone(), uid)
            .body("Just some data to complete message")
            .send()
            .await?;
        Ok(request)
    }

}