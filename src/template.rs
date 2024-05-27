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

pub fn confirmation_email_html(name: &Name, link: &Url) -> String {
    let mut context = Context::new();
    context.insert("name", name.as_ref());
    context.insert("confirmation_link", link.as_str());

    TEMPLATES
        .render("confirmation_email.html", &context)
        .unwrap()
}

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
