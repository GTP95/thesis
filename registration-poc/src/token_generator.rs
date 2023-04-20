use std::ops::Add;
use uuid::Uuid;

///Generates a unique token for a participant in the form of "participant:UUIDv4".
pub(crate) fn generate_participant_token() -> String {
    let id = Uuid::new_v4();
    let id: String = String::from("participant:").add(&id.to_string());
    return id;
}
