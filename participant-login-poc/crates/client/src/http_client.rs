use std::fmt;
use std::fs::read;
use reqwest::redirect;
use serde::{Deserialize, Serialize};

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

/** Used to parse the server's response a sJSON */
#[derive(Serialize, Deserialize)]
pub struct IrmaSessionStatusResponse {
    attributes: Vec<Vec<Attribute>>,
    error: String,
    status: String,
}

#[derive(Serialize, Deserialize)]
pub struct Attribute {
    id: String,
    rawvalue: String,
    status: String,
    value: Value,
}

#[derive(Serialize, Deserialize)]
pub struct Value {
    en: String,
    nl: String,
}

/** Stores the QR code and the session pointer of an IRMA session */
#[derive(Serialize, Deserialize, Clone)]
pub struct QRCodeAndSessionPtr {
    pub qr_code: String,
    pub session_ptr: String
}

/** My own type to handle errors getting IRMA session status
 * Needs to be public to avoid ownership errors in the main.rs file
 */
#[derive(Debug, Clone)]
pub struct GetStatusError {
   pub message: String,
}

impl fmt::Display for GetStatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GetStatusError: {}", self.message)
    }
}

impl From<serde_json::Error> for GetStatusError {
    fn from(error: serde_json::Error) -> Self {
        GetStatusError {
            message: error.to_string()
        }
    }
}



/**My own generic error to sidestep ownership issues in Dioxus' multithreaded code */
#[derive(Debug, Clone)]
pub struct BoxedError {
    pub message: String,
}
impl fmt::Display for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BoxedError: {}", self.message)
    }
}


/**Converts a serde_json::Error into a BoxedError */
impl From<serde_json::Error> for BoxedError {
    fn from(error: serde_json::Error) -> Self {
        BoxedError {
            message: "serde_json error: ".to_owned()  + error.to_string().as_str()
        }
    }
}

/**Converts a reqwest::Error into a BoxedError */
impl From<reqwest::Error> for BoxedError {
    fn from(error: reqwest::Error) -> Self {
        BoxedError {
            message: error.to_string()
        }
    }
}

/** My own enum to represent IRMA session's status in a more logical way: curiously, the IRMA library
*   considers the "NotFinished" case as an error.
*/
#[derive(Debug, Clone)]
pub enum IrmaSessionStatus {
    Initialized,
    Pairing,
    Connected,
    Cancelled,
    Done,
    Timeout,
    NotFinished
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

    pub async fn request_qr_code_and_sessionptr(&self) ->Result<QRCodeAndSessionPtr, BoxedError>{
        let result=reqwest::get(self.url.clone()+&std::string::String::from("/qr")).await;
        match result {
            Ok(response) => {
                match response.status() {
                    reqwest::StatusCode::OK => {
                        let qr_code_and_sessionptr = response.text().await?;
                        println!("qr_code_and_sessionptr: {}", qr_code_and_sessionptr); //DEBUG
                        let qr_code_and_sessionptr: QRCodeAndSessionPtr = serde_json::from_str(&qr_code_and_sessionptr)?;
                        Ok(qr_code_and_sessionptr)
                    },
                    _ => {Err(BoxedError{message: format!("Error getting QR code and session pointer: {}", response.status())})}
                }

            }
            Err(error) => {
                Err(BoxedError{message: error.to_string()})
            }
        }

    }

    pub async fn get_irma_session_status(&self, session_token: &str)->Result<IrmaSessionStatus, GetStatusError>{
        //query the /status endpoint of the server component
        let response=reqwest::get((&self.url).to_owned()+"/status/"+session_token).await;
        match response {
            Ok(response) => {
                let response=response.text().await;
                println!("response: {:?}", response); //DEBUG
                let response: IrmaSessionStatusResponse = serde_json::from_str(&response.unwrap())?;
                match response.status.as_str() {
                    "Initialized" => {Ok(IrmaSessionStatus::Initialized)},
                    "Pairing" => {Ok(IrmaSessionStatus::Pairing)},
                    "Connected" => {Ok(IrmaSessionStatus::Connected)},
                    "Cancelled" => {Ok(IrmaSessionStatus::Cancelled)},
                    "DONE" => {Ok(IrmaSessionStatus::Done)},
                    "Timeout" => {Ok(IrmaSessionStatus::Timeout)},
                    "error" => {
                        if response.error=="Irma session not finished"{
                        Ok(IrmaSessionStatus::NotFinished)}
                        else{
                            Err(GetStatusError{message: format!("Error getting IRMA session status: {}", response.error)})
                        }},
                    _ => {Err(GetStatusError{message: format!("Error getting IRMA session status: invalid status returned: {}", response.status)})}
                }



            }
            Err(error) => {Err(GetStatusError{message: error.to_string()} )}
        }

    }

}