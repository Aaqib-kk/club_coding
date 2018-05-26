use rocket::Route;
use rocket_contrib::Template;
use rocket::response::{Flash, NamedFile, Redirect};
use club_coding::{create_new_user_series_access, create_new_user_view,
                  insert_new_users_stripe_charge};
use club_coding::models::{Series, UsersSeriesAccess, UsersStripeCustomer, UsersViews, Videos};
use users::User;
use std::io::{Error, ErrorKind};
use series::{get_video_watched, PublicVideo};
use rocket::request::FlashMessage;
use stripe::Source::Card;
use email::{EmailBody, PostmarkClient};
use database::DbConn;
use diesel::prelude::*;

pub fn get_videos(connection: &DbConn) -> Vec<Videos> {
    use club_coding::schema::videos::dsl::*;

    match videos
        .filter(published.eq(true))
        .filter(archived.eq(false))
        .order(created.asc())
        .load::<Videos>(&**connection)
    {
        Ok(vec_of_vids) => vec_of_vids,
        Err(_) => vec![],
    }
}

fn get_video_data_from_uuid(connection: &DbConn, uid: &String) -> Result<Videos, Error> {
    use club_coding::schema::videos::dsl::*;

    match videos
        .filter(uuid.eq(uid))
        .filter(published.eq(true))
        .filter(archived.eq(false))
        .limit(1)
        .load::<Videos>(&**connection)
    {
        Ok(result) => {
            if result.len() == 1 {
                Ok(result[0].clone())
            } else {
                Err(Error::new(ErrorKind::Other, "no video found"))
            }
        }
        Err(_) => Err(Error::new(ErrorKind::Other, "error loading videos")),
    }
}

fn get_series_title(connection: &DbConn, uid: Option<i64>) -> Option<String> {
    match uid {
        Some(uid) => {
            use club_coding::schema::series::dsl::*;

            match series
                .filter(id.eq(uid))
                .filter(published.eq(true))
                .filter(archived.eq(false))
                .limit(1)
                .load::<Series>(&**connection)
            {
                Ok(result) => {
                    if result.len() == 1 {
                        Some(result[0].title.clone())
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        }
        None => None,
    }
}

fn get_option_series(connection: &DbConn, uid: Option<i64>) -> Option<Series> {
    match uid {
        Some(sid) => {
            use club_coding::schema::series::dsl::*;

            match series.filter(id.eq(sid)).first(&**connection) {
                Ok(serie) => Some(serie),
                Err(_) => None,
            }
        }
        None => None,
    }
}

fn get_videos_of_series(connection: &DbConn, uid: i64, sid: i64) -> Vec<PublicVideo> {
    use club_coding::schema::videos::dsl::*;

    match videos
        .filter(series.eq(sid))
        .filter(published.eq(true))
        .filter(archived.eq(false))
        .order(episode_number.asc())
        .load::<Videos>(&**connection)
    {
        Ok(v_ideos) => {
            let mut to_return: Vec<PublicVideo> = vec![];
            for video in v_ideos {
                to_return.push(PublicVideo {
                    episode_number: video.episode_number,
                    uuid: video.uuid,
                    title: video.title,
                    description: video.description,
                    watched: get_video_watched(connection, uid, video.id),
                });
            }
            to_return
        }
        Err(_) => vec![],
    }
}

#[derive(Serialize)]
struct WatchContext<'a> {
    uuid: String,
    series_title: String,
    price: i32,
    title: String,
    description: String,
    user: &'a User,
    vimeo_id: String,
    videos: Vec<PublicVideo>,
    flash_name: String,
    flash_msg: String,
}

fn create_new_view(connection: &DbConn, vid: i64, uid: i64) -> Result<(), Error> {
    use club_coding::schema::users_views::dsl::*;

    match users_views
        .filter(user_id.eq(uid))
        .filter(video_id.eq(vid))
        .load::<UsersViews>(&**connection)
    {
        Ok(view) => {
            if view.len() == 0 {
                let _ = create_new_user_view(&connection, uid, vid)?;
            }
        }
        Err(_) => {}
    }
    Ok(())
}

fn user_has_bought(connection: &DbConn, sid: i64, uid: i64) -> bool {
    use club_coding::schema::users_series_access::dsl::*;

    match users_series_access
        .filter(user_id.eq(uid))
        .filter(series_id.eq(sid))
        .limit(1)
        .load::<UsersSeriesAccess>(&**connection)
    {
        Ok(series) => series.len() == 1,
        Err(_) => false,
    }
}

#[get("/watch/<uuid>")]
fn watch_as_user(
    conn: DbConn,
    user: User,
    flash: Option<FlashMessage>,
    uuid: String,
) -> Result<Template, Redirect> {
    match get_video_data_from_uuid(&conn, &uuid) {
        Ok(video) => {
            let videos: Vec<PublicVideo> = match video.series {
                Some(series_id) => get_videos_of_series(&conn, user.id, series_id),
                None => vec![],
            };
            let (name, msg) = match flash {
                Some(flash) => (flash.name().to_string(), flash.msg().to_string()),
                None => ("".to_string(), "".to_string()),
            };
            let mut context = WatchContext {
                uuid: video.uuid,
                series_title: "".to_string(),
                price: 0,
                title: video.title,
                description: video.description,
                user: &user,
                vimeo_id: video.vimeo_id,
                videos: videos,
                flash_name: name,
                flash_msg: msg,
            };
            match get_option_series(&conn, video.series) {
                Some(serie) => {
                    context.series_title = serie.title.clone();
                    context.price = serie.price;
                }
                None => {}
            }
            if video.membership_only {
                match video.series {
                    Some(series_id) => {
                        if user_has_bought(&conn, series_id, user.id) {
                            match create_new_view(&conn, video.id, user.id) {
                                Ok(_) => Ok(Template::render("watch_member", &context)),
                                Err(_) => Ok(Template::render("watch_member", &context)),
                            }
                        } else {
                            Ok(Template::render("watch_nomember", &context))
                        }
                    }
                    None => Ok(Template::render("watch_nomember", &context)),
                }
            } else {
                Ok(Template::render("watch_member", &context))
            }
        }
        Err(_video_not_found) => Err(Redirect::to("/")),
    }
}

fn get_videos_of_series_nologin(connection: &DbConn, sid: i64) -> Vec<PublicVideo> {
    use club_coding::schema::videos::dsl::*;

    match videos
        .filter(series.eq(sid))
        .filter(published.eq(true))
        .filter(archived.eq(false))
        .order(episode_number.asc())
        .load::<Videos>(&**connection)
    {
        Ok(v_ideos) => {
            let mut to_return: Vec<PublicVideo> = vec![];
            for video in v_ideos {
                to_return.push(PublicVideo {
                    episode_number: video.episode_number,
                    uuid: video.uuid,
                    title: video.title,
                    description: video.description,
                    watched: false,
                });
            }
            to_return
        }
        Err(_) => vec![],
    }
}

#[derive(Serialize)]
struct WatchNoUser {
    uuid: String,
    series_title: String,
    title: String,
    description: String,
    videos: Vec<PublicVideo>,
    flash_name: String,
    flash_msg: String,
}

#[get("/watch/<uuid>", rank = 2)]
fn watch_nouser(
    conn: DbConn,
    flash: Option<FlashMessage>,
    uuid: String,
) -> Result<Template, Redirect> {
    match get_video_data_from_uuid(&conn, &uuid) {
        Ok(video) => {
            let (name, msg) = match flash {
                Some(flash) => (flash.name().to_string(), flash.msg().to_string()),
                None => ("".to_string(), "".to_string()),
            };
            let videos: Vec<PublicVideo> = match video.series {
                Some(series_id) => get_videos_of_series_nologin(&conn, series_id),
                None => vec![],
            };
            let series_title = match get_series_title(&conn, video.series) {
                Some(title) => title,
                None => "".to_string(),
            };
            let context = WatchNoUser {
                uuid: video.uuid,
                series_title: series_title,
                title: video.title,
                description: video.description,
                videos: videos,
                flash_name: name,
                flash_msg: msg,
            };
            Ok(Template::render("watch_nologin", &context))
        }
        Err(_video_not_found) => Err(Redirect::to("/")),
    }
}

#[get("/thumbnail/<uuid>")]
fn thumbnail(uuid: String) -> Option<NamedFile> {
    match NamedFile::open(format!("thumbnails/{}.png", uuid)) {
        Ok(file) => Some(file),
        Err(_) => None,
    }
}

fn get_customer(connection: &DbConn, uid: i64) -> Option<UsersStripeCustomer> {
    use club_coding::schema::users_stripe_customer::dsl::*;

    match users_stripe_customer
        .filter(user_id.eq(uid))
        .limit(1)
        .load::<UsersStripeCustomer>(&**connection)
    {
        Ok(result) => {
            if result.len() == 1 {
                Some(result[0].clone())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

fn get_serie(connection: &DbConn, sid: i64) -> Option<Series> {
    use club_coding::schema::series::dsl::*;

    match series.filter(id.eq(sid)).first(&**connection) {
        Ok(serie) => Some(serie),
        Err(_) => None,
    }
}

fn send_bought_email(email: String) -> Result<(), Error> {
    let body = EmailBody {
        from: "axel@clubcoding.com".to_string(),
        to: email,
        subject: Some("Series bought!".to_string()),
        html_body: Some("<html><body>You recently bought a series.</body></html>".to_string()),
        cc: None,
        bcc: None,
        tag: None,
        text_body: None,
        reply_to: None,
        headers: None,
        track_opens: None,
        track_links: None,
    };
    let postmark_client = PostmarkClient::new("5f60334c-c829-45c6-aa34-08144c70559c");
    postmark_client.send_email(&body)?;
    Ok(())
}

fn charge(
    conn: &DbConn,
    series_id: i64,
    user: &User,
    stripe_customer: &UsersStripeCustomer,
) -> Result<(), Error> {
    let serie = match get_serie(&conn, series_id) {
        Some(serie) => serie,
        None => return Err(Error::new(ErrorKind::Other, "no serie")),
    };
    match stripe_customer.default_source {
        Some(ref customer_source) => {
            // Create the customer
            let client = stripe::Client::new("sk_test_cztFtKdeTEnlPLL6DpvkbjFf");

            let charge = match stripe::Charge::create(
                &client,
                stripe::ChargeParams {
                    amount: Some(serie.price as u64),
                    currency: Some(stripe::Currency::USD),
                    application_fee: None,
                    capture: None,
                    description: None,
                    destination: None,
                    fraud_details: None,
                    transfer_group: None,
                    on_behalf_of: None,
                    metadata: None,
                    receipt_email: None,
                    shipping: None,
                    customer: Some(stripe_customer.uuid.clone()),
                    source: Some(stripe::CustomerSource::Token(&customer_source)),
                    statement_descriptor: None,
                },
            ) {
                Ok(charge) => charge,
                Err(_) => return Err(Error::new(ErrorKind::Other, "couldn't create charge")),
            };
            let failure_code: Option<String> = match charge.failure_code {
                Some(code) => Some(code.to_string()),
                None => None,
            };
            let source_id = match charge.source {
                Card(card) => card.id,
            };
            let _ = insert_new_users_stripe_charge(
                &*conn,
                user.id,
                series_id,
                &charge.id,
                charge.amount as i32,
                charge.amount_refunded as i32,
                charge.balance_transaction,
                charge.captured,
                charge.created,
                charge.description,
                charge.destination,
                charge.dispute,
                failure_code,
                charge.failure_message,
                charge.livemode,
                charge.on_behalf_of,
                charge.order,
                charge.paid,
                charge.refunded,
                &source_id,
                charge.source_transfer,
                charge.statement_descriptor,
                &charge.status,
            )?;
            let _ = create_new_user_series_access(&*conn, user.id, series_id, true)?;
            let _ = send_bought_email(user.email.clone())?;
            Ok(())
        }
        None => Err(Error::new(ErrorKind::Other, "no customer_source")),
    }
}

#[get("/watch/<uuid>/buy")]
fn buy_serie(conn: DbConn, user: User, uuid: String) -> Result<Flash<Redirect>, Redirect> {
    match get_video_data_from_uuid(&conn, &uuid) {
        Ok(video) => match video.series {
            Some(series_id) => {
                if !user_has_bought(&conn, series_id, user.id) {
                    match get_customer(&conn, user.id) {
                        Some(stripe_customer) => {
                            match charge(&conn, series_id, &user, &stripe_customer) {
                                Ok(_) => Ok(Flash::success(
                                    Redirect::to(&format!("/watch/{}", uuid)),
                                    "Series unlocked! Congratulations!",
                                )),
                                Err(_) => Ok(Flash::error(
                                    Redirect::to(&format!("/watch/{}", uuid)),
                                    "An error occured, please try again later.",
                                )),
                            }
                        }
                        None => Err(Redirect::to(&format!("/card/add/{}", uuid))),
                    }
                } else {
                    Err(Redirect::to(&format!("/watch/{}", uuid)))
                }
            }
            None => Err(Redirect::to("/")),
        },
        Err(_video_not_found) => Err(Redirect::to("/")),
    }
}

pub fn endpoints() -> Vec<Route> {
    routes![thumbnail, watch_as_user, watch_nouser, buy_serie]
}
