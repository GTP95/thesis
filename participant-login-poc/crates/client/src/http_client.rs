use std::error::Error;
use std::fmt;
use std::fs::read;
use reqwest::redirect;
use irma::SessionStatus;
use serde::{Deserialize, Serialize};
use serde_json::Result as SerdeResult;



pub struct HttpClient {
    pub client: reqwest::Client,
    pub url: String,
    uid_field_name: String,
    spoof_check_secret: String
}

pub struct AuthResponse {
    pub code_verifier: [u8; 32],
    pub response: reqwest::Response
}

/** Stores the QR code and the session pointer of an IRMA session */
#[derive(Serialize, Deserialize)]
pub struct QRCodeAndSessionPtr {
    pub qr_code: String,
    pub session_ptr: String
}

/** My own type to handle errors getting IRMA session status */
#[derive(Debug, Clone)]
struct GetStatusError {
   pub message: String,
}

impl fmt::Display for GetStatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GetStatusError: {}", self.message)
    }
}

impl HttpClient {
    /// Creates a new HTTPS bin
    /// * `url` - The base URL to send the authentication request to. Must be PEP's authentication server's URL
    /// * `uid_field_name` - The name of the HTTP header that contains the user ID
    /// * `spoof_check_secret` - The secret to use for the Shibboleth spoof check
    /// * `root_ca_certificate_path` - The path to the root CA certificate, used to verify the authenticity of the server when using self-signed certificates
    pub fn new(url: String, uid_field_name: String, spoof_check_secret: String, root_ca_certificate_path: String)-> HttpClient {
        let buf= read(root_ca_certificate_path).expect("Error reading root CA certificate");
        reqwest::Certificate::from_pem(&buf).expect("Error parsing root CA certificate");
        let client_builder=reqwest::Client::builder()
            .connection_verbose(true) //print verbose connection info for debugging
            .redirect(redirect::Policy::none())//Do not follow redirects, so that I can get the code without contacting localhost:16515/
            .http1_title_case_headers();    //case-sensitive headers. See https://github.com/seanmonstar/reqwest/discussions/1895#discussioncomment-6355126
        let client= client_builder.build().expect("Error building HTTPS bin");

        HttpClient {
            client,
            url,
            uid_field_name,
            spoof_check_secret
        }
    }

    pub async fn request_qr_code_and_sessionptr(&self) ->Result<QRCodeAndSessionPtr, Box<dyn Error>>{
        let result=reqwest::get(self.url.clone()+&std::string::String::from("/qr")).await;
        match result {
            Ok(response) => {
                let qr_code_and_sessionptr = response.text().await?;
                let qr_code_and_sessionptr: QRCodeAndSessionPtr = serde_json::from_str(&qr_code_and_sessionptr)?;
                Ok(qr_code_and_sessionptr)
            }
            Err(error) => {
                Err(Box::try_from(error).unwrap())
            }
        }

    }

    pub async fn get_irma_session_status(&self, session_token: &str)->Result<SessionStatus, GetStatusError>{
        //query the /status endpoint of the server component
        let response=reqwest::get((&self.url).to_owned()+"/status/"+session_token).await;
        match response {
            Ok(response) => {
                let response=response.text().await;
                match response{
                    Ok(status)=>match status.as_str() {
                        "INITIALIZED" => Ok(SessionStatus::Initialized),
                        "PAIRING" => Ok(SessionStatus::Pairing),
                        "CONNECTED" => Ok(SessionStatus::Connected),
                        "CANCELLED" => Ok(SessionStatus::Cancelled),
                        "DONE" => Ok(SessionStatus::Done),
                        "TIMEOUT" => Ok(SessionStatus::Timeout),

                        _ => {Err(GetStatusError{message: String::from("Unknown status")} )}
                    }
                    Err(error) => {Err(GetStatusError{message: error.to_string()} )}
                }



            }
            Err(error) => {Err(GetStatusError{message: error.to_string()} )}
        }

    }

}