use rocket::get;

#[get("/health")]
pub fn health() -> &'static str {
    "OK"
}
