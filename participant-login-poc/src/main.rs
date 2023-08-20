mod http_client;
mod irma_session_handler;


use std::error::Error;
use crate::irma_session_handler::IrmaSessionHandler;
use crate::http_client::HttpClient;
use std::fs;
use dioxus::html::input;
use dioxus::prelude::*;
use irma::{SessionStatus, SessionToken};
use tera::Tera;

enum CurrentStatus { StartUp, Qr, Success, Error }

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


async fn irma_disclose_id(template_engine: &Tera) -> String {
    let irma_session_handler = IrmaSessionHandler::new("http://localhost:8088");    //TODO: get from global shared state


    let request_result = irma_session_handler
        .disclose_id(String::from("irma-demo.PEP.id.id"))
        .await;

    let qr = request_result.qr;

    //render Tera template showing the Qr code
    let mut context = tera::Context::new();
    context.insert("Qr", &qr);
    context.insert("session_id", &request_result.session.token.0);
    template_engine.render("disclose.html", &context).unwrap()
}


async fn get_status(session_id: String, irma_session_handler: IrmaSessionHandler) -> String {
    let session_token = SessionToken(session_id);
    let sesion_result = irma_session_handler.get_status(&session_token).await;

    match sesion_result {
        Ok(session_result) => {
            match session_result.status {
                SessionStatus::Initialized => String::from("Initialized"),
                SessionStatus::Pairing => String::from("Pairing"),
                SessionStatus::Connected => String::from("Connected"),
                SessionStatus::Cancelled => String::from("Cancelled"),
                SessionStatus::Done => String::from("Done"),
                SessionStatus::Timeout => String::from("Timeout")
            }
        }

        Err(error) => {
            error.to_string()
        }
    }
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


#[allow(non_snake_case)] //PascalCase isn't just a convention in Dioxus
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
    let uid_field_name = config["uid_field_name"]
        .as_str()
        .expect("Error parsing uid_field_name from config/config.toml");
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

    //simple_logger::init_with_env().unwrap_or_else(simple_logger::init_with_level(log::Level::Warn).unwrap());   //logging, see https://docs.rs/simple_logger/4.2.0/simple_logger/

    let config = Config {
        server_address: String::from(auth_server_address),
        user_id: String::from(uid_field_name),
        spoof_check_secret: spoof_check_secret,
        uid_field_name: String::from(uid_field_name),
    };
    let status = State {
        config: config,
        template_engine:  tera,
        irma_session_handler: irma_session_handler,
        http_client: http_client,
        current_status: CurrentStatus::StartUp,
    };

    use_shared_state_provider(cx, || status);

    let status=use_shared_state::<State>(cx).unwrap().read();  //Get a new reference since I lost ownership by calling use_shared_state_provider

    match status.current_status {
        CurrentStatus::StartUp => {
            render!{Startup{ }}
        }
        CurrentStatus::Qr => {
            render!{Qr{ }}
        }
        CurrentStatus::Success => { cx.render(rsx!("Logged in")) }
        CurrentStatus::Error => { cx.render(rsx!("Error")) }
    }
}

#[allow(non_snake_case)] //PascalCase isn't just a convention in Dioxus
pub fn Startup(cx: Scope) -> Element{
    let status=use_shared_state::<State>(cx).unwrap();
    cx.render(rsx!(button{
                onclick: move |event| status.write().current_status=CurrentStatus::Qr,
                "Login with Yivi app"
            }))
}
#[allow(non_snake_case)] //PascalCase isn't just a convention in Dioxus
pub fn Qr(cx: Scope) -> Element {
    let status=use_shared_state::<State>(cx).unwrap();
    let contents = use_future(
        cx, (),
        {to_owned![status]; move |_| async move { irma_disclose_id(&status.read().template_engine).await} }

    ).value().unwrap();
    cx.render(rsx! {
    div {
        dangerous_inner_html: "{contents}"
    }
})
}

///Sends an HTTP request to PEP's auth server containing the headers with the disclosed attribute
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

fn main() {
    dioxus_desktop::launch(App);
}