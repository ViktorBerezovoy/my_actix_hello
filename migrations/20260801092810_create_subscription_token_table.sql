CREATE TABLE subscription_tokens(
    subscription_token TEXT NOT NULL,
    subscriber_id uuid NOT NULL REFERENCES subscriptions (id),
    used_at timestamptz NULL,
    PRIMARY KEY (subscription_token)
);
