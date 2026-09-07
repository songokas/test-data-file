INSERT INTO account (id, username, is_active) VALUES
  (1, 'ada', true),
  (2, 'lin', true),
  (3, 'sam', false);

INSERT INTO profile (account_id, country, plan) VALUES
  (1, 'US', 'pro'),
  (2, 'GB', 'free'),
  (3, 'DE', 'pro');
