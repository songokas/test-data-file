-- The root table (`product`) has more rows than `customer`. Root row 0 pairs
-- with customer row 0; root row 1 has no customer at its position, so a
-- `customer: Option<Customer>` parameter deserializes to `None`.
INSERT INTO product (name, price, in_stock) VALUES
  ('Widget', 999, true),
  ('Gadget', 1499, false);

INSERT INTO customer (name, country, vip) VALUES
  ('Ada', 'US', true);
