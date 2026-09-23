/**
 * The one audiobook that keeps playing while the reader browses the rest of
 * the site.
 *
 * The engine and its `<audio>` element live here rather than inside a page
 * component. A media element does not have to be in the document to play — that
 * is how `new Audio()` works — so handing the element to this module lets the
 * audio outlive the navigation that tears down the page which started it. The
 * mini player then drives the very same engine.
 */
import { browser } from '$app/environment';
import { AudiobookPlayer } from './AudiobookPlayer.svelte.js';

class AudiobookSession {
	/** `{ engine, book }` once a book has been taken over, otherwise `null`. */
	current = $state(null);
	/**
	 * Id of the book whose full player is on screen. The mini player steps
	 * aside while the reader is already looking at that book.
	 */
	onScreenId = $state(null);

	#engine = null;
	#book = null;

	/**
	 * Whether the mini player is actually on screen: something is playing and
	 * the reader is not already looking at that book's full player. Other
	 * floating furniture (the "to top" button) reads this to move out of the
	 * way, so the rule lives here rather than in the mini player alone.
	 */
	get miniVisible() {
		const current = this.current;
		return Boolean(current) && this.onScreenId !== current.book?.id;
	}

	/**
	 * The engine for `book`.
	 *
	 * Returns the live engine when the reader is back on the book that is
	 * playing, so returning to the page continues the same session. Otherwise
	 * it builds a fresh, silent engine: opening another audiobook must never
	 * interrupt what is currently playing.
	 */
	engineFor(book) {
		if (this.#engine && this.#book?.id === book.id) return this.#engine;
		return this.#build(book);
	}

	/**
	 * Hand playback to `engine` and show it in the mini player. Called when
	 * audio actually starts, which is also what makes a second audiobook
	 * replace the first.
	 */
	claim(engine, book) {
		if (engine === this.#engine) {
			this.#book = book;
			return;
		}
		// The previous engine belongs to a page that is already gone: stop it
		// and let it go rather than leaving a silenced element attached.
		this.#engine?.pause();
		this.#engine?.detach();
		this.#engine = engine;
		this.#book = book;
		this.current = { engine, book };
	}

	/**
	 * Forget an engine whose page was torn down. A no-op for the live engine,
	 * which is the whole point: this is how playback outlives the page.
	 */
	release(engine) {
		if (engine === this.#engine) return;
		engine.detach();
	}

	/** Stop playback and dismiss the mini player. */
	close() {
		this.#engine?.pause();
		this.#engine?.detach();
		this.#engine = null;
		this.#book = null;
		this.current = null;
	}

	#build(book) {
		const engine = new AudiobookPlayer(book.tracks, { storageKey: book.id });
		if (browser) {
			const audio = document.createElement('audio');
			audio.preload = 'metadata';
			engine.attach(audio);
			// Attaching installs this engine's media-session handlers, and those
			// are per-document: hand them back to whatever is still playing.
			this.#engine?.setupMediaSession();
		}
		return engine;
	}
}

export const audiobookSession = new AudiobookSession();
