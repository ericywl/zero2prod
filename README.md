# Rust Zero2Prod

We will be developing an Email Newsletter to learn about Rust development.

## User Stories

Blog visitor:
```
As a blog visitor,
I want to subscribe to the newsletter,
So that I can receive email updates when new content is published on the blog.
```

Blog author:
```
As the blog author,
I want to send an email to all my subscribers,
So that I can notify them when new content is published.
```

## Development

### Database

Before running anything, add the following environment variable to `.env` or export it.
```
DATABASE_URL=postgres://postgres:password@localhost:5432/newsletter
```

Start up the database Docker container.
```sh
$ ./scripts/init_db.sh
```

### Application

Then, run the following to start the application.
```sh
$ cargo run
```

You can also use `cargo watch` to automatically run on any changes.
```sh
$ cargo install cargo-watch
$ cargo watch -x check -x test -x run
```

## Deployment

The code can be setup so that pushes to `main` branch will trigger Continuous Deployment pipeline on DigitalOcean.

To create app deployment, use:
```sh
$ doctl apps create --spec spec.yaml
```

To view the list of apps, use:
```sh
$ doctl apps list
```