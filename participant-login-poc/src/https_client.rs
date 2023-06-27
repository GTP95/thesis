use std::fs::read;

pub struct HttpsClient {
    pub client: reqwest::Client,
    pub url: String,
    uid_field_name: String,
    spoof_check_secret: String
}

impl HttpsClient {
    pub fn new(url: String, uid_field_name: String, spoof_check_secret: String, root_ca_certificate_path: String)-> HttpsClient {
        let buf= read(root_ca_certificate_path).expect("Error reading root CA certificate");
        let cert = reqwest::Certificate::from_pem(&buf).expect("Error parsing root CA certificate");
        let client_builder=reqwest::Client::builder()
            .add_root_certificate(cert)
            .https_only(true)
            //Currently, PEP only supports TLSv1.2, forcing that version
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .max_tls_version(reqwest::tls::Version::TLS_1_2);
        let client= client_builder.build().expect("Error building HTTPS client");

        HttpsClient {
            client: client,
            url: url,
            uid_field_name: uid_field_name,
            spoof_check_secret: spoof_check_secret
        }
    }

    pub async fn send_auth_request(&self, uid: &str, spoof_check_secret: &str) -> Result<reqwest::Response, reqwest::Error> {
        let request = self.client
            .post(&self.url)
            .header("Shib-Spoof-Check", spoof_check_secret)
            .header(self.uid_field_name.clone(), uid)
            .body("")
            .send()
            .await?;
        Ok(request)
    }

}