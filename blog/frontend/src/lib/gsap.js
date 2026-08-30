let gsapPromise = null;

/**
 * GSAP is only needed for a few animations (comment scroll trigger, image
 * previewer, home Flip transitions), so it is fetched as its own chunk the
 * first time someone asks for it instead of shipping with the initial page.
 * Returns a promise; call sites must handle it not being ready yet.
 */
export function getGsap() {
	gsapPromise ??= Promise.all([
		import('gsap'),
		import('gsap/ScrollTrigger'),
		import('gsap/Flip')
	]).then(([{ gsap }, { ScrollTrigger }, { Flip }]) => {
		gsap.registerPlugin(ScrollTrigger, Flip);
		return { gsap, ScrollTrigger, Flip };
	});
	return gsapPromise;
}
