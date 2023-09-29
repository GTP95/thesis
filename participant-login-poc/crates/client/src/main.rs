mod http_client;
mod irma_session_handler;



use crate::irma_session_handler::{IrmaSessionHandler, RequestResult};
use crate::http_client::HttpClient;
use std::fs;
use dioxus::prelude::*;
use irma::{SessionResult, SessionToken};
use tera::Tera;
use http_client::IrmaSessionStatus;
use regex::Regex;
use lazy_static::lazy_static;

enum CurrentStatus { StartUp, Disclose, IrmaSessionDone, Success, Error }

struct State {
    config: Config,
    template_engine: Tera,
    irma_session_id: Option<String>,
    http_client: HttpClient,
    current_status: CurrentStatus,
    irma_session_ptr: Option<String>
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

#[derive(PartialEq, Props)]
pub struct SessionID {
    session_id: String,
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
    let middleware_auth_server_address = config["middleware_auth_server_address"]
        .as_str()
        .expect("Error parsing auth_server_address from config/config.toml");



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

    let http_client = http_client::HttpClient::new(middleware_auth_server_address.parse().unwrap(), uid_field_name.parse().unwrap(), spoof_check_secret.parse().unwrap(), path_to_root_ca_certificate.parse().unwrap());


    let config = Config {
        server_address: String::from(middleware_auth_server_address),
        user_id: String::from(uid_field_name),
        spoof_check_secret: spoof_check_secret,
        uid_field_name: String::from(uid_field_name),
    };

    let status = State {
        config: config,
        template_engine: tera,
        irma_session_id: None,
        http_client: http_client,
        current_status: CurrentStatus::StartUp,
        irma_session_ptr: None
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
        CurrentStatus::IrmaSessionDone => {
            match &status.irma_session_ptr{
                Some(session_id) => {
                    render! {GetPEPtoken{session_id: session_id.to_owned()}}
                }
                None => {
                    cx.render(rsx!("Error: IRMA session ID not found"))
                }
            }

        }
        CurrentStatus::Success => { cx.render(rsx!("Logged in")) }
        CurrentStatus::Error => { cx.render(rsx!("Error")) }
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
            move |_| async move { status.read().http_client.request_qr_code_and_sessionptr().await }
        },
    ).value();

    match qr_and_session_id {
        None => {
            cx.render(rsx!(div{"Waiting for the server to respond..."}))
        }
        Some(Ok(qr_and_session_id)) => {
            let session_id = &qr_and_session_id.session_ptr;
            let qr_code = &qr_and_session_id.qr_code;
            //status.write().current_status=CurrentStatus::Disclose;   //Go to next step
            cx.render(rsx!{
                Qr{qr: qr_code.to_string()},
                IrmaSessionStatus{session_id: session_id.to_string()},
            })
        }
        Some(Err(error)) => {
            cx.render(rsx!(div{"Error, can't get the QR code needed for authentication. Please try again later. If you would like to report this error, please include the following information: {error.to_string()}"}))
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
    status.write().irma_session_ptr=Some(session_id.clone()); //Store the session ID in the state so that I can use it to get PEP's token at the end of the session

    let future_irma_session_result = use_future(
        cx, (),
        {
            to_owned![status];
            move |_| async move { status.read().http_client.get_irma_session_status(&session_id).await }
        },
    );
    let irma_session_result = future_irma_session_result.state();   //I need two separate variables, so that I can restart the future later. I restart the future until I have a result to implement polling

    match irma_session_result {
        dioxus::prelude::UseFutureState::Pending => {
            cx.render(rsx!(div{"Please scan the QR code with the Yivi app."}))
        }
        dioxus::prelude::UseFutureState::Complete(irma_session_result) => {
            match irma_session_result {
                Ok(session_status) => {
                    match session_status {
                       IrmaSessionStatus::Timeout=>{
                            println!("timeout");
                            cx.render(rsx!(div{"Login session timed out, please try again."}))
                        }
                        IrmaSessionStatus::Cancelled=>{
                            println!("cancelled");
                            cx.render(rsx!(div{"Login session cancelled, please try again."}))
                        }
                        IrmaSessionStatus::Done=>{
                            println!("done, updating status");
                            status.write().current_status=CurrentStatus::IrmaSessionDone;   //Go to next step
                            cx.render(rsx!(div{"Login session done, please wait while we log you in."})) //It's actually lying
                        }
                        IrmaSessionStatus::NotFinished=> {
                            println!("not finished");
                            future_irma_session_result.restart(); //Restart the future to poll again
                            cx.render(rsx!(div{"Please scan the QR code with the Yivi app."}))
                        }
                        _ => {
                            println!("other");
                            future_irma_session_result.restart(); //Restart the future to poll again
                            cx.render(rsx!(div{"Please scan the QR code with the Yivi app."}))
                        }
                    }
                }
                Err(error) => {
                    cx.render(rsx!(div{"Error: {error.to_string()}"}))
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

#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
pub fn GetPEPtoken(cx: Scope<SessionID>) -> Element {
    let status=use_shared_state::<State>(cx).unwrap();
    let session_id = cx.props.session_id.clone();

    let token=use_future(
        cx, (),
        {
            to_owned![status];
            move |_| async move { status.read().http_client.get_pep_auth_token(session_id).await }
        },
    ).value();

    match token {
        None => {
            cx.render(rsx!(div{"Waiting for the server to respond..."}))
        }
        Some(Ok(token)) => {
            cx.render(rsx!(div{"PEP token: {token}"}))
        }
        Some(Err(error)) => {
            if error.message.contains("http://127.0.0.1:16515/?code="){ //The error contains the token. This is not elegant, but is the way to go since I've got nothing listening to the URL I get redirected to and PEP doesn't let me specify a different URL for the redirection (if I do, it gets ignored)

                let token=extract_token_from_error_message(&error.message);
                cx.render(rsx!(div{"PEP token: {token}"}))


            }
            else {
                cx.render(rsx!(div{"Error, can't get the PEP token. Please try again later. If you would like to report this error, please include the following information: {error.message}"}))
            }

        }
    }

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


/**
 * Gets the status of the IRMA session with the given session ID
 * # Arguments
 * * `session_id` - The session ID to get the status of
 * * `irma_session_handler` - The IRMA session handler to use to get the status
 */
async fn get_status(session_id: &String, irma_session_handler: &IrmaSessionHandler) -> Result<SessionResult, irma::Error> {
    let session_token = SessionToken(session_id.to_string());
    irma_session_handler.get_status(&session_token).await
}

/**
 * Extracts PEP's auth token from the error message returned when trying to follow the redirect to localhost:16515
    * # Arguments
    * * `error_message` - The error message
 */
fn extract_token_from_error_message(error_message: &str) -> String { //This can be made more robust against possible future changes of the error by writing a regex, but finding one that works can be tricky
    let start=error_message.find("code=");
    let end=error_message.find(")");
    let result=&error_message[start.unwrap()+5..end.unwrap()];
    println!("extracted token: {}", result);
    result.to_string()
}