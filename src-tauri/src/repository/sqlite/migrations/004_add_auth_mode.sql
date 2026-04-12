-- Add auth_mode column to providers: 'bearer' (default) or 'anthropic_api_key'
ALTER TABLE providers ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'bearer';
