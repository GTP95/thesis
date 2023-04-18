use rocket::Rocket;
use crate::irma_session_handler::IrmaSessionHandler;
use crate::token_generator::generate_participant_token;
#[macro_use] extern crate rocket;

mod token_generator;
mod irma_session_handler;


#[get("/")]
async fn index() -> String {
    let irmaSessionHandler=IrmaSessionHandler::new("http://localhost:8088");
    let id = generate_participant_token();
    let qr=irmaSessionHandler.issue_credential("irma-demo.PEP.id".to_string(), id).await;
    return qr;
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}









