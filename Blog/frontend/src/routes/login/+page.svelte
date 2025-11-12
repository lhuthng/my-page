<script>
	import { login, logout, register, user } from "$lib/client/user";

	let isLogged = $derived($user !== undefined);

	let isLogging = $state(true);
	let username = $state("");
	let password = $state("");
	let repassword = $state("");
	let email = $state("");

	async function handleLogin() {
		await login(username, password);
	}

	async function handleRegister() {
		await register(username, password, email);
	}

	let handleSubmit = $derived(isLogging ? handleLogin : handleRegister);
</script>

<div class="flex flex-col w-cap h-screen min-h-200">
	<div class="h-18"></div>
	<div class="flex bg-green-200 w-full h-160 my-auto">
		<div class="grow"></div>
		<div class="w-120 bg-green-300 h-full p-4">
			{#if !isLogged}
				<form
					class="flex flex-col bg-green-400 gap-2 [&>input]:bg-green-500"
					onsubmit={handleSubmit}
				>
					<input
						id="f_username"
						type="text"
						placeholder="username"
						bind:value={username}
					/>
					<input
						id="f_password"
						type="text"
						placeholder="password"
						bind:value={password}
					/>
					{#if !isLogging}
						<input
							id="f_repassword"
							type="text"
							placeholder="repassword"
							bind:value={repassword}
						/>
						<input
							id="f_email"
							type="text"
							placeholder="email"
							bind:value={email}
						/>
						<div>
							<label
								>Already have an account? <button
									type="button"
									onclick={() => (isLogging = true)}
									>Login!</button
								></label
							>
						</div>
					{:else}
						<div>
							<label
								>Don't have an account? <button
									type="button"
									onclick={() => (isLogging = false)}
									>Signup!</button
								></label
							>
						</div>
					{/if}
					<input
						type="submit"
						value={isLogging ? "Login" : "Signup"}
					/>
				</form>
			{:else}
				<div>
					You're already logged. <button onclick={() => logout()}
						>Log out?</button
					>
				</div>
			{/if}
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "../../app.css";
</style>
