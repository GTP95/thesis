use crate::irma_session_handler::IrmaSessionHandler;
use crate::token_generator::generate_participant_token;

mod token_generator;
mod irma_session_handler;

fn main() {
    let id = generate_participant_token();
    println!("id: {}", id);
    let irmaSessionHandler=IrmaSessionHandler::new("http://localhost:8088");
    irmaSessionHandler.issue_credential("irma-demo.PEP.id".to_string(), id);
}
