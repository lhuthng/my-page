<script>
	import { logout, user } from "$lib/client/user";

	let displayName = $derived($user?.displayName);
</script>

<header class="fixed w-full h-18 bg-gray-200">
	<div class="flex w-full justify-between p-4">
		<div class="bg-red-500 rounded-full w-10 h-10"></div>
		<div class="flex flex-col items-end">
			<div class="flex gap-8">
				<span>Tags</span>
				<span>Archives</span>
				<span>Portfolio</span>
				<span>About</span>
			</div>
			<div>
				{#if displayName}
					<span
						>Hi <span class="text-blue-700">{displayName}</span
						></span
					>
					<button
						class="ring-1 rounded-full px-2"
						type="button"
						onclick={() => logout()}>Log out</button
					>
				{:else}
					<span>Sign up</span>
					<a class="text-white bg-black px-2" href="/login">Log in</a>
				{/if}
			</div>
		</div>
	</div>
</header>
