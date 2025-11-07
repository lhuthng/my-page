<script>
	let isLogging = $state(true);
	let username = $state("");
	let password = $state("");
	let repassword = $state("");
	let email = $state("");
	async function login() {
		try {
			const response = await fetch("/api/auth/login", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					username: username,
					password: password,
				}),
			});
			if (!response.ok) {
				const errorData = await response.text();
				console.error(
					"Registration failed with status:",
					response.status,
					errorData,
				);
				return;
			}

			const data = await response.json();
			console.log("Login successful! Response data:", data);
		} catch (error) {
			console.error("Network or fetch error during login:", error);
		}
	}
	async function register() {
		if (password !== repassword) {
			alert("password doesn't match repassword");
			return;
		}
		try {
			const response = await fetch("/api/auth/register", {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					username: username,
					password: password,
					email: email,
				}),
			});

			if (!response.ok) {
				const errorData = await response.text();
				console.error(
					"Registration failed with status:",
					response.status,
					errorData,
				);
				return;
			}

			const data = await response.json();
			console.log("Registration successful! Response data:", data);
		} catch (error) {
			console.error("Network or fetch error during registration:", error);
		}
	}

	let handleSubmit = $derived(isLogging ? login : register);
</script>

<div class="flex flex-col w-cap h-screen min-h-200">
	<div class="h-18"></div>
	<div class="flex bg-green-200 w-full h-160 my-auto">
		<div class="grow"></div>
		<div class="w-120 bg-green-300 h-full p-4">
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
				<input type="submit" value={isLogging ? "Login" : "Signup"} />
			</form>
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "../../app.css";
</style>
