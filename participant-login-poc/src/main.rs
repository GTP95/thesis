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
    rocket::build().mount("/", routes![index, irma_disclose_id])

}
