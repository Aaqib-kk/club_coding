use rocket::Route;
use rocket::request::FlashMessage;
use rocket_contrib::{Json, Template};
use users::User;
use series::PublicSeries;
use series::get_last_10_series;
use database::DbConn;
use structs::{Context, LoggedInContext};
use rocket::response::NamedFile;
use club_coding::create_new_newsletter_subscriber;

#[derive(Serialize)]
struct IndexLoggedInContext {
    header: String,
    user: User,
    flash_name: String,
    flash_msg: String,
    series: Vec<PublicSeries>,
}

#[derive(Serialize)]
struct IndexContext {
    header: String,
    flash_name: String,
    flash_msg: String,
    series: Vec<PublicSeries>,
}

#[get("/")]
fn index(conn: DbConn, user: User, flash: Option<FlashMessage>) -> Template {
    let (name, msg) = match flash {
        Some(flash) => (flash.name().to_string(), flash.msg().to_string()),
        None => ("".to_string(), "".to_string()),
    };
    let context = IndexLoggedInContext {
        header: "Club Coding".to_string(),
        user: user,
        flash_name: name,
        flash_msg: msg,
        series: get_last_10_series(&conn),
    };
    Template::render("pages/home", &context)
}

#[get("/", rank = 2)]
fn index_nouser(conn: DbConn, flash: Option<FlashMessage>) -> Template {
    let (name, msg) = match flash {
        Some(flash) => (flash.name().to_string(), flash.msg().to_string()),
        None => ("".to_string(), "".to_string()),
    };
    let context = IndexContext {
        header: "Club Coding".to_string(),
        flash_name: name,
        flash_msg: msg,
        series: get_last_10_series(&conn),
    };
    Template::render("pages/index", &context)
}

#[derive(Deserialize, Serialize)]
pub struct NewSubscriber {
    email: String,
}

#[post("/subscribe", format = "application/json", data = "<data>")]
fn subscribe(conn: DbConn, data: Json<NewSubscriber>) -> Result<(), ()> {
    match create_new_newsletter_subscriber(&*conn, &data.0.email) {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

#[get("/terms_of_service")]
fn terms_of_service(user: User) -> Template {
    let context = LoggedInContext {
        header: "Club Coding".to_string(),
        user: user,
    };
    Template::render("pages/terms_of_service", &context)
}

#[get("/terms_of_service", rank = 2)]
fn terms_of_service_nologin() -> Template {
    let context = Context {
        header: "Club Coding".to_string(),
    };
    Template::render("pages/terms_of_service_nologin", &context)
}

#[get("/cookie_policy")]
fn cookie_policy(user: User) -> Template {
    let context = LoggedInContext {
        header: "Club Coding".to_string(),
        user: user,
    };
    Template::render("pages/cookie_policy", &context)
}

#[get("/cookie_policy", rank = 2)]
fn cookie_policy_nologin() -> Template {
    let context = Context {
        header: "Club Coding".to_string(),
    };
    Template::render("pages/cookie_policy_nologin", &context)
}

#[get("/privacy_policy")]
fn privacy_policy(user: User) -> Template {
    let context = LoggedInContext {
        header: "Club Coding".to_string(),
        user: user,
    };
    Template::render("pages/privacy_policy", &context)
}

#[get("/privacy_policy", rank = 2)]
fn privacy_policy_nologin() -> Template {
    let context = Context {
        header: "Club Coding".to_string(),
    };
    Template::render("pages/privacy_policy_nologin", &context)
}

#[get("/thumbnail/<uuid>")]
fn thumbnail(uuid: String) -> Option<NamedFile> {
    match NamedFile::open(format!("thumbnails/{}.png", uuid)) {
        Ok(file) => Some(file),
        Err(_) => None,
    }
}

#[get("/img/<uuid>")]
fn images(uuid: String) -> Option<NamedFile> {
    match NamedFile::open(format!("images/{}", uuid)) {
        Ok(file) => Some(file),
        Err(_) => None,
    }
}

pub fn endpoints() -> Vec<Route> {
    routes![
        index,
        index_nouser,
        subscribe,
        terms_of_service,
        terms_of_service_nologin,
        cookie_policy,
        cookie_policy_nologin,
        privacy_policy,
        privacy_policy_nologin,
        thumbnail,
        images
    ]
}
