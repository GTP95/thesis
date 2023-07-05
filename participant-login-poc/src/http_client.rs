use std::fs::read;
use rand::{Rng, rngs, SeedableRng};
use rand::rngs::StdRng;
use sha256::digest;

pub struct HttpClient {
    pub client: reqwest::Client,
    pub url: String,
    uid_field_name: String,
    spoof_check_secret: String
}

impl HttpClient {
    /// Creates a new HTTPS client
    /// * `url` - The base URL to send the authentication request to. Must be PEP's authentication server's URL
    /// * `uid_field_name` - The name of the HTTP header that contains the user ID
    /// * `spoof_check_secret` - The secret to use for the Shibboleth spoof check
    /// * `root_ca_certificate_path` - The path to the root CA certificate, used to verify the authenticity of the server when using self-signed certificates
    pub fn new(url: String, uid_field_name: String, spoof_check_secret: String, root_ca_certificate_path: String)-> HttpClient {
        let buf= read(root_ca_certificate_path).expect("Error reading root CA certificate");
        let cert = reqwest::Certificate::from_pem(&buf).expect("Error parsing root CA certificate");
        let client_builder=reqwest::Client::builder()
            .connection_verbose(true) //print verbose connection info for debugging
            .http1_title_case_headers();    //case-sensitive headers. See https://github.com/seanmonstar/reqwest/discussions/1895#discussioncomment-6355126
        let client= client_builder.build().expect("Error building HTTPS client");

        HttpClient {
            client: client,
            url: url,
            uid_field_name: uid_field_name,
            spoof_check_secret: spoof_check_secret
        }
    }

    /// Sends an authentication request to the server. Handles PEP's authentication flow.
    /// * `uid` - The user ID to send in the HTTP header
    /// * `spoof_check_secret` - The secret to use for the Shibboleth spoof check
    pub async fn send_auth_request(&self, uid: &str, spoof_check_secret: &str) -> Result<reqwest::Response, reqwest::Error> {
        // Use the ChaCha20 or ChaCha12 cipher as a random number generator to generate a random string of 32 bytes
        // This gives a security level of 128 bits against collisions, so it's in line with the rest of PEP
        // It is subject to change, see https://docs.rs/rand/0.7.0/rand/rngs/struct.StdRng.html#impl-Rng
        let mut code_verifier = [0u8; 32];
        let mut rng = StdRng::from_entropy();   //TODO: Since this is thread-safe, it should be possible to have a single instance of this for the entire application. But this isn't trivial due to ownership issues.
        rng.fill(&mut code_verifier[..]);

        let code_challenge= digest(&code_verifier);
        let auth_endpoint_url_with_params=self.url.to_owned()+"/auth?&user="+uid+"&client_id=123&redirect_uri=\"http://127.0.0.1:16515\",\"http://localhost:16515\"&response_type=code&code_challenge="+&code_challenge+"&code_challenge_method=S256";   //TODO: what's client_id again? Difference with user?
        let host_with_port=self.url.to_owned().replace("https://", "").replace("http://", "");
        let request = self.client
            .get(auth_endpoint_url_with_params)
           // .header("Host", host_with_port)   //let's be compliant to the HTTP specs
            .header("Shib-Spoof-Check", spoof_check_secret)
            .header(self.uid_field_name.clone(), uid)
            .body("")
            .send()
            .await?;
        Ok(request)
    }

}