use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();

    for msg in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", msg.content()).unwrap();
    }

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
        <head>
          <meta charset="utf-8" />
          <meta name="viewport" content="width=device-width, initial-scale=1" />
          <title>Login</title>
        </head>
        <html lang="en">
          <body>
            {}
            <form action="/login" method="post">
              <label
                >Username
                <input type="text" placeholder="Enter Username" name="username" />
              </label>
        
              <label
                >Password
                <input type="password" placeholder="Enter Password" name="password" />
              </label>
        
              <button type="submit">Login</button>
            </form>
          </body>
        </html>"#,
            error_html
        ))
}
