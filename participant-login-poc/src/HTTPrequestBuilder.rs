use http::{Request, Response};

pub struct HTTPrequestBuilder {
    pub url: String,
    pub method: String,
    pub body: String,
    uid_field_name: String,
    spoof_check_secret: String,
}

impl HTTPrequestBuilder {
    pub fn new(url: String, method: String, body: String, uid_field_name: String, spoof_check_secret: String) -> HTTPrequestBuilder {
        HTTPrequestBuilder {
            url,
            method,
            body,
            uid_field_name,
            spoof_check_secret
        }
    }

    pub fn build(&self, uid: &String, spoof_check_secret: &String) -> Request<&str> {
        let request = Request::builder()
            .method(self.method.as_str())
            .uri(self.url.as_str())
            .header("Shib-Spoof-Check", spoof_check_secret)
            .header(self.uid_field_name.clone(), uid)
            .body(self.body.as_str())
            .unwrap();
        request
    }
}

