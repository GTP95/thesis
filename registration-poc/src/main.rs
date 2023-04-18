use std::ops::Add;
use rocket::Rocket;
use crate::irma_session_handler::IrmaSessionHandler;
use crate::token_generator::generate_participant_token;
#[macro_use] extern crate rocket;

mod token_generator;
mod irma_session_handler;


#[get("/")]
async fn index() -> String {
    let irma_session_handler=IrmaSessionHandler::new("http://localhost:8088");   //eduroam ip: 145.116.169.159
    let id = generate_participant_token();
    let qr=irma_session_handler.issue_credential("irma-demo.PEP.id".to_string(), &id).await;
    return qr.add("\n\nCredential to be added to auth server: ").add(&id);
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}









