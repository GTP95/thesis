mod http_client;
mod irma_session_handler;
mod pepcli_wrapper;
mod file_browser;

use crate::irma_session_handler::{IrmaSessionHandler, RequestResult};
use crate::http_client::HttpClient;
use std::fs;
use std::path::PathBuf;
use dioxus::html::{br, h1, h2, img};
use dioxus::prelude::*;
use irma::{SessionResult, SessionToken};
use log::debug;
use tera::Tera;
use http_client::IrmaSessionStatus;
use pepcli_wrapper::PepCliWrapper;
use crate::file_browser::Path;

enum CurrentStatus { StartUp, Disclose, IrmaSessionDone, DownloadFiles, BrowseFiles(PathBuf), Error(String) }

struct State {
    config: Config,
    template_engine: Tera,
    irma_session_id: Option<String>,
    http_client: HttpClient,
    current_status: CurrentStatus,
    irma_session_ptr: Option<String>,
    path_to_pepcli: String,
    pepcli_wrapper: Option<PepCliWrapper>,
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
pub struct IrmaSessionPtr {
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

#[derive(PartialEq, Props)]
pub struct Error {
    error_message: String,
}


fn main() {
    env_logger::init();  //Set up logging
    dioxus_desktop::launch(App);
}

/**
 * Dioxus' entry point
 * # Arguments
 * * `cx` - The scope
 */
#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
fn App(cx: Scope<'_>) -> Element<'_> {
    /* every time the application is re-rendered, it executes this function again. Re-initializing
    the configuration isn't just wasteful: it actually causes a crash when the Browser function inside
    file_browser changes the current working directory: after that, this function can't find the
    configuration file anymore. So I'm initializing the state only the very first time this gets executed.
     */
    if use_shared_state::<State>(cx).is_none() {
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
        let path_to_pepcli = config["path_to_pepcli"]
            .as_str()
            .expect("Error parsing path_to_pepcli from config/config.toml");


        //get spoof_check_secret from path_to_spoof_check_secret_file
        let spoof_check_secret = fs::read_to_string(path_to_spoof_check_secret_file)
            .expect("Error reading spoof_check_secret file indicated in config/config.toml");
        //remove eventual withespace characters, including newlines
        let spoof_check_secret = spoof_check_secret.trim().to_string();
        //parse Tera templates
        let mut tera = match Tera::new("templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                debug!("Parsing error(s): {}", e);
                std::process::exit(1);
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
            irma_session_ptr: None,
            path_to_pepcli: path_to_pepcli.to_string(),
            pepcli_wrapper: None,
        };

        use_shared_state_provider(cx, || status);
    }
    let status = use_shared_state::<State>(cx).unwrap().read();  //Get a new reference since I lost ownership by calling use_shared_state_provider

    match &status.current_status {
        CurrentStatus::StartUp => {
            render! {Startup{ }}
        }
        CurrentStatus::Disclose => {
            render! {Disclose{}}
        }
        CurrentStatus::IrmaSessionDone => {
            match &status.irma_session_ptr {
                Some(session_id) => {
                    render! {GetPEPtoken{session_id: session_id.to_owned()}}
                }
                None => {
                    cx.render(rsx!("Error: IRMA session ID not found"))
                }
            }
        }
        CurrentStatus::DownloadFiles => { render! {DownloadFiles{}} }
        CurrentStatus::BrowseFiles(path) => {render!{file_browser::Browser{path: path.clone()}}}
        CurrentStatus::Error(error_message) => {

            render! {Error{error_message: error_message.to_string()}}
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
    cx.render(rsx!(
         h1 { "PEP PLP" }
        h2{"PEP Participant Login Portal"}
        button{
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
pub fn Disclose(cx: Scope) -> Element {
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
            cx.render(rsx!(div{"Waiting for the IRMA server to respond..."}))
        }
        Some(Ok(qr_and_session_id)) => {
            let session_id = &qr_and_session_id.session_ptr;
            let qr_code = &qr_and_session_id.qr_code;
            //status.write().current_status=CurrentStatus::Disclose;   //Go to next step

            cx.render(rsx! {
    Qr{qr: qr_code.to_string()},
    IrmaSessionStatus{session_id: session_id.to_string()},
    })
        }
        Some(Err(error)) => {
            status.write().current_status = CurrentStatus::Error(error.to_string());
            cx.render(rsx!(div{"Error, can't get the QR code needed for authentication. Please try again later. If you would like to report this error, please include the following information: {error.to_string()}"}))   //This will not get rendered, as updating the status triggers a re-render
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
pub fn IrmaSessionStatus(cx: Scope<IrmaSessionPtr>) -> Element {
    let status = use_shared_state::<State>(cx).unwrap();
    let session_id = cx.props.session_id.clone();

    status.write().irma_session_ptr = Some(session_id.clone()); //Store the session ID in the state so that I can use it to get PEP's token at the end of the session

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
                        IrmaSessionStatus::Timeout => {
                            debug!("timeout");
                            cx.render(rsx!(div{"Login session timed out, please try again."}))
                        }
                        IrmaSessionStatus::Cancelled => {
                            debug!("cancelled");
                            cx.render(rsx!(div{"Login session cancelled, please try again."}))
                        }
                        IrmaSessionStatus::Done => {
                            debug!("done, updating status");
                            status.write().current_status = CurrentStatus::IrmaSessionDone;   //Go to next step
                            cx.render(rsx!(div{"Login session done, please wait while we log you in."})) //It's actually lying
                        }
                        IrmaSessionStatus::NotFinished => {
                            debug!("not finished");
                            future_irma_session_result.restart(); //Restart the future to poll again
                            cx.render(rsx!(div{"Please scan the QR code with the Yivi app."}))
                        }
                        _ => {
                            debug!("other");
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
        dioxus::prelude::UseFutureState::Reloading(_) => cx.render(rsx!(div{"Waiting for the IRMA login session to complete..."}))
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
    let qr = &cx.props.qr;

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
    let status = use_shared_state::<State>(cx).unwrap();
    let session_id = cx.props.session_id.clone();

    let response = use_future(
        cx, (),
        {
            to_owned![status];
            move |_| async move { status.read().http_client.get_pep_auth_token(session_id).await }
        },
    ).value();

    match response {
        None => {
            cx.render(rsx!(div{"Waiting for the authentication middleware to respond..."}))
        }
        Some(Ok(token_and_attribute)) => {    //TODO: this is the place to create an instance of PepcliWrapper
            let  mut writable_status=status.write();
            if writable_status.pepcli_wrapper.is_none(){
                writable_status.pepcli_wrapper=Some(PepCliWrapper::new(writable_status.path_to_pepcli.clone(), token_and_attribute.token.clone(), token_and_attribute.irma_attribute.clone()));
            }
            writable_status.current_status = CurrentStatus::DownloadFiles;   //Go to next step
            cx.render(rsx!(div{"If you're seeing this, something went wrong. You can drop an email to support@pep.cs.ru.nl about what happened."}))    //This should never get rendered, as updating the status triggers a re-render that moves the app to the next state
        }
        Some(Err(error)) => {
            cx.render(rsx!(div{"Error, can't get the PEP token. Please try again later. If you would like to report this error, please include the following information: {error.message}"}))
        }
    }
}

#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
pub fn DownloadFiles(cx: Scope) -> Element {
    debug!("DownloadFile");
    let status = use_shared_state::<State>(cx).expect("Error getting shared state inside DownloadFile");
    let pepcli_wrapper = status.read().pepcli_wrapper.clone();
    match pepcli_wrapper {
        Some(pepcli_wrapper) => {
            let download_files = use_future(
                cx, (),
                {
                    to_owned![pepcli_wrapper];
                    move |_| async move { pepcli_wrapper.download_all().await }
                },
            ).value();
            match download_files {
                Some(download_result) => {
                    match download_result {
                        Ok(path_to_plp_temp_dir) => {
                            debug!("Path to temp directory: {:?}", &path_to_plp_temp_dir);
                            status.write().current_status = CurrentStatus::BrowseFiles(path_to_plp_temp_dir.clone());   //Go to next step
                            cx.render(rsx!(div{"If you're seeing this, something went wrong. You can drop an email to support@pep.cs.ru.nl about what happened."}))    //This should never get rendered, as updating the status triggers a re-render that moves the app to the next state
                        }
                        Err(error) => {
                            debug!("Error fetching files: {}", error.to_string());
                            status.write().current_status = CurrentStatus::Error(error.to_string());
                            cx.render(rsx!(div{"If you're seeing this, something went wrong. You can drop an email to support@pep.cs.ru.nl about what happened."}))  //this doesn't get rendered, as updating the status triggers a re-render that moves the app to the next state
                        }
                    }
                }
                None => {

                    cx.render(rsx!(
                        h1{"Downloading files"}
                        div{"Please wait while we download your data."
                            br{}
                        img { src: "resources/loading.gif" }
                        }

                    ))
                }

            }
        }
        None => {
            debug!("Error: no PepCliWrapper instance found");
            status.write().current_status = CurrentStatus::Error("No PepCliWrapper instance found".to_string());
            cx.render(rsx!(div{"Error: no PepCliWrapper instance found"}))  //this doesn't get rendered, as updating the status triggers a re-render that moves the app to the next state, that renders another thing
        }
    }
}

#[allow(non_snake_case)] //UpperCamelCase isn't just a convention in Dioxus
pub fn Error(cx: Scope<Error>) -> Element {
    let error_message = &cx.props.error_message;
    cx.render(rsx!(
        h1{"Error"}
        div{"An error occurred, you can report it to the PEP team by sending an email to \
         support@pep.cs.ru.nl. Please include in your report the following error message:"}
        div{"{error_message}"}

    ))
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
fn extract_token_from_error_message(error_message: &str) -> String {
    //This can be made more robust against possible future changes of the error by writing a regex, but finding one that works can be tricky
    debug!("Going to extract a token from the following error message: {}", error_message);
    let start = error_message.find("code=");
    let end = error_message.find(")");
    let result = &error_message[start.unwrap() + 5..end.unwrap()];
    debug!("extracted token: {}", result);
    result.to_string()
}