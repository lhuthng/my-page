-- A manual dashboard send can reference the same post_id as the auto-campaign
-- fired on publish, and previously that one-off send consumed the per-post
-- unique slot — silently blocking every later publish from notifying
-- subscribers. Scope the double-send guard to publish-triggered rows only, and
-- record each campaign's origin so history still shows what it was for.
ALTER TABLE newsletter_campaigns
    ADD COLUMN source TEXT NOT NULL DEFAULT 'publish';

DROP INDEX idx_newsletter_campaigns_post_id_unique;

CREATE UNIQUE INDEX idx_newsletter_campaigns_post_id_unique
ON newsletter_campaigns (post_id)
WHERE post_id IS NOT NULL AND source = 'publish';
