-- `customer` has more rows than the root table (`product`). Positions beyond
-- the last root row (here customer row 2) are never referenced.
INSERT INTO product (name, price, in_stock) VALUES
  ('Widget', 999, true),
  ('Gadget', 1499, false);

INSERT INTO customer (name, country, vip) VALUES
  ('Ada', 'US', true),
  ('Bea', 'DE', false),
  ('Cy', 'FR', true);
