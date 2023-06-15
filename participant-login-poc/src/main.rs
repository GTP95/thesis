mod irma_session_handler;
mod HTTPrequestBuilder;
mod HTTPclient;

#[macro_use]
extern crate rocket;

use crate::irma_session_handler::IrmaSessionHandler;
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::response::stream::TextStream;
use rocket::response::{content, status};
use rocket::tokio::time::{interval, sleep, Duration};
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

fn oauth_request(server_address: String, user_id: &String, spoof_check_secret: &String){
    let request_builder=HTTPrequestBuilder::HTTPrequestBuilder::new (
        server_address,
        String::from("POST"),
        String::from(""),
        user_id,
        spoof_check_secret
    );
    let request=request_builder.build(user_id, spoof_check_secret);
    
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, irma_disclose_id])
}
