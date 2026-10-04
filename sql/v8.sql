-- v7 -> v8: drop time_spent — the last dead column
-- (time tracking is another product's philosophy; dp is lightweight recording.
--  Schema reaches zero dead columns: every column has a writer, a reader, semantics.)
ALTER TABLE tasks DROP COLUMN time_spent;
