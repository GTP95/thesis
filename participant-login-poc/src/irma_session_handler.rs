use irma::{AttributeRequest, DisclosureRequestBuilder, Error, IrmaClient, IrmaRequest, SessionData, SessionResult, SessionToken};
use qrcode::render::svg;
use qrcode::QrCode;

pub(crate) struct IrmaSessionHandler {
    client: IrmaClient,
}

///My own type to return both the QR code and the session so I can check the session's status after the request. Contains also a reference to the client.
pub(crate) struct RequestResult {
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
    }

    /// Queries the server for the session's status.
    /// # Arguments
    /// * `session_token` - The session token returned by the server when the session was started
    pub async fn get_status(&self, session_token: &SessionToken) -> Result<SessionResult, Error> {
            let result=self.client.result(session_token).await;
            return result;
    }

    /// Renders a QR code containing the IRMA session pointer
    fn generate_qr(&self, sessionptr: String) -> String {
        let code = QrCode::new(sessionptr).unwrap();
        let image = code.render()
            .min_dimensions(250, 250)
            .dark_color(svg::Color("#800000"))
            .light_color(svg::Color("#ffff80"))
            .build();

        //Remove the first part, so that I can use the result directly into HTML code
        let image=image.replace("<?xml version=\"1.0\" standalone=\"yes\"?>", "");
        return image;
    }

    /// Builds a disclosure request for the specified credential
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
