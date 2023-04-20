mod irma_session_handler;

#[macro_use]
extern crate rocket;

use crate::irma_session_handler::IrmaSessionHandler;
use rocket::response::stream::TextStream;
use rocket::tokio::time::{interval, sleep, Duration};
use std::ops::Add;

#[get("/")]
async fn index() -> String {
    String::from("AUTH")
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

/// Produce an infinite series of `"hello"`s, one per second.
#[get("/infinite-hellos")]
fn hello() -> TextStream![&'static str] {
    TextStream! {
        let mut interval = interval(Duration::from_secs(1));
        loop {
            yield "hello";
            interval.tick().await;
        }
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, irma_disclose_id, hello])
    // rocket::build().mount("/disclose", routes![irma_disclose_id])
}
