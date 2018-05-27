use rocket::Route;
use rocket::request::FlashMessage;
use rocket_contrib::Template;
use users::User;
use series::PublicSeries;
use series::get_last_10_series;
use database::DbConn;
use structs::{Context, LoggedInContext};

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
    Template::render("home", &context)
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
    Template::render("index", &context)
}

#[get("/user_policy")]
fn user_policy(user: User) -> Template {
    let context = LoggedInContext {
        header: "Club Coding".to_string(),
        user: user,
    };
    Template::render("user_policy", &context)
}

#[get("/user_policy", rank = 2)]
fn user_policy_nologin() -> Template {
    let context = Context {
        header: "Club Coding".to_string(),
    };
    Template::render("user_policy_nologin", &context)
}

#[get("/privacy_policy")]
fn privacy_policy(user: User) -> Template {
    let context = LoggedInContext {
        header: "Club Coding".to_string(),
        user: user,
    };
    Template::render("privacy_policy", &context)
}

#[get("/privacy_policy", rank = 2)]
fn privacy_policy_nologin() -> Template {
    let context = Context {
        header: "Club Coding".to_string(),
    };
    Template::render("privacy_policy_nologin", &context)
}

pub fn endpoints() -> Vec<Route> {
    routes![
        index,
        index_nouser,
        user_policy,
        user_policy_nologin,
        privacy_policy,
        privacy_policy_nologin
    ]
}
