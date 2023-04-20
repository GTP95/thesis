use irma::{AttributeRequest, DisclosureRequestBuilder, IrmaClient, IrmaRequest, SessionData};
use qrcode::render::unicode;
use qrcode::QrCode;

pub(crate) struct IrmaSessionHandler {
    client: IrmaClient,
}

pub(crate) struct RequestResult {
    //My own type to return both the QR code and the session so I can check the session's status after the request
    pub(crate) qr: String,
    pub(crate) session: SessionData,
    pub(crate) client: IrmaClient,
}

impl IrmaSessionHandler {
    pub fn new(url: &str) -> IrmaSessionHandler {
        IrmaSessionHandler {
            client: IrmaClient::new(url).unwrap(),
        }
    }

    pub async fn disclose_id(&self, credential: String) -> RequestResult {
        let disclosure_request = self.build_disclosure_request(credential);

        let session = self
            .client
            .request(&disclosure_request)
            .await
            .expect("Failed to start session");

        // Encode the session pointer
        let sessionptr = serde_json::to_string(&session.session_ptr).unwrap();
        println!("Session pointer: {}", sessionptr);

        let result = RequestResult {
            qr: self.generate_qr(sessionptr),
            session: session,
            client: self.client.clone(),
        };
        return result;

        // Periodically poll if the session was successfully concluded
        //loop {
        //    match self.client.result(&session.token).await {
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

    fn build_disclosure_request(&self, credential: String) -> IrmaRequest {
        //Description of the attribute we want to request
        let attribute_request = AttributeRequest::Simple(credential);

        //disjunction of attributes we want to request. We need only one, so it will contain only one attribute and look rendundant, but it is required.
        let discon = vec![vec![attribute_request]];

        //Now create a disclosure request for the attributes we want
        let disclosure_request = DisclosureRequestBuilder::new().add_discon(discon).build();

        return disclosure_request;
    }
}
