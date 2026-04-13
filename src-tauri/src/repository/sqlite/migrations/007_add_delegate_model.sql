ALTER TABLE task_route_preferences
ADD COLUMN delegate_model TEXT;

UPDATE task_route_preferences
SET delegate_model = CASE
    WHEN target_member IS NOT NULL AND trim(target_member) <> '' THEN trim(target_member)
    ELSE NULL
END
WHERE delegate_model IS NULL;
