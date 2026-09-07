-- Two independent tables that do not reference each other. With no single
-- root table, each table becomes its own struct, embedded as a field of the
-- generated `_Data` under a name matching the table. Each table must hold
-- exactly one row.
INSERT INTO product (name, price, in_stock) VALUES
  ('Widget', 999, true);

INSERT INTO customer (name, country, vip) VALUES
  ('Ada', 'US', true);
