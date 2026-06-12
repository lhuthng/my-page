-- Migration: Populate demo_url column for existing projects
UPDATE projects SET demo_url = 'project-demos/' || id || '/' || demo_entry_path WHERE demo_url IS NULL;
