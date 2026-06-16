<script>
	import Smug from '../svgs/emotes/Smug.svelte';

	let { intro, delay = 0, href } = $props();
</script>

<li
	class="flex md:flex-col gap-2 justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
	class:animate-fly-in={intro}
	style:--delay={`${delay}ms`}
>
	<Smug
		class="relative w-24 hover:scale-110 origin-[60%_96%] duration-100 transition-[scale,rotate] animate-wave"
	/>
	<div class="duo-btn duo-blue">
		<a class="no-underline!" {href}>explore more?</a>
	</div>
</li>
