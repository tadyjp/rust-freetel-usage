extern crate reqwest;
extern crate cookie;
extern crate select;
extern crate regex;
extern crate time;

use std::env;
use std::io::Read;
use reqwest::header;
use reqwest::{Response, RedirectPolicy};
use cookie::Cookie;

static LOGIN_FORM_URL: &str = "https://mypage.freetel.jp/login";
static USAGE_PAGE_URL: &str = "https://mypage.freetel.jp/SavingMode/saveModeDetail/";
static SESSION_COOKIE_NAME: &str = "CAKEPHP";
static INFLUXDB_URL: &str = "http://rusthelloworld_influxdb_1:8086/write?db=homelog";

fn get_session_cookie(resp: &Response) -> String {
    if let Some(set_cookies) = resp.headers().get::<header::SetCookie>() {
        let mut cookie_value = String::new();
        for set_cookie in &set_cookies.0 {
            println!("Set-Cookie: {:?}", set_cookie);
            let set_cookie = set_cookie.clone();
            let c = Cookie::parse(set_cookie).expect("Failed to parse cookie.");
            let (name, value) = c.name_value();

            println!("name: {:?}, value: {:?}", name, value);

            if name == SESSION_COOKIE_NAME {
                cookie_value = value.to_string();
            }
        }
        if cookie_value != "" {
            return cookie_value;
        }
        panic!("session_cookie not exists!");
    } else {
        panic!("session_cookie not exists!");
    }
}

fn get_usage_html(tel: &str, cookie_value: &str) -> String {
    let usage_page_url = &(USAGE_PAGE_URL.to_string() + tel);
    let client = reqwest::Client::new().unwrap();
    let mut cookie = header::Cookie::new();
    cookie.append(SESSION_COOKIE_NAME, cookie_value.to_string().clone());
    println!("[get_usage_html cookie] {:?}", cookie);
    let mut resp = client.get(usage_page_url).unwrap()
        .header(cookie)
        .send().unwrap();

    if !resp.status().is_success() {
        panic!("request failed!: {}", usage_page_url);
    }

    let mut content = String::new();
    resp.read_to_string(&mut content).unwrap();

    content
}

fn parse_usage(html: &str) -> (f32, f32) {
    use select::document::Document;
    use select::predicate::{Predicate, Attr, Class};
    use regex::Regex;

    let re_usage = Regex::new(r"([\d\.]+)GB").unwrap();

    let mut current_usage: f32 = 0.0;
    let mut usage_limit: f32 = 0.0;

    let document = Document::from(html);

    for node in document.find(Class("sim-usage").descendant((Attr("style", "font-size: x-large;")))).take(1) {
        let text = node.text();
        let caps = re_usage.captures(&text).unwrap();
        current_usage = caps.get(1).unwrap().as_str().parse::<f32>().unwrap();
    }

    for node in document.find(Class("sim-usage").descendant((Attr("style", "font-size: smaller;")))).take(1) {
        let text = node.text();
        let caps = re_usage.captures(&text).unwrap();
        usage_limit = caps.get(1).unwrap().as_str().parse::<f32>().unwrap();
    }

    (current_usage, usage_limit)
}

// curl -i -XPOST 'http://localhost:8086/write?db=homelog' --data 'cpu_load_short,host=server01,region=us-west value=0.64 1503063534888000000'
fn post_to_influxdb((current_usage, usage_limit): ((f32, f32))) {
    let timespec = time::get_time();
    let current_time_nano = [timespec.sec.to_string(), format!("{:09}", timespec.nsec.to_string())].join("");
    let data = [
        format!("freetel_usage value={} {}\n", current_usage, current_time_nano),
        format!("freetel_limit value={} {}\n", usage_limit, current_time_nano)
    ].join("\n");
    let client = reqwest::Client::new().unwrap();
    let resp = client.post(INFLUXDB_URL).unwrap()
        .body(data.clone())
        .send().unwrap();

    println!("resp: {:?}", resp);

    if !resp.status().is_success() {
        panic!("influxdb request failed! {}, {:?}, {:?}", INFLUXDB_URL, resp.status(), data);
    }
}

fn main() {
    let email = env::var("FREETEL_EMAIL").unwrap(); // TODO: error message...
    let password = env::var("FREETEL_PASSWORD").unwrap();
    let tel = env::var("FREETEL_TEL").unwrap();

    let params = [
        ("_method", "POST"),
        ("data[SimUser][userDesignationId]", &email),
        ("data[SimUser][password]", &password),
    ];

    let custom = RedirectPolicy::custom(|attempt| {
        println!("attempt.url(): {:?}, {:?}", attempt.url(), attempt.previous());
        attempt.stop()
    });

    let client = reqwest::Client::builder().unwrap()
        .redirect(custom)
        .build().unwrap();
    let resp = client.post(LOGIN_FORM_URL).unwrap()
        .form(&params).unwrap()
        .send().unwrap();

    println!("resp: {:?}", resp);

    if !resp.status().is_success() && !resp.status().is_redirection() {
        panic!("request failed! {}, {:?}", LOGIN_FORM_URL, resp.status());
    }

    // let mut content = String::new();
    // resp.read_to_string(&mut content).unwrap();
    // println!("{:?}", content);

    let session_cookie = get_session_cookie(&resp);
    println!("[session_cookie] {:?}", session_cookie);

    let usage_html = get_usage_html(&tel, &session_cookie);
    println!("[usage_html] {}", usage_html);

    // let usage_html = include_str!("sample.html");

    let usage = parse_usage(&usage_html);
    println!("[usage] {:?}", usage);

    post_to_influxdb(usage);
}
