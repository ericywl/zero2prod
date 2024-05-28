use lazy_static::lazy_static;
use tera::{Context, Tera};

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
pub fn index_html(success_msg: Option<String>, error_msg: Option<String>) -> String {
    let mut context = Context::new();
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
pub fn login_page_html(error_msg: Option<String>) -> String {
    let mut context = Context::new();
    if let Some(msg) = error_msg {
        context.insert("error_msg", &msg);
    }

    TEMPLATES.render("login.html", &context).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn index_template_works() {
        let mut context = Context::new();
        context.insert("error_msg", "something");

        assert!(
            TEMPLATES.render("index.html", &context).is_ok(),
            "Failed to render template."
        )
    }

    #[test]
    fn confirmation_email_template_works() {
        let mut context = Context::new();
        context.insert("name", "Mamamia");
        context.insert("confirmation_link", "hecomundo@bleach.com");

        assert!(
            TEMPLATES
                .render("confirmation_email.html", &context)
                .is_ok(),
            "Failed to render template."
        );
    }

    #[test]
    fn login_template_works() {
        let mut context = Context::new();
        context.insert("error_msg", "something");

        assert!(
            TEMPLATES.render("login.html", &context).is_ok(),
            "Failed to render template."
        )
    }
}
