ALTER TABLE task_route_preferences
ADD COLUMN delegate_agent_kind TEXT NOT NULL DEFAULT 'auto';
