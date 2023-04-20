use crate::irma_session_handler::IrmaSessionHandler;
use crate::token_generator::generate_participant_token;
use rocket::response::stream::TextStream;
use rocket::tokio::time::{sleep, Duration};
use std::ops::Add;
#[macro_use]
extern crate rocket;

mod irma_session_handler;
mod token_generator;

#[get("/")]
async fn index() -> TextStream![String] {
    let irma_session_handler = IrmaSessionHandler::new("http://localhost:8088"); //eduroam ip: 145.116.169.159
    let id = generate_participant_token();
    let request_result = irma_session_handler
        .issue_credential("irma-demo.PEP.id".to_string(), &id)
        .await;

    TextStream! {
        yield request_result.qr.add("\n\nCredential to be added to auth server: ").add(&id);   //Shows QR code and the credential contained therein

        // Periodically poll if the session was successfully concluded
        loop {
            match request_result.client.result(&request_result.session.token).await {
                Ok(_) => break,
                Err(irma::Error::SessionNotFinished(_)) => {}
                Err(v) => panic!("{}", v),
            }

            sleep(Duration::from_secs(2)).await;
        }

        yield String::from("\nIssuance done");
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
