use actix_web::HttpResponse;

/// Route that we can ping to make sure that the server is running
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
