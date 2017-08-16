extern crate hyper;
extern crate hyper_openssl;
extern crate tokio_core;
extern crate futures;
extern crate cookie;

use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use hyper::client::HttpConnector;
use hyper_openssl::HttpsConnector;
use tokio_core::reactor::Core;
use hyper::{Method, Request};
use hyper::header::{Headers, ContentLength, ContentType, SetCookie};
use cookie::Cookie;


struct Freetel {
    core: Option<Core>,
    client: Option<Client<HttpsConnector<HttpConnector>>>,
    session_cookie: Option<String>,
    usage: Option<f64>,
}

impl Freetel {
    fn new() -> Freetel {
        Freetel {
            core: None,
            client: None,
            session_cookie: None,
            usage: None,
        }
    }

    fn fetch_usage() -> f64 {
        let mut freetel = Freetel::new();
        freetel.get_login_session();
        freetel.fetch_usage_html();
        freetel.parse_usage_html();
        freetel.usage.unwrap()
    }

    fn create_client(&mut self) {
        let core = Core::new().unwrap();

        let client = Client::configure()
            .connector(HttpsConnector::new(4, &core.handle()).unwrap())
            .build(&core.handle());

        self.client = Some(client);
        self.core = Some(core);
    }

    fn get_login_session(&mut self) {
        let url = "https://requestb.in/1kc55ou1".parse().unwrap();

        let data = "abc=123&zzz=999";
        let mut req = Request::new(Method::Post, url);
        req.headers_mut().set(ContentType::form_url_encoded());
        req.headers_mut().set(ContentLength(data.len() as u64));
        req.set_body(data);

        let work = self.client.unwrap().request(req).and_then(|res| {
            println!("Response: {}", res.status());

            match res.headers().get::<SetCookie>() {
                Some(set_cookies) => {
                    for set_cookie in &set_cookies.0 {
                        println!("Set-Cookie: {:?}", set_cookie);
                        let set_cookie = set_cookie.clone();
                        let c = Cookie::parse(set_cookie).expect("Failed to parse cookie.");
                        let (name, value) = c.name_value();

                        if name == "__cfduid" {
                            self.session_cookie = Some(value.to_string());
                        }
                    }
                },
                None => {
                    println!("No Set-Cookie found.");
                },
            }

            res.body().for_each(|chunk| {
                io::stdout()
                    .write_all(&chunk)
                    .map_err(From::from)
            })
        });

        self.core.unwrap().run(work).unwrap();
    }

    fn fetch_usage_html(&mut self) {

    }

    fn parse_usage_html(&mut self) {

    }
}


// 1. ログインフォームにPOSTする
// 2. 帰ってきたSet-Cookieを保存
// 3. 取得したいページにGET
// 4. HTML解析
fn main() {

    let aaa = Freetel::fetch_usage();
    println!("{:?}", aaa);

}
