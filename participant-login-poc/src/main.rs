mod irma_session_handler;
mod HTTPclient;

#[macro_use]
extern crate rocket;

use crate::irma_session_handler::IrmaSessionHandler;
use rocket::http::Status;
use rocket::response::stream::TextStream;
use rocket::response::{content, status};
use rocket::tokio::time::{sleep, Duration};
use std::fs;
use std::ops::Add;


struct Config {
    server_address: String,
    user_id: String,
    spoof_check_secret: String,
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
async fn irma_disclose_id() -> TextStream![String] {
    let irma_session_handler = IrmaSessionHandler::new("http://localhost:8088");
    let request_result = irma_session_handler
        .disclose_id(String::from("irma-demo.PEP.id.id"))
        .await;
    TextStream! {
        yield request_result.qr;

        // Periodically poll if the session was succesfully concluded
    let result = loop {
        match request_result.client.result(&request_result.session.token).await {
            Ok(result) => break result,
            Err(irma::Error::SessionNotFinished(_)) => {}
            Err(v) => panic!("{}", v),
        }

        sleep(Duration::from_secs(2)).await;
    };
        let disclosed=result.disclosed;
        let disclosed_attribute=&disclosed[0][0];
        let attribute_value=&disclosed_attribute.raw_value;
        match attribute_value{
            Some(attribute_value) => yield String::from("\nLogging in with token ").add(attribute_value),
            None => yield String::from("\nError: can't get attribute value")
        }
    }


}

async fn oauth_request(server_address: &str, user_id: &str, spoof_check_secret: &str) {
    let client = HTTPclient::HTTPclient::new(server_address, "Shib-Session-ID", spoof_check_secret);
    let request_result = client.send_auth_request(&String::from(user_id), &String::from(spoof_check_secret)).await;
    match request_result {
        Ok(response) => println!("Response: {:?}", response),
        Err(error) => println!("Error: {:?}", error)
    }
}

#[launch]
fn rocket() -> _ {
    //open config.toml configuartion file
    let config = fs::read_to_string("config/config.toml").expect("Error reading config/config.toml file");
    //parse config.toml
    let config: toml::Value = toml::from_str(&config).expect("Error parsing config/config.toml file");
    //get path_to_spoof_check_secret_file from config.toml
    let path_to_spoof_check_secret_file = config["path_to_spoof_check_secret_file"]
        .as_str()
        .expect("Error parsing path_to_spoof_check_secret_file from config/config.toml");
    //get uid_field_name from config.toml
    let uid_field_name = config["uid_field_name"]
        .as_str()
        .expect("Error parsing uid_field_name from config/config.toml");
    //get server_address from config.toml
    let auth_server_address = config["auth_server_address"]
        .as_str()
        .expect("Error parsing auth_server_address from config/config.toml");
    //get spoof_check_secret from path_to_spoof_check_secret_file
    let spoof_check_secret = fs::read_to_string(path_to_spoof_check_secret_file)
        .expect("Error reading spoof_check_secret file indicated in config/config.toml");
    //instantiate HTTPclient
    let client = HTTPclient::HTTPclient::new(auth_server_address, "Shib-Session-ID", spoof_check_secret.as_str());
    rocket::build().mount("/", routes![index, irma_disclose_id]).manage(Config {
        server_address: String::from(auth_server_address),
        user_id: String::from(uid_field_name),
        spoof_check_secret: spoof_check_secret,
    })
}
