mod http_client;
mod irma_session_handler;


use std::error::Error;
use crate::irma_session_handler::{IrmaSessionHandler, RequestResult};
use crate::http_client::HttpClient;
use std::fs;
use dioxus::prelude::*;
use irma::{SessionResult, SessionStatus, SessionToken};
use tera::Tera;

enum CurrentStatus { StartUp, Disclose, Success, Error }

struct State {
    config: Config,
    template_engine: Tera,
    irma_session_handler: IrmaSessionHandler,
    http_client: HttpClient,
    current_status: CurrentStatus,
}

struct Config {
    server_address: String,
    user_id: String,
    spoof_check_secret: String,
    uid_field_name: String,
}


struct Codes {
    code: String,
    code_verifier: [u8; 32],
}

#[derive(PartialEq, Props)]
pub struct IrmaSessionId {
    session_id: String,
}

#[derive(PartialEq, Props)]
pub struct QrCode {
    qr: String,
}


fn main() {
    dioxus_desktop::launch(App);
}

/**
 * Dioxus' entry point
 * # Arguments
 * * `cx` - The scope
 */
#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
fn App(cx: Scope<'_>) -> Element<'_> {
    //open and parse config.toml configuration file
    let config =
        fs::read_to_string("config/config.toml").expect("Error reading config/config.toml file");
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
    //remove eventual withespace characters, including newlines
    let spoof_check_secret = spoof_check_secret.trim().to_string();
    //parse Tera templates
    let mut tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    tera.autoescape_on(vec![]); //Turns escaping OFF, otherwise the SVG containing the QR code in the disclose page gets displayed as text (i.e, the text description of the SVG format, no image)

    let irma_session_handler = IrmaSessionHandler::new(irma_server_address);
    let http_client = http_client::HttpClient::new(auth_server_address.parse().unwrap(), uid_field_name.parse().unwrap(), spoof_check_secret.parse().unwrap(), path_to_root_ca_certificate.parse().unwrap());


    let config = Config {
        server_address: String::from(auth_server_address),
        user_id: String::from(uid_field_name),
        spoof_check_secret: spoof_check_secret,
        uid_field_name: String::from(uid_field_name),
    };

    match irma_session_handler {
        Ok(irma_session_handler) => {   //If I can create the IRMA session handler, the application works as expected
            let status = State {
                config: config,
                template_engine: tera,
                irma_session_handler: irma_session_handler,
                http_client: http_client,
                current_status: CurrentStatus::StartUp,
            };

            use_shared_state_provider(cx, || status);

            let status = use_shared_state::<State>(cx).unwrap().read();  //Get a new reference since I lost ownership by calling use_shared_state_provider

            match status.current_status {
                CurrentStatus::StartUp => {
                    render! {Startup{ }}
                }
                CurrentStatus::Disclose => {
                    render! {Disclose{}}
                }
                CurrentStatus::Success => { cx.render(rsx!("Logged in")) }
                CurrentStatus::Error => { cx.render(rsx!("Error")) }
            }
        }
        Err(error) => { //If I can't create the IRMA session handler, the application can't work as expected. TODO: somehow this doesn't work and the error is reported only later, after clicking on the button
            let status = use_shared_state::<State>(cx).unwrap();
            status.write().current_status = CurrentStatus::Error;
            render! {error.to_string()}
        }
    }
}



/**
 * Dioxus component that displays a button to start the IRMA session
 * # Arguments
 * * `cx` - The scope
 */
#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
pub fn Startup(cx: Scope) -> Element {
    let status = use_shared_state::<State>(cx).unwrap();
    cx.render(rsx!(button{
                onclick: move |event| status.write().current_status=CurrentStatus::Disclose,
                "Login with Yivi app"
            }))
}

/**
 * Dioxus component that starts an IRMA session and displays the QR code containing the IRMA session pointer
 * # Arguments
 * * `cx` - The scope
 */
#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
pub fn Disclose(cx: Scope) ->Element{

    let status = use_shared_state::<State>(cx).unwrap();
    let qr_and_session_id = use_future(
        cx, (),
        {
            to_owned![status];
            move |_| async move { irma_disclose_id(&status.read().template_engine, &status.read().irma_session_handler).await }
        },
    ).value();

    match qr_and_session_id {
        None => {
            cx.render(rsx!(div{"Waiting for the IRMA server to respond..."}))
        }
        Some(Ok(qr_and_session_id)) => {
            let session_id = &qr_and_session_id.session.token.0;
            let qr_code = qr_and_session_id.qr.clone();
            //status.write().current_status=CurrentStatus::Disclose;   //Go to next step
            cx.render(rsx!{
                Qr{qr: qr_code},
                IrmaSessionStatus{session_id: session_id.to_string()},
            })
        }
        Some(Err(error)) => {
            cx.render(rsx!(div{"Error, can't connect to IRMA server. Please try again later. If you would like to report this error, please include the following information: {error.to_string()}"}))
        }
    }

}


/**
 * Dioxus component that polls and displays the status of the IRMA session with the given session ID.
 * Once the session is done, it updates the status of the application to Success.
 * # Arguments
 * * `cx` - The scope
 * * `session_id` - The session ID to get the status of
 */
#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
pub fn IrmaSessionStatus(cx: Scope<IrmaSessionId>)->Element{
    let status = use_shared_state::<State>(cx).unwrap();
    let session_id= cx.props.session_id.clone();
    let future_irma_session_result = use_future(
        cx, (),
        {
            to_owned![status];
            move |_| async move { get_status(&session_id, &status.read().irma_session_handler).await }
        },
    );
    let irma_session_result = future_irma_session_result.state();   //I need two separate variables, so that I can restart the future later. I restart the future until I have a result to implement polling

    match irma_session_result {
        dioxus::prelude::UseFutureState::Pending => {
            println!("Pending...");
            cx.render(rsx!(div{"Please scan the QR code with the Yivi app."}))
        }
        dioxus::prelude::UseFutureState::Complete(irma_session_result) => {
            print!("Complete, ");
            match irma_session_result {
                Ok(session_result) => {
                    match session_result.status {
                        irma::SessionStatus::Timeout=>{
                            println!("timeout");
                            cx.render(rsx!(div{"Login session timed out, please try again."}))
                        }
                        irma::SessionStatus::Cancelled=>{
                            println!("cancelled");
                            cx.render(rsx!(div{"Login session cancelled, please try again."}))
                        }
                        irma::SessionStatus::Done=>{
                            println!("done");
                            //status.write().current_status=CurrentStatus::Success;   //Go to next step
                            cx.render(rsx!(div{"Login session done, please wait while we log you in."})) //It's actually lying
                        }
                        _=> {
                            println!("other status: {:?}", session_result.status);
                            cx.render(rsx!(div{"Please scan the QR code with the Yivi app."}))
                        }
                    }
                }
                Err(error) => {
                    println!("irma_session_result contains error {:?}", error.to_string());
                    match error {
                        irma::Error::InvalidUrl(_) => {cx.render(rsx!(div{"Error, can't connect to IRMA server. Please try again later. If you would like to report this error, please include the following information: {error.to_string()}"}) )}
                        irma::Error::NetworkError(_) => {cx.render(rsx!(div{"Error, can't connect to IRMA server. Please try again later. If you would like to report this error, please include the following information: {error.to_string()}"}))}
                        irma::Error::SessionCancelled => {cx.render(rsx!(div{"Login session cancelled, please try again."}))}
                        irma::Error::SessionTimedOut => {cx.render(rsx!(div{"Login session timed out, please try again."}))}
                        irma::Error::SessionNotFinished(_) => {
                            println!("Session not finished, restarting...");
                            future_irma_session_result.restart();
                            cx.render(rsx!(div{"Please scan this QR code with the Yivi app and share your data."}))}
                    }
                }
            }

        }
        dioxus::prelude::UseFutureState::Reloading(_)=> cx.render(rsx!(div{"Waiting for the IRMA login session to complete..."}))
    }
}
/**
 * Dioxus component that renders the QR code containing the IRMA session pointer
 * # Arguments
 * * `cx` - The scope
 * * `qr` - The QR code to render
 */
#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
pub fn Qr(cx: Scope<QrCode>) -> Element {
    let status = use_shared_state::<State>(cx).unwrap();
    let qr =&cx.props.qr;

    let mut context = tera::Context::new();
    context.insert("Qr", qr);
    let html = status.read().template_engine.render("disclose.html", &context).unwrap();


    cx.render(
        rsx!(div{
                        dangerous_inner_html: "{html}"
                    })
    )
}

/**
 * Starts an IRMA session to disclose the user's PEP ID
 * # Arguments
 * * `template_engine` - The template engine to use to render the QR code
 * * `irma_session_handler` - The IRMA session handler to use to start the session
 */
async fn irma_disclose_id(template_engine: &Tera, irma_session_handler: &IrmaSessionHandler) -> Result<RequestResult, irma::Error> {
    let request_result = irma_session_handler
        .disclose_id(String::from("irma-demo.PEP.id.id"))
        .await?;

    Ok(request_result)
}


/**
 * Gets the status of the IRMA session with the given session ID
 * # Arguments
 * * `session_id` - The session ID to get the status of
 * * `irma_session_handler` - The IRMA session handler to use to get the status
 */
async fn get_status(session_id: &String, irma_session_handler: &IrmaSessionHandler) -> Result<SessionResult, irma::Error> {
    println!("Called get_status with session_id: {}", session_id);
    let session_token = SessionToken(session_id.to_string());
    let result=irma_session_handler.get_status(&session_token).await;
    println!("get_status is about to return {:?}", result);
    return result;
}


async fn success(session_id: String, irma_session_handler: IrmaSessionHandler, template_engine: Tera, config: Config, http_client: HttpClient) -> String {
    let session_token = SessionToken(session_id);
    let session_result = irma_session_handler.get_status(&session_token).await;
    let disclosed_attribute = session_result.unwrap().disclosed[0][0].clone().raw_value.unwrap(); //TODO: see if this expression can be simplified
    let request = request_code_for_token(&config.server_address, &disclosed_attribute, &config.spoof_check_secret, &config.uid_field_name, &http_client);
    let mut context = tera::Context::new();
    let code_for_token = request.await;
    return match code_for_token {
        Ok(codes) => {
            context.insert("disclosed_attribute", &disclosed_attribute);
            context.insert("code_for_token", &codes.code);
            context.insert("auth_server_base_url", &config.server_address);
            context.insert("code_verifier", &codes.code_verifier);
            let rendered_html = template_engine.render("Success.html", &context).unwrap();
            rendered_html
        }
        Err(error) => {
            context.insert("error_message", &error.to_string());
            let rendered_html = template_engine.render("error.html", &context).unwrap();
            rendered_html
        }
    };
}






/**Sends an HTTP request to PEP's auth server containing the headers with the disclosed attribute
* # Arguments
* * `server_address` - The base URL of the PEP auth server
* * `user_id` - The user ID to send in the HTTP header
* * `spoof_check_secret` - The secret to use for the Shibboleth spoof check
* * `uid_field_name` - The name of the HTTP header that contains the user ID
* * `bin` - The HTTP bin to use to send the request
*/
async fn request_code_for_token(server_address: &str, user_id: &str, spoof_check_secret: &str, uid_field_name: &str, client: &HttpClient) -> Result<Codes, Box<dyn Error>> {
    let auth_response = client
        .send_auth_request(&String::from(user_id))
        .await?;
    let redirect_url = auth_response.response.headers()["location"].to_str()?;
    let code_verifier = auth_response.code_verifier;
    let code = redirect_url.split('=').collect::<Vec<&str>>()[1].to_owned();

    let result = Codes {
        code,
        code_verifier,
    };
    Ok(result)
}

