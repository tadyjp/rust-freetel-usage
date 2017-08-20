use std::env;
use reqwest::{self, header, Response, RedirectPolicy};
use cookie::Cookie;
use std::io::Read;
use std::vec::Vec;

static USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.101 Safari/537.36";

fn update_set_cookies(cookie_store: &mut header::Cookie, resp: &Response) {
    if let Some(_set_cookies) = resp.headers().get::<header::SetCookie>() {
        for set_cookie in &_set_cookies.0 {
            println!("Set-Cookie: {:?}\n", set_cookie);
            let set_cookie = set_cookie.clone();
            let c = Cookie::parse(set_cookie).expect("Failed to parse cookie.");
            let (name, value) = c.name_value();

            cookie_store.set(name.to_string(), value.to_string());
        }
    }

    // println!("update_set_cookies: {:?}\n", cookie_store);
}

fn get_location(resp: &Response) -> String {
    if let Some(location) = resp.headers().get::<header::Location>() {
        return location.to_string();
    } else {
        panic!("No Location header found.")
    }
}

fn parse_ylogin_ticket(html: &str) -> String {
    use select::document::Document;
    use select::predicate::{Predicate, Attr};

    let document = Document::from(html);

    for node in document.find((Attr("name", "yloginForm").descendant(Attr("name", "ticket")))).take(1) {
        return node.attr("value").unwrap().to_string();
    }
    panic!("ticket value not found.");
}

fn parse_ylogin_hidden_fields(html: &str) -> Vec<(String, String)> {
    use select::document::Document;
    use select::predicate::{Predicate, Attr};

    let document = Document::from(html);
    let mut hidden_fields = vec![];

    for node in document.find((Attr("name", "login_form").descendant(Attr("type", "hidden")))) {
        hidden_fields.push((node.attr("name").unwrap().to_string(), node.attr("value").unwrap().to_string()));
    }
    hidden_fields
}

fn crawl(cookie_store: &mut header::Cookie, method: reqwest::Method, url: &str, params: Vec<(String, String)>) -> (String, String) {
    let custom = RedirectPolicy::custom(|attempt| {
        // println!("[RedirectPolicy] attempt.url(): {:?}, {:?}\n", attempt.url(), attempt.previous());
        attempt.stop()
    });

    let client = reqwest::Client::builder().unwrap()
        .redirect(custom)
        .build().unwrap();

    let mut next_url = url.to_string();

    let mut done = false;
    let mut redirect_count = 0;
    let mut content = String::new();

    while !done {
        if redirect_count >= 10 {
            panic!("redirect loop detected.");
        }

        let mut resp = match method {
            reqwest::Method::Get => {
                client.get(&next_url).unwrap()
                    .header(cookie_store.clone())
                    .header(header::UserAgent::new(USER_AGENT))
                    .send().unwrap()
            },
            reqwest::Method::Post => {
                client.post(&next_url).unwrap()
                    .header(cookie_store.clone())
                    .header(header::UserAgent::new(USER_AGENT))
                    .form(&params).unwrap()
                    .send().unwrap()
            },
            _ => {
                panic!("unexpected method: {:?}", method);
            }
        };

        println!("resp: {:?}\n", resp);

        if resp.status().is_success() {
            update_set_cookies(cookie_store, &resp);
            resp.read_to_string(&mut content).unwrap();
            done = true;
        } else if resp.status().is_redirection() {
            redirect_count += 1;
            update_set_cookies(cookie_store, &resp);
            next_url = get_location(&resp);
            continue;
        } else {
            panic!("request failed! {:?}", resp.status());
        }
    }

    (next_url, content)
}

pub fn fetch_usage() {

    let mut cookie_store = header::Cookie::new();

    let (next_url, html) = crawl(&mut cookie_store, reqwest::Method::Get, "https://my.softbank.jp/msb/d/webLink/doSend/MSB020063", vec![]);

    println!("[next_url 1]: {:?}\n\ncookie: {:?}\n", next_url, cookie_store);

    let ticket = parse_ylogin_ticket(&html);

    let (next_url, html) = crawl(&mut cookie_store, reqwest::Method::Post, "https://id.my.softbank.jp/sbid_auth/type1/2.0/y_login.php", vec![("ticket".to_string(), ticket)]);

    let mut login_fields: Vec<(String, String)> = vec![];
    for (name, value) in parse_ylogin_hidden_fields(&html) {
        login_fields.push((name.clone(), value.clone()));
    }
    login_fields.push(("login".to_string(), env::var("YAHOO_EMAIL").unwrap().to_string()));
    login_fields.push(("passwd".to_string(), env::var("YAHOO_PASSWORD").unwrap().to_string()));

    let (next_url, html) = crawl(&mut cookie_store, reqwest::Method::Post, "https://login.yahoo.co.jp/config/login", login_fields);

    println!("[next_url 2]: {:?}\n\ncookie: {:?}\n\n{:?}\n", next_url, cookie_store, html);
}
