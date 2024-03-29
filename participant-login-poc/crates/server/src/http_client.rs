use std::collections::HashMap;
use std::fs::read;
use log::debug;
use rand::{Rng, thread_rng};
use reqwest::{Error, redirect, Response, StatusCode};
use reqwest::header::LOCATION;
use serde::Deserialize;
use sha256::digest;

pub struct HttpClient {
    pub client: reqwest::Client,
    pub url: String,
    uid_field_name: String,
    spoof_check_secret: String,
}



#[derive(Deserialize)]
struct TokenResponse{
    access_token: String,
    token_type: String,
    expires_in: String  //This could have been a number, but the server is sending it as a string. I guess this is what happens when you use C++
}
impl HttpClient {
    /// Creates a new HTTPS client.
    /// * `url` - The base URL to send the authentication request to. Must be PEP's authentication server's URL
    /// * `uid_field_name` - The name of the HTTP header that contains the user ID
    /// * `spoof_check_secret` - The secret to use for the Shibboleth spoof check
    /// * `root_ca_certificate_path` - The path to the root CA certificate, used to verify the authenticity of the server when using self-signed certificates
    pub fn new(url: String, uid_field_name: String, spoof_check_secret: String, root_ca_certificate_path: String) -> HttpClient {
        let buf = read(root_ca_certificate_path).expect("Error reading root CA certificate");
        reqwest::Certificate::from_pem(&buf).expect("Error parsing root CA certificate");
        let client_builder = reqwest::Client::builder()
            .connection_verbose(true) //print verbose connection info for debugging
            .redirect(redirect::Policy::none())//Do not follow redirects, so that I can get the code without contacting localhost:16515/
            .http1_title_case_headers();    //case-sensitive headers. See https://github.com/seanmonstar/reqwest/discussions/1895#discussioncomment-6355126
        let client = client_builder.build().expect("Error building HTTPS client");

        HttpClient {
            client,
            url,
            uid_field_name,
            spoof_check_secret,
        }
    }

    /// Sends an authentication request to the server. Handles PEP's authentication flow.
    /// * `uid` - The user ID to send in the HTTP header
    /// Returns PEP's OAuth token. In the future it could also return the token's type and expiration
    pub async fn send_auth_request(&self, uid: &str) -> Result<String, Box<dyn std::error::Error>> {
        let code_verifier=HttpClient::generate_code_verifier();
        debug!("Code verifier: {code_verifier}");
        let sha256_code_verifier=digest(&code_verifier);
        debug!("SHA256(code_verifier): {sha256_code_verifier}");

        /*The digest library outputs SHA256 as an HEX string, while instead PEP's function outputs bytes
        (even though the function's signature says it outputs a string, I will report this)
        So I need to convert back the result to bytes before computing the BASE64-URL-ENCODING, otherwise
        the result won't match PEP's result and the auth's server check will fail.
        */
        let sha256_code_verifier_as_bytes=hex::decode(sha256_code_verifier)?;

        let code_challenge = base64_url::encode(&sha256_code_verifier_as_bytes);
        debug!("Code challenge: {code_challenge}");

        let auth_endpoint_url_with_params = self.url.to_owned() + "/auth?&user=" + uid + "&client_id=123&redirect_uri=http://127.0.0.1:16515/&response_type=code&code_challenge=" + &code_challenge + "&code_challenge_method=S256";

        let request = self.client
            .get(auth_endpoint_url_with_params)
            .header("Shib-Spoof-Check", &self.spoof_check_secret)
            .header(self.uid_field_name.clone(), uid)
            .body("");
        debug!("PEP auth code request: {:?}", request);
        let response=request.send().await?;
        debug!("PEP auth code response: {:?}", response);

        //If the request is successful, I will get a Response containing a redirect status code and an header with the redirect URL. See https://github.com/seanmonstar/reqwest/discussions/1988#discussioncomment-7147102
        if StatusCode::is_redirection(&response.status()) {
            let redirect_url = response.headers().get(LOCATION).unwrap().to_str().unwrap();
            //Since it's a redirect, the header should be present so I'm not considering the case of a missing header. Also, it's 22:30 now. I'm also assuming it will contain a valid ASCII string, so I'm not handling the case in which it doesn't.
            debug!("Redirect URL: {:?}", redirect_url);
            let authorization_code = HttpClient::extract_code_from_redirect_url(redirect_url);
            let token_response_body = self.get_token(&authorization_code, &code_verifier, uid).await?.text().await?;
           // let token_response_text=token_response.text().await?;
           // debug!("Token response text: {token_response_text}");   //This is the body of the response to the POST request to the /token endpoint, as text.
            debug!("Token response body: {token_response_body}");
            let json: TokenResponse=serde_json::from_str(&token_response_body)?;
            Ok(json.access_token)
        } else {
            //If the status code isn't a redirect, something went wrong
            debug!("Unexpected response status code: {:?}", response.status());
            Err(Box::try_from("Unexpected response status code").unwrap())
        }
    }

    async fn get_token(&self, code: &str, code_verifier: &str, uid: &str) -> Result<Response, Error> {
        let token_endpoint = self.url.to_owned() + "/token";

        let mut request_body = HashMap::new();
        request_body.insert("client_id", "123");
           request_body.insert("redirect_uri", "http://127.0.0.1:16515/");
           request_body.insert("grant_type", "authorization_code");
           request_body.insert("code", code);
           request_body.insert("code_verifier", code_verifier);

        debug!("PEP token request body (not form-url-encoded yet: {:?}", request_body);





        //the request gets interrupted, so I'll write it in a loop to retry it
        let mut failures=0;
        loop{
            let request = self.client.post(&token_endpoint)
                .header("Shib-Spoof-Check", &self.spoof_check_secret)
                .header(self.uid_field_name.clone(), uid)
                .form(&request_body);   //The server is expecting the parameters inside a body in URL-encoded form
            debug!("PEP token request: {:?}", request);
            let response=request.send().await;
            match response {
                Ok(response) => {
                    debug!("PEP token response: {:?}", response);
                  //  let response_text=response.text().await.unwrap_or("Couldn't get response's text".to_string());
                  //  debug!("Pepe token response's text: {response_text}");
                    return Ok(response);
                }
                Err(error) => {
                    debug!("Error sending token request: {:?}", error);
                    failures+=1;
                    if failures>10{
                        debug!("Too many errors trying to POST to /token, giving up");
                        return Err(error);
                    }
                }
            }
        }
    }

    fn extract_code_from_redirect_url(redirect_url: &str) -> String {
        //This can be made more robust against possible future changes of the error by writing a regex, but finding one that works can be tricky
        debug!("Going to extract an authorization code from the following redirect url: {}", redirect_url);
        let start = redirect_url.find("code=");
        let end = redirect_url.len();
        let result = &redirect_url[start.unwrap() + 5..end];
        debug!("extracted code: {}", result);
        result.to_string()
    }

   /**
   * Generates a code verifier. Not all ASCII characters are valid ones, that's why I'm implementing
    * this function instead of using a random string generator.
    * It generates a random string of 128 characters, which is the maximum length allowed by the OAuth2 spec.
    * See <https://tools.ietf.org/html/rfc7636#section-4.1>
    */
    fn generate_code_verifier()->String{
       let valid_characters="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
       let valid_characters: Vec<char>=valid_characters.chars().collect();
       let mut rng = thread_rng();
       let mut code_verifier=String::new();
      for _ in 0..128{
          let index=rng.gen_range(0..valid_characters.len());
          code_verifier.push(valid_characters[index]);
      }
       code_verifier
    }
}

