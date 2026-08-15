/**
 * Chunked demo upload for js-dos bundles and v86 game packages, lifted out of
 * `ProjectEditor.svelte`. The logic is unchanged from the original — this only
 * replaces the direct `editor.status = ...` writes with an `onProgress`
 * callback, so the caller decides where progress text goes (previously it
 * shared a field with the 2-second-autoclearing save toast, so a slow upload
 * could have its progress blanked mid-transfer by that timer).
 *
 * @param {object} deps
 * @param {() => string} deps.authHeader
 * @param {typeof fetch} [deps.fetchImpl]
 * @param {(message: string) => void} [deps.onProgress]
 */
export function createUploadController({ authHeader, fetchImpl = fetch, onProgress = () => {} }) {
	async function prepareV86Artifact({
		file,
		sourceProjectId,
		systemVersionId,
		expectedArtifactRevision,
		manifest
	}) {
		const request = {
			source_project_id: sourceProjectId ?? null,
			system_version_id: Number(systemVersionId),
			expected_artifact_revision: sourceProjectId ? expectedArtifactRevision : 0,
			manifest,
			file_name: file?.name ?? null,
			size_bytes: file?.size ?? null
		};
		const start = await fetchImpl('/api/v86/games/upload', {
			method: 'POST',
			headers: { Authorization: authHeader(), 'Content-Type': 'application/json' },
			body: JSON.stringify(request)
		});
		if (!start.ok) throw new Error(await start.text());
		const session = await start.json();
		try {
			if (session.upload_required) {
				onProgress('Uploading v86 game…');
				for (
					let index = session.next_chunk_index;
					index * session.chunk_size_bytes < file.size;
					index++
				) {
					const startByte = index * session.chunk_size_bytes;
					const chunk = file.slice(
						startByte,
						Math.min(startByte + session.chunk_size_bytes, file.size)
					);
					const result = await fetchImpl(
						`/api/v86/games/upload/${session.upload_id}/chunk/${index}`,
						{
							method: 'PUT',
							headers: { Authorization: authHeader(), 'Content-Type': 'application/octet-stream' },
							body: chunk
						}
					);
					if (!result.ok) throw new Error(await result.text());
					onProgress(
						`Uploading v86 game… ${Math.round(((startByte + chunk.size) / file.size) * 100)}%`
					);
				}
			}
			onProgress('Building v86 disc…');
			const complete = await fetchImpl(`/api/v86/games/upload/${session.upload_id}/complete`, {
				method: 'POST',
				headers: { Authorization: authHeader() }
			});
			if (!complete.ok) throw new Error(await complete.text());
			const uploadId = session.upload_id;
			await new Promise((resolve, reject) => {
				const interval = setInterval(async () => {
					try {
						const res = await fetchImpl(`/api/v86/games/upload/${uploadId}`, {
							headers: { Authorization: authHeader() }
						});
						if (!res.ok) {
							clearInterval(interval);
							reject(new Error(await res.text()));
							return;
						}
						const data = await res.json();
						if (data.chunk_progress?.message) onProgress(data.chunk_progress.message);
						if (data.status === 'ready') {
							clearInterval(interval);
							resolve();
						}
						if (data.status === 'failed') {
							clearInterval(interval);
							await fetchImpl(`/api/v86/games/upload/${uploadId}`, {
								method: 'DELETE',
								headers: { Authorization: authHeader() }
							}).catch(() => {});
							reject(new Error(data.error_message ?? 'Game build failed.'));
						}
					} catch (e) {
						clearInterval(interval);
						reject(e);
					}
				}, 800);
			});
			return uploadId;
		} catch (error) {
			await fetchImpl(`/api/v86/games/upload/${session.upload_id}`, {
				method: 'DELETE',
				headers: { Authorization: authHeader() }
			}).catch(() => {});
			throw error;
		}
	}

	async function uploadJsDosBundle(projectId, file) {
		const start = await fetchImpl(`/api/projects/id/${projectId}/jsdos/upload`, {
			method: 'POST',
			headers: { Authorization: authHeader(), 'Content-Type': 'application/json' },
			body: JSON.stringify({ file_name: file.name, size_bytes: file.size })
		});
		if (!start.ok) throw new Error(await start.text());
		const session = await start.json();
		onProgress('Uploading game…');
		for (
			let index = session.next_chunk_index;
			index * session.chunk_size_bytes < file.size;
			index++
		) {
			const startByte = index * session.chunk_size_bytes;
			const chunk = file.slice(
				startByte,
				Math.min(startByte + session.chunk_size_bytes, file.size)
			);
			const chunkRes = await fetchImpl(
				`/api/projects/id/${projectId}/jsdos/upload/${session.upload_id}/chunk/${index}`,
				{
					method: 'PUT',
					headers: { Authorization: authHeader(), 'Content-Type': 'application/octet-stream' },
					body: chunk
				}
			);
			if (!chunkRes.ok) throw new Error(await chunkRes.text());
			const progress = Math.round(((startByte + chunk.size) / file.size) * 100);
			onProgress(`Uploading game… ${progress}%`);
		}
		const complete = await fetchImpl(
			`/api/projects/id/${projectId}/jsdos/upload/${session.upload_id}/complete`,
			{ method: 'POST', headers: { Authorization: authHeader() } }
		);
		if (!complete.ok) throw new Error(await complete.text());
	}

	return { prepareV86Artifact, uploadJsDosBundle };
}
