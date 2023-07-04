mod http_client;
mod irma_session_handler;

#[macro_use]
extern crate rocket;

use crate::irma_session_handler::IrmaSessionHandler;
use crate::http_client::HttpClient;
use rocket::http::Status;
use rocket::response::stream::TextStream;
use rocket::response::{content, status};
use rocket::tokio::time::{sleep, Duration};
use rocket::State;
use std::fs;
use std::ops::Add;
use irma::{SessionStatus, SessionToken};
use rocket::response::content::RawHtml;
use tera::Tera;

struct Config {
    server_address: String,
    user_id: String,
    spoof_check_secret: String,
    uid_field_name: String
}

struct TemplateEngine{
    tera: Tera
}


#[get("/")]
async fn index() -> status::Custom<content::RawHtml<String>> {
    let index = fs::read_to_string("static/index.html");
    status::Custom(
        Status::Accepted,
        content::RawHtml(index.expect("Error reading index page")),
    )
}

#[get("/disclose")]
async fn irma_disclose_id(template_engine: &State<TemplateEngine>) -> content::RawHtml<String> {
    let irma_session_handler = IrmaSessionHandler::new("http://localhost:8088");


    let request_result = irma_session_handler
        .disclose_id(String::from("irma-demo.PEP.id.id"))
        .await;

    let qr= request_result.qr;

    //render Tera template showing the qr code
    let mut context = tera::Context::new();
    context.insert("qr", &qr);
    context.insert("session_id", &request_result.session.token.0);
    let template = template_engine.tera.render("disclose.html", &context).unwrap();
    content::RawHtml(template)
}

#[get("/status/<session_id>")]
async fn get_status(session_id: String, irma_session_handler: &State<IrmaSessionHandler>) -> status::Custom<content::RawText<String>>{
    let session_token=SessionToken(session_id);
    let sesion_result =irma_session_handler.get_status(&session_token).await;

    match sesion_result {
        Ok(session_result) => {

            match  session_result.status {
                SessionStatus::Initialized => {status::Custom(Status::Accepted, content::RawText(String::from("Initialized")))}
                SessionStatus::Pairing => {status::Custom(Status::Accepted, content::RawText(String::from("Pairing")))}
                SessionStatus::Connected => {status::Custom(Status::Accepted, content::RawText(String::from("Connected")))}
                SessionStatus::Cancelled => {status::Custom(Status::Accepted, content::RawText(String::from("Cancelled")))}
                SessionStatus::Done => {status::Custom(Status::Accepted, content::RawText(String::from("Done")))}
                SessionStatus::Timeout => {status::Custom(Status::Accepted, content::RawText(String::from("Timeout")))}
            }
        }


        Err(error) => {
            let error=error.to_string();
            status::Custom(Status::InternalServerError, content::RawText(error))
        }
    }

}

#[get("/success/<session_id>")]
async fn success(session_id: String, irma_session_handler: &State<IrmaSessionHandler>, template_engine: &State<TemplateEngine>, config: &State<Config>, http_client: &State<HttpClient>) -> status::Custom<content::RawHtml<String>> {
    let session_token = SessionToken(session_id);
    let session_result = irma_session_handler.get_status(&session_token).await;
    let disclosed_attribute=session_result.unwrap().disclosed[0][0].clone().raw_value.unwrap(); //TODO: see if this expression can be simplified
    let request=oauth_request(&config.server_address, &disclosed_attribute, &config.spoof_check_secret, &config.uid_field_name, &http_client);
    let mut context = tera::Context::new();
    context.insert("disclosed_attribute", &disclosed_attribute);
    let template = template_engine.tera.render("success.html", &context).unwrap();
    request.await;
    status::Custom(Status::Accepted, content::RawHtml(template))
}

#[catch(404)]
fn not_found() -> RawHtml<String> {
    let not_found = fs::read_to_string("static/notfound.html");
    RawHtml(not_found.expect("Error reading 404 page"))
}



#[launch]
fn rocket() -> _ {
    //open and parse config.toml configuration file
    let config =
        fs::read_to_string("config/config.toml").expect("Error reading config/config.toml file");
    let config: toml::Value =
        toml::from_str(&config).expect("Error parsing config/config.toml file");

    //Get configuration values from configuration file
    let path_to_spoof_check_secret_file = config["path_to_spoof_check_secret_file"]
        .as_str()
        .expect("Error parsing path_to_spoof_check_secret_file from config/config.toml");
    let path_to_root_ca_certificate=config["path_to_root_ca_certificate"]
        .as_str()
        .expect("Error parsing path_to_root_ca_certificate from config/config.toml");
    let uid_field_name = config["uid_field_name"]
        .as_str()
        .expect("Error parsing uid_field_name from config/config.toml");
    let auth_server_address = config["auth_server_address"]
        .as_str()
        .expect("Error parsing auth_server_address from config/config.toml");
    let uid_field_name= config["uid_field_name"]
        .as_str()
        .expect("Error parsing uid_field_name from config/config.toml");
    let irma_server_address= config["irma_server_address"]
        .as_str()
        .expect("Error parsing irma_server_address from config/config.toml");

    //get spoof_check_secret from path_to_spoof_check_secret_file
    let spoof_check_secret = fs::read_to_string(path_to_spoof_check_secret_file)
        .expect("Error reading spoof_check_secret file indicated in config/config.toml");
    //remove eventual withespace characters, including newlines
    let spoof_check_secret= spoof_check_secret.trim().to_string();
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

    simple_logger::SimpleLogger::new().env().init().unwrap();   //logging, see https://docs.rs/simple_logger/4.2.0/simple_logger/

    rocket::build()
        .mount("/", routes![index, irma_disclose_id, get_status, success])
        .register("/", catchers![not_found])
        .manage(Config {
            server_address: String::from(auth_server_address),
            user_id: String::from(uid_field_name),
            spoof_check_secret: spoof_check_secret,
            uid_field_name: String::from(uid_field_name)
        })
        .manage(TemplateEngine{tera})
        .manage(irma_session_handler)
        .manage(http_client)
}

///Sends an HTTP request to PEP's auth server containing the headers with the disclosed attribute
async fn oauth_request(server_address: &str, user_id: &str, spoof_check_secret: &str, uid_field_name: &str, client: &HttpClient) {

    let request_result = client
        .send_auth_request(&String::from(user_id), &spoof_check_secret)
        .await;
    match request_result {
        Ok(response) => println!("Response: {:?}", response),
        Err(error) => {
            println!("Error: {:?}", error);
            println!("user_id: {:?}", user_id);
            println!("uid_field_name: {:?}", uid_field_name);
            println!("spoof_check_secret: {:?}", spoof_check_secret);

        },
    }
}

