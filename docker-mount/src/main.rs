extern crate hyper;
extern crate hyper_openssl;
extern crate tokio_core;
extern crate futures;
extern crate cookie;

use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use hyper_openssl::HttpsConnector;
use tokio_core::reactor::Core;
use hyper::{Method, Request};
use hyper::header::{Headers, ContentLength, ContentType, SetCookie};

fn get_session_cookie(headers: &Headers) -> Option<String> {
    use cookie::Cookie;

    match headers.get::<SetCookie>() {
        Some(set_cookies) => {
            for set_cookie in &set_cookies.0 {
                println!("Set-Cookie: {:?}", set_cookie);
                let set_cookie = set_cookie.clone();
                let c = Cookie::parse(set_cookie).expect("Failed to parse cookie.");
                let (name, value) = c.name_value();

                if name == "__cfduid" {
                    return Some(value.to_string())
                }
            }
            None
        },
        None => {
            println!("No Set-Cookie found.");
            None
        },
    }
}


// 1. ログインフォームにPOSTする
// 2. 帰ってきたSet-Cookieを保存
// 3. 取得したいページにGET
// 4. HTML解析
fn main() {
    let mut core = Core::new().unwrap();

    let client = Client::configure()
        .connector(HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());

    let url = "https://requestb.in/1kc55ou1".parse().unwrap();

    let data = "abc=123&zzz=999";
    let mut req = Request::new(Method::Post, url);
    req.headers_mut().set(ContentType::form_url_encoded());
    req.headers_mut().set(ContentLength(data.len() as u64));
    req.set_body(data);


    let work = client.request(req).and_then(|res| {
        println!("Response: {}", res.status());

        // let raw = res.headers().get_raw("set-cookie").unwrap();
        if let Some(aaa) = get_session_cookie(res.headers()) {
            println!("aaa: {:?}", aaa);
        }

        res.body().for_each(|chunk| {
            io::stdout()
                .write_all(&chunk)
                .map_err(From::from)
        })
    });

    core.run(work).unwrap();
}
