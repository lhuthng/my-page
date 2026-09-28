// One person, three accounts.
//
// Everything here is written by the same author, but it is published under
// three accounts so the archive stays navigable: systems work, personal notes,
// and creative work. A byline cannot carry that on its own -- "The Architect"
// only means something to a visitor who already knows -- so every surface that
// shows an author says which persona it is, and the colours come from the
// `data-role` theming in `app.css` rather than from here.
//
// Keyed by profile slug, which is what the payloads carry (`author_slug` on
// cards, `username` on a post). Accounts that are not one of the three, such as
// a reader who signed up to comment, have no entry and keep the neutral theme.

export const PERSONAS = [
	{
		slug: 'admin',
		name: 'The Architect',
		role: 'systems',
		blurb: 'System architecture, technical implementation, and platform infrastructure.'
	},
	{
		slug: 'lhuthng',
		name: 'Thắng',
		role: 'personal',
		blurb: 'Direct experiences, unfiltered thoughts, and hands-on experiments.'
	},
	{
		slug: 'memo-fie',
		name: 'Memory Field',
		role: 'creative',
		blurb: 'Creative works, artistic side projects, and the juice that makes the work feel alive.'
	}
];

const BY_SLUG = new Map(PERSONAS.map((persona) => [persona.slug, persona]));

/** The persona behind a profile slug, or `null` when the account is not one. */
export const personaFor = (slug) => BY_SLUG.get(slug) ?? null;

/** The `data-role` value for a profile slug, or `undefined` for any other account. */
export const roleFor = (slug) => BY_SLUG.get(slug)?.role;
