import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
import { Flip } from 'gsap/Flip';

let registered = false;

export function getGsap() {
	if (!registered) {
		gsap.registerPlugin(ScrollTrigger, Flip);
		registered = true;
	}
	return { gsap, ScrollTrigger, Flip };
}
