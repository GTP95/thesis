mod HTTPclient;
mod irma_session_handler;

#[macro_use]
extern crate rocket;

use crate::irma_session_handler::IrmaSessionHandler;
use rocket::http::Status;
use rocket::response::stream::TextStream;
use rocket::response::{content, status};
use rocket::tokio::time::{sleep, Duration};
use rocket::State;
use std::fs;
use std::ops::Add;
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
    let template = template_engine.tera.render("disclose.html", &context).unwrap();
    content::RawHtml(template)
}

async fn oauth_request(server_address: String, user_id: String, spoof_check_secret: String, uid_field_name: String) {
    let client = HTTPclient::HTTPclient::new(server_address, uid_field_name, spoof_check_secret.clone());
    let request_result = client
        .send_auth_request(&String::from(user_id), &spoof_check_secret)
        .await;
    match request_result {
        Ok(response) => println!("Response: {:?}", response),
        Err(error) => println!("Error: {:?}", error),
    }
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
    let uid_field_name = config["uid_field_name"]
        .as_str()
        .expect("Error parsing uid_field_name from config/config.toml");
    let auth_server_address = config["auth_server_address"]
        .as_str()
        .expect("Error parsing auth_server_address from config/config.toml");
    let uid_field_name= config["uid_field_name"]
        .as_str()
        .expect("Error parsing uid_field_name from config/config.toml");

    //get spoof_check_secret from path_to_spoof_check_secret_file
    let spoof_check_secret = fs::read_to_string(path_to_spoof_check_secret_file)
        .expect("Error reading spoof_check_secret file indicated in config/config.toml");

    //parse Tera templates
    let mut tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    tera.autoescape_on(vec![]); //Turns escaping OFF, otherwise the SVG containing the QR code in the disclose page gets displayed as text (i.e, the text description of the SVG format, no image)

    rocket::build()
        .mount("/", routes![index, irma_disclose_id])
        .manage(Config {
            server_address: String::from(auth_server_address),
            user_id: String::from(uid_field_name),
            spoof_check_secret: spoof_check_secret,
            uid_field_name: String::from(uid_field_name)
        }).manage(TemplateEngine{tera})
}
