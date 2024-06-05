use lazy_static::lazy_static;
use tera::{Context, Tera};
use uuid::Uuid;

use crate::domain::{Name, Url};

lazy_static! {
    static ref TEMPLATES: Tera = {
        match Tera::new("src/templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Tera parsing error: {}", e);
                ::std::process::exit(1);
            }
        }
    };
}

/// Renders index page with either success message or error message or none if both are `None`.
/// Success message takes precedence.
pub fn index_html(
    user_id: Option<Uuid>,
    success_msg: Option<String>,
    error_msg: Option<String>,
) -> String {
    let mut context = Context::new();
    if let Some(user_id) = user_id {
        context.insert("user_id", &user_id.to_string());
    }

    if let Some(msg) = success_msg {
        context.insert("success_msg", &msg);
    } else if let Some(msg) = error_msg {
        context.insert("error_msg", &msg);
    }

    TEMPLATES.render("index.html", &context).unwrap()
}

/// Renders confirmation email with name and confirmation link.
pub fn confirmation_email_html(name: &Name, link: &Url) -> String {
    let mut context = Context::new();
    context.insert("name", name.as_ref());
    context.insert("confirmation_link", link.as_str());

    TEMPLATES
        .render("confirmation_email.html", &context)
        .unwrap()
}

/// Renders login page with optional error message.
pub fn login_html(success_msg: Option<String>, error_msg: Option<String>) -> String {
    let mut context = Context::new();
    if let Some(msg) = success_msg {
        context.insert("success_msg", &msg);
    } else if let Some(msg) = error_msg {
        context.insert("error_msg", &msg);
    }

    TEMPLATES.render("login.html", &context).unwrap()
}

/// Renders admin dashboard with username.
pub fn admin_dashboard_html(username: &Name) -> String {
    let mut context = Context::new();
    context.insert("username", username.as_ref());

    TEMPLATES.render("admin/dashboard.html", &context).unwrap()
}

/// Renders admin change password form with optional error message.
pub fn admin_change_password_html(
    success_msg: Option<String>,
    error_msg: Option<String>,
) -> String {
    let mut context = Context::new();
    if let Some(msg) = success_msg {
        context.insert("success_msg", &msg);
    } else if let Some(msg) = error_msg {
        context.insert("error_msg", &msg);
    }

    TEMPLATES
        .render("admin/change_password.html", &context)
        .unwrap()
}

/// Renders admin publish newsletter form with optional error message.
pub fn admin_newsletter_html(
    success_msg: Option<String>,
    error_msg: Option<String>,
    idempotency_key: String,
) -> String {
    let mut context = Context::new();
    context.insert("idempotency_key", &idempotency_key);
    if let Some(msg) = success_msg {
        context.insert("success_msg", &msg);
    } else if let Some(msg) = error_msg {
        context.insert("error_msg", &msg);
    }

    TEMPLATES.render("admin/newsletter.html", &context).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn index_template_works() {
        index_html(Some(Uuid::new_v4()), None, Some("something".into()));
    }

    #[test]
    fn confirmation_email_template_works() {
        let name = Name::parse("Mamamia").unwrap();
        let link = Url::parse("https://hecomundo-bleach.com").unwrap();
        confirmation_email_html(&name, &link);
    }

    #[test]
    fn login_template_works() {
        login_html(None, Some("something".into()));
    }

    #[test]
    fn admin_dashboard_template_works() {
        let name = Name::parse("Capoo").unwrap();
        admin_dashboard_html(&name);
    }

    #[test]
    fn admin_change_password_template_works() {
        admin_change_password_html(Some("good".into()), Some("something".into()));
    }

    #[test]
    fn admin_newsletter_template_works() {
        admin_newsletter_html(Some("yeah".into()), None, Uuid::new_v4().to_string());
    }
}
