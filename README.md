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
Before running anything, start up the database Docker container.
```sh
$ DATABASE_URL=postgres://postgres:password@localhost:5432/newsletter ./scripts/init_db.sh
```

Then, run the following to start the application.
```sh
$ cargo run
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