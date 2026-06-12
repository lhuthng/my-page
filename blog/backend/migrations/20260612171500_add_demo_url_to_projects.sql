-- Migration: Add demo_url column to projects table
ALTER TABLE projects ADD COLUMN demo_url TEXT;

