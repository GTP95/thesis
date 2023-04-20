//Code adapted from https://github.com/tweedegolf/irmars/blob/main/examples/issuance.rs

use irma::{CredentialBuilder, IrmaClient, IrmaRequest, IssuanceRequestBuilder, SessionData};
use qrcode::render::unicode;
use qrcode::QrCode;

pub(crate) struct IrmaSessionHandler {
    client: IrmaClient,
}

pub(crate) struct IssueCredentialRequestResult {
    //My own type to return both the QR code and the session so I can check the session's status after the request
    pub(crate) qr: String,
    pub(crate) session: SessionData,
    pub(crate) client: IrmaClient,
}

impl IrmaSessionHandler {
    pub(crate) fn new(url: &str) -> IrmaSessionHandler {
        IrmaSessionHandler {
            client: IrmaClient::new(url).unwrap(),
        }
    }

    pub async fn issue_credential(
        &self,
        credential: String,
        value: &String,
    ) -> IssueCredentialRequestResult {
        let request = self.build_request(credential, value.to_string());

        // Start the session
        let session = self
            .client
            .request(&request)
            .await
            .expect("Failed to start session");

        // Encode the session pointer
        let sessionptr = serde_json::to_string(&session.session_ptr).unwrap();
        println!("Session pointer: {}", sessionptr);

        let result = IssueCredentialRequestResult {
            qr: self.generate_qr(sessionptr),
            session: session,
            client: self.client.clone(),
        };

        return result;
    }

    fn build_request(&self, credential: String, value: String) -> IrmaRequest {
        let request = IssuanceRequestBuilder::new()
            .add_credential(
                CredentialBuilder::new(credential.into())
                    .attribute("id".into(), value.into())
                    .build(),
            )
            .build();
        return request;
    }

    fn generate_qr(&self, sessionptr: String) -> String {
        //TODO: make this generate an image instead of a string, so that I can embed it into the web page
        // Render a qr
        let code = QrCode::new(sessionptr).unwrap();
        let image = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();
        return image;
    }
}
