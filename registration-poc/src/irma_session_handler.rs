//Code adapted from https://github.com/tweedegolf/irmars/blob/main/examples/issuance.rs

use std::future::Future;
use std::time::Duration;
use irma::{CredentialBuilder, IrmaClient, IrmaRequest, IssuanceRequestBuilder};
use tokio::time::sleep;

pub(crate) struct IrmaSessionHandler {
    client: IrmaClient
}

impl IrmaSessionHandler{
    pub(crate) fn new(url: &str) ->IrmaSessionHandler{
        IrmaSessionHandler{
            client: IrmaClient::new(url).unwrap()
        }
    }

    fn build_request(&self, credential: String, value: String) -> IrmaRequest {
        let request = IssuanceRequestBuilder::new()
            .add_credential(
                CredentialBuilder::new(credential.into())
                    .attribute("id".into(), value.into())//id hardcoded for now, not even sure about what's supposed to be in there TODO
                    .build(),
            )
            .build();
        return request;
    }

    fn generate_qr(&self, sessionptr: String) -> String {  //TODO: make this generate an image instead of a string, so that I can embed it into the web page
        // Render a qr
        let qr = qrcode::QrCode::new(sessionptr)
            .unwrap()
            .render::<char>()
            .quiet_zone(false)
            .module_dimensions(2, 1)
            .build();
        return qr;
    }

    pub async fn issue_credential(&self, credential: String, value: String) -> String  {
        let request=self.build_request(credential, value);

        // Start the session
        let session = self.client
            .request(&request)
            .await
            .expect("Failed to start session");

        // Encode the session pointer
        let sessionptr = serde_json::to_string(&session.session_ptr).unwrap();
        return self.generate_qr(sessionptr);

        // Periodically poll if the session was successfully concluded
       //loop {
       //    match self.client.result(&session.token).await {
       //        Ok(_) => break,
       //        Err(irma::Error::SessionNotFinished(_)) => {}
       //        Err(v) => panic!("{}", v),
       //    }
       //
       //    sleep(Duration::from_secs(2)).await;
       //}
       //
       //println!("Issuance done");
    }



}

