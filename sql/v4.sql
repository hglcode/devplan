-- v3 -> v4: updated_at invariant moves to storage layer

CREATE TRIGGER IF NOT EXISTS trg_tasks_updated_at
AFTER UPDATE ON tasks
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at   -- 跳过显式带 updated_at 的写(兼容期)
BEGIN
    UPDATE tasks SET updated_at = datetime('now','localtime') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_plans_updated_at
AFTER UPDATE ON plans
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE plans SET updated_at = datetime('now','localtime') WHERE id = NEW.id;
END;
