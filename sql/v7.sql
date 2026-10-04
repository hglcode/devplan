 -- v6 -> v7: status_note — note on any status transition
 -- (symmetry with resolution_note; one column, all transitions share it)
 ALTER TABLE tasks ADD COLUMN status_note TEXT NOT NULL DEFAULT '';
