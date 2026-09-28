<script>
	import { PERSONAS } from '$lib/config/personas.js';
</script>

<div class="w-full bg-black text-background text-lg space-y-4 rounded-xl p-2">
	<div class="relative flex not-sm:flex-col">
		<div
			class="not-sm:absolute z-9 min-w-full sm:min-w-[max(16rem,20%)] not-sm:h-full rounded-lg overflow-hidden"
		>
			<div
				class="absolute w-full not-sm:-top-20 not-sm:opacity-80 sm:static sm:flex mask-b-from-10% sm:mask-r-from-50% sm:mask-r-to-80%"
			>
				<img
					class="w-full h-80 object-cover sm:object-contain object-top-left brightness-110 my-auto"
					src="/me-1.jpg"
					alt="me"
				/>
			</div>
		</div>
		<div class="relative z-10 not-md:px-2 [&>p]:text-base [&>p]:lg:text-lg space-y-2">
			<h2 class="text-xl md:text-2xl pt-4 pb-2">
				Welcome to the <strong>Field</strong>
				<span aria-hidden="true" class="text-sm opacity-60 font-normal whitespace-normal">
					ヾ(＾∇＾)
				</span>
			</h2>
			<p>
				I'm <a href="/profiles/lhuthng">Thắng</a>
				, and this is my digital garden. This site serves as a personal archive where I document my work
				experiences, my evolving hobbies, and the various experiments I run in my spare time.
			</p>
			<p>
				Everything you see here is a reflection of my curiosity. Whether it's a deep dive into a
				technical challenge or a small creative spark, I wanted a central place to capture the
				process. <span aria-hidden="true" class="opacity-50 text-sm whitespace-normal">
					(ง •̀_•́)ง
				</span>
			</p>
		</div>
	</div>
	<div class="flex flex-col w-full max-w-200 mx-auto px-2 gap-6 text-white text-base lg:text-lg">
		<p class="md:w-3/5">
			One person writes all of this: to keep the archive navigable, I publish under <span
				class="text-red-400"
			>
				three accounts
			</span>
			. The byline on any post or project tells you which kind of thing it is.
		</p>

		<div class="flex flex-col gap-4">
			<img class="mx-auto w-180 py-2" src="/thinkcats.jpg" alt="The three persona cats" />

			<ul class="grid grid-cols-1 md:grid-cols-3 gap-3">
				{#each PERSONAS as persona}
					<li
						class="border-role-border flex flex-col gap-1 rounded-lg border-3 bg-white/5 p-3"
						data-role={persona.role}
					>
						<a class="w-fit font-semibold text-role-on-dark" href={`/profiles/${persona.slug}`}>
							{persona.name}
						</a>
						<p class="w-auto text-sm leading-5 opacity-90">{persona.blurb}</p>
					</li>
				{/each}
			</ul>
		</div>

		<div class="flex justify-between w-full *:inline-block">
			<span>
				<a class="text-accent-blue" href="/about">read more</a>
			</span>
			<span aria-hidden="true" class="opacity-60 self-center text-sm whitespace-normal">
				_(:3 」∠)_
			</span>
			<span>
				by <a class="text-background" href="/profiles/lhuthng">@lhuthng</a>
			</span>
		</div>
	</div>
</div>
