-- Two independent tables with equal row counts: the first table (`product`)
-- is the root, and `customer` rows pair with product rows by position.
INSERT INTO product (name, price, in_stock) VALUES
  ('Widget', 999, true),
  ('Gadget', 1499, false);

INSERT INTO customer (name, country, vip) VALUES
  ('Ada', 'US', true),
  ('Bea', 'DE', false);
