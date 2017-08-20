use std::env;
use std::io::Read;
use reqwest::{self, header, RedirectPolicy};
use cookie::Cookie;
use time;

const LOGIN_FORM_URL: &str = "https://mypage.freetel.jp/login";
const USAGE_PAGE_URL: &str = "https://mypage.freetel.jp/SavingMode/saveModeDetail/";
const SESSION_COOKIE_NAME: &str = "CAKEPHP";
const INFLUXDB_URL: &str = "http://rusthelloworld_influxdb_1:8086/write?db=homelog";
const USER_AGENT: &str = "Rust/reqwest freetel_usage";


// freetel マイページのログインセッションを取得
fn get_session_cookie(email: &str, password: &str) -> String {
    let params = [
        ("_method", "POST"),
        ("data[SimUser][userDesignationId]", &email),
        ("data[SimUser][password]", &password),
    ];

    // カスタム RedirectPolicy
    // ログインリクエスト後に別のページに遷移するのを防ぐため `stop()` する
    let custom = RedirectPolicy::custom(|attempt| {
        attempt.stop()
    });

    // HTTP クライアントの生成
    let client = reqwest::Client::builder().unwrap()
        .redirect(custom)
        .build().unwrap();

    // HTTP Post リクエスト実行
    let resp = client.post(LOGIN_FORM_URL).unwrap()
        .header(header::UserAgent::new(USER_AGENT))
        .form(&params).unwrap()
        .send().unwrap();

    // 2xx か 3xx 以外の場合にはエラーとする
    if !resp.status().is_success() && !resp.status().is_redirection() {
        panic!("request failed! {}, {:?}", LOGIN_FORM_URL, resp);
    }

    // Set-Cookie ヘッダを取得
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
        panic!("Set-Cookie '{}' does not exist!", SESSION_COOKIE_NAME);
    } else {
        panic!("Set-Cookie '{}' does not exist!", SESSION_COOKIE_NAME);
    }
}

// 利用状況の HTML を取得
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

// 利用状況のギガ数を取得
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

// InfluxDB にギガ数を登録
// curl -i -XPOST 'http://INFLUXDB_URL' --data 'freetel_usage value=0.64 1503063534888000000'
fn post_to_influxdb((current_usage, usage_limit): ((f32, f32))) {
    let timespec = time::get_time();
    let current_time_nano = [timespec.sec.to_string(), format!("{:09}", timespec.nsec.to_string())].join("");
    let data = [
        format!("freetel_usage value={} {}", current_usage, current_time_nano),
        format!("freetel_limit value={} {}", usage_limit, current_time_nano)
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


// 1. freetel マイページのログインセッションを取得
// 2. 利用状況のギガ数を取得
// 3. InfluxDB にギガ数を登録
pub fn fetch_usage() {
    let email = env::var("FREETEL_EMAIL").expect("env 'FREETEL_EMAIL' not found"); // TODO: error message...
    let password = env::var("FREETEL_PASSWORD").expect("env 'FREETEL_PASSWORD' not found");
    let tel = env::var("FREETEL_TEL").expect("env 'FREETEL_TEL' not found");

    // HTTP レスポンスからセッションCookieを取得
    let session_cookie = get_session_cookie(&email, &password);
    println!("session_cookie {:?}\n", session_cookie);

    // セッション Cookie を使って利用状況ページの HTML を取得
    let usage_html = get_usage_html(&tel, &session_cookie);

    // HTML からギガ数を取得
    let usage = parse_usage(&usage_html);
    println!("usage {:?}\n", usage);

    // InfluxDB にギガ数を登録
    post_to_influxdb(usage);
}
