use reqwest;

pub struct HTTPclient {
    pub client: reqwest::Client,
    pub url: String,
    uid_field_name: String,
    spoof_check_secret: String
}

impl HTTPclient {
    pub fn new(url: String, uid_field_name: String, spoof_check_secret: String)-> HTTPclient{
        HTTPclient{
            client: reqwest::Client::new(),
            url,
            uid_field_name,
            spoof_check_secret
        }
    }

    pub async fn send_auth_request(&self, uid: &String, spoof_check_secret: &String) -> Result<reqwest::Response, reqwest::Error> {
        let request = reqwest::Client::new()
            .post(self.url.as_str())
            .header("Shib-Spoof-Check", spoof_check_secret)
            .header(self.uid_field_name.clone(), uid)
            .body("")
            .send()
            .await?;
        Ok(request)
    }

}