mod irma_session_handler;
mod http_client;

use std::error::Error;
use std::fs;
use std::future::Future;
use std::fmt;
use reqwest::Response;
use serde_json::json;
use rocket::{launch, get, routes, State};
use rocket::http::Status;
use rocket::response::{status, content};
use crate::http_client::HttpClient;
use crate::irma_session_handler::{IrmaSessionHandler, RequestResult};

#[derive(Debug)]
struct Codes {
    code: String,
    code_verifier: [u8; 32],
}

/**
    * My own error type for the get_codes_for_token function.
    * I need this to avoid using the `dyn` keyword in the function's signature, which isn't thread-safe.
    */
type CodeResult<T> = std::result::Result<T, GetCodesError>;
#[derive(Debug, Clone)]
struct GetCodesError {
    message: String,
}

impl fmt::Display for GetCodesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GetTokenError: {}", self.message)
    }
}



/**
    * Starts an IRMA session to disclose the user's PEP ID and returns a QR code to perform the session
    * # Arguments
    * * `irma_session_handler` - The IRMA session handler to use to start the session
    */
#[get("/qr")]
pub async fn qr(irma_session_handler: &State<IrmaSessionHandler>) -> status::Custom<content::RawJson<String>> {
    let request_result = irma_disclose_id(irma_session_handler).await;
    match request_result {
        Ok(request_result) => {
            let sessionptr = request_result.session.token.0;
            let json = json!({
                "qr": request_result.qr,
                "sessionptr": sessionptr
            }).to_string();
            status::Custom(Status::Ok, content::RawJson(json))
        }

        Err(error) => {
            let json = json!({
                "error": error.to_string()
            }).to_string();
            status::Custom(Status::InternalServerError, content::RawJson(json))
        }
    }
}

/**
    * Returns the status of an IRMA session
    * # Arguments
    * * `sessionptr` - The session pointer of the IRMA session
    * * `irma_session_handler` - The IRMA session handler to use to get the status
    */
#[get("/status/<sessionptr>")]
pub async fn irma_session_status(sessionptr: &str, irma_session_handler: &State<IrmaSessionHandler>) -> status::Custom<content::RawJson<String>> {
    let session_token = serde_json::from_str(sessionptr).expect("Error parsing session pointer");
    let session_result = irma_session_handler.get_status(&session_token).await;
    match session_result {
        Ok(session_result) => {
            let json = json!({
                "status": session_result.status,
                "attributes": session_result.disclosed
            }).to_string();
            status::Custom(Status::Ok, content::RawJson(json))
        }

        Err(error) => {
            let json = json!({
                "error": error.to_string()
            }).to_string();
            status::Custom(Status::InternalServerError, content::RawJson(json))
        }
    }
}

/**
    * Returns the PEP token for an IRMA session, if the attributes were disclosed successfully
    * # Arguments
    * * `sessionptr` - The session pointer of the IRMA session
    * * `irma_session_handler` - The IRMA session handler to use to get the result
    */
#[get("/token/<sessionptr>")]
pub async fn irma_session_result(sessionptr: &str, irma_session_handler: &State<IrmaSessionHandler>, http_client: &State<HttpClient>) -> status::Custom<content::RawJson<String>> {
    let session_token = serde_json::from_str(sessionptr).expect("Error parsing session pointer");
    let session_result = irma_session_handler.get_status(&session_token).await;
    match session_result {
        Ok(session_result) => {
            let uid = &session_result.disclosed[0][0].raw_value;
            match uid {
                None => {
                    let json = json!({
                "error": "No UID found in IRMA session result"
            }).to_string();
                    return status::Custom(Status::InternalServerError, content::RawJson(json));
                }
                Some(uid) => {
                    let pep_codes = request_code_for_token(uid, &http_client).await;
                    match pep_codes {
                        Ok(pep_codes)=>{
                            println!("PEP codes: {:?}", pep_codes);
                            let code=pep_codes.code;
                            let code_verifier=pep_codes.code_verifier;
                            return match http_client.get_token(&code, &code_verifier).await {
                                Ok(response)=>{
                                    let json = json!({
                                        "token": response.text().await.unwrap()
                                    }).to_string();
                                    status::Custom(Status::Ok, content::RawJson(json))
                                },
                                _ => {
                                    let json = json!({
                                        "error": "Error getting token"
                                    }).to_string();
                                    status::Custom(Status::InternalServerError, content::RawJson(json))
                                }
                            }
                        },
                        Err(error)=>{
                            let json = json!({
                                "error": error.to_string()
                            }).to_string();
                            status::Custom(Status::InternalServerError, content::RawJson(json))
                        },

                    }
                }
            }


        }

        Err(error) => {
            let json = json!({
                "error": error.to_string()
            }).to_string();
            status::Custom(Status::InternalServerError, content::RawJson(json))
        }
    }
}

#[launch]
fn rocket() -> _ {
    //open and parse config.toml configuration file
    let config =
        fs::read_to_string("crates/server/config/config.toml").expect("Error reading config/config.toml file");
    let config: toml::Value =
        toml::from_str(&config).expect("Error parsing config/config.toml file");

    //Get configuration values from configuration file
    let path_to_spoof_check_secret_file = config["path_to_spoof_check_secret_file"]
        .as_str()
        .expect("Error parsing path_to_spoof_check_secret_file from config/config.toml");
    let path_to_root_ca_certificate = config["path_to_root_ca_certificate"]
        .as_str()
        .expect("Error parsing path_to_root_ca_certificate from config/config.toml");
    let uid_field_name = config["uid_field_name"]
        .as_str()
        .expect("Error parsing uid_field_name from config/config.toml");
    let auth_server_address = config["auth_server_address"]
        .as_str()
        .expect("Error parsing auth_server_address from config/config.toml");
    let irma_server_address = config["irma_server_address"]
        .as_str()
        .expect("Error parsing irma_server_address from config/config.toml");


    //get spoof_check_secret from path_to_spoof_check_secret_file
    let spoof_check_secret = fs::read_to_string(path_to_spoof_check_secret_file)
        .expect("Error reading spoof_check_secret file indicated in config/config.toml");
    //remove eventual whitespace characters, including newlines
    let spoof_check_secret = spoof_check_secret.trim().to_string();

    let irma_session_handler = IrmaSessionHandler::new(irma_server_address).expect("Error creating IrmaSessionHandler");
    let http_client = http_client::HttpClient::new(auth_server_address.to_string(), uid_field_name.to_string(), spoof_check_secret, path_to_root_ca_certificate.to_string());

    rocket::build()
        .mount("/base", routes![qr])
        .manage(irma_session_handler)
        .manage(http_client)
}

/**
 * Starts an IRMA session to disclose the user's PEP ID
 * # Arguments
 * * `template_engine` - The template engine to use to render the QR code
 * * `irma_session_handler` - The IRMA session handler to use to start the session
 */
async fn irma_disclose_id(irma_session_handler: &IrmaSessionHandler) -> Result<RequestResult, irma::Error> {
    let request_result = irma_session_handler
        .disclose_id(String::from("irma-demo.PEP.id.id"))
        .await?;

    Ok(request_result)
}



/** Sends an HTTP request to PEP's auth server containing the headers with the disclosed attribute
* # Arguments
* * `server_address` - The base URL of the PEP auth server
* * `user_id` - The user ID to send in the HTTP header
* * `spoof_check_secret` - The secret to use for the Shibboleth spoof check
* * `uid_field_name` - The name of the HTTP header that contains the user ID
* * `client` - The HTTP client to use to send the request
 */
async fn request_code_for_token(user_id: &str, client: &HttpClient) -> CodeResult<Codes> {
    let auth_response = client
        .send_auth_request(&String::from(user_id))
        .await;
    match auth_response{
        Ok(auth_response)=>{
            let redirect_url = auth_response.response.headers()["location"].to_str();
            match redirect_url{
                Ok(redirect_url)=>{
                    let code_verifier = auth_response.code_verifier;
                    let code = redirect_url.split('=').collect::<Vec<&str>>()[1].to_owned();

                    let result = Codes {
                        code,
                        code_verifier,
                    };
                    Ok(result)
                }
                Err(e)=>{
                    Err(GetCodesError {
                        message: e.to_string(),
                    })
                }
            }

        }
        Err(error)=>{
            Err(GetCodesError {
                message: error.to_string(),
            })
        }
    }
}




