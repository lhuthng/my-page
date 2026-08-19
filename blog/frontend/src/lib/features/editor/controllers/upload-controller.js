/**
 * Demo upload flows for js-dos bundles and v86 game packages, lifted out of
 * `ProjectEditor.svelte`. The logic is unchanged from the original — this only
 * replaces the direct `editor.status = ...` writes with an `onProgress`
 * callback, so the caller decides where progress text goes (previously it
 * shared a field with the 2-second-autoclearing save toast, so a slow upload
 * could have its progress blanked mid-transfer by that timer).
 *
 * The v86 flow builds the game disk and launcher CDs in the browser (the
 * server no longer runs mtools/xorriso): the client unzips the game, builds
 * the FAT16 disk, hashes it, and splits/compresses it at the chunk size the
 * server returns after the start call. The server verifies the parts on
 * complete and stores everything content-addressed.
 *
 * @param {object} deps
 * @param {() => string} deps.authHeader
 * @param {typeof fetch} [deps.fetchImpl]
 * @param {(message: string) => void} [deps.onProgress]
 */
import {
	buildGame,
	buildLauncherIsos,
	buildDiskParts,
	DEFAULT_BUILD_DATE
} from '../../v86/build-game.js';
import { createZstdCompress } from '../../v86/zstd.js';

export function createUploadController({ authHeader, fetchImpl = fetch, onProgress = () => {} }) {
	let launcherCache = null;
	async function getLauncher() {
		if (launcherCache) return launcherCache;
		const res = await fetchImpl('/api/v86/launcher', { headers: authHeaders() });
		if (!res.ok) throw new Error(await res.text());
		launcherCache = new Uint8Array(await res.arrayBuffer());
		return launcherCache;
	}

	const jsonHeaders = () => ({
		Authorization: authHeader(),
		'Content-Type': 'application/json'
	});
	const authHeaders = () => ({ Authorization: authHeader() });
	const octetHeaders = () => ({
		Authorization: authHeader(),
		'Content-Type': 'application/octet-stream'
	});

	async function prepareV86Artifact({
		file,
		sourceProjectId,
		systemVersionId,
		expectedArtifactRevision,
		manifest
	}) {
		const launcherExe = await getLauncher();
		let disk = null;
		let variants;
		if (file) {
			const built = await buildGame({
				zipBytes: await file.arrayBuffer(),
				manifest,
				launcherExe,
				onProgress
			});
			disk = built.disk;
			variants = built.variants;
		} else {
			variants = await buildLauncherIsos({ manifest, launcherExe, now: DEFAULT_BUILD_DATE });
		}

		const request = {
			source_project_id: sourceProjectId ?? null,
			system_version_id: Number(systemVersionId),
			expected_artifact_revision: sourceProjectId ? expectedArtifactRevision : 0,
			manifest,
			plans: {
				disk: disk ? { sha256: disk.sha256, size_bytes: disk.size_bytes } : null,
				variants: variants.map((v) => ({
					index: v.index,
					sha256: v.sha256,
					size_bytes: v.size_bytes
				}))
			}
		};
		const start = await fetchImpl('/api/v86/games/upload', {
			method: 'POST',
			headers: jsonHeaders(),
			body: JSON.stringify(request)
		});
		if (!start.ok) throw new Error(await start.text());
		const session = await start.json();
		try {
			const zstdCompress = createZstdCompress();
			if (session.disk && !session.disk.reuse && disk) {
				if (session.disk.sha256 !== disk.sha256) {
					throw new Error('The server disc plan does not match the built disc.');
				}
				const parts = await buildDiskParts(
					disk.sparse,
					session.disk.chunk_size_bytes,
					zstdCompress,
					(done, total) => onProgress(`Compressing game disc ${done}/${total}…`)
				);
				if (parts.chunk_count !== session.disk.chunk_count) {
					throw new Error('The disc chunk count does not match the server plan.');
				}
				for (let index = 0; index < parts.parts.length; index++) {
					const part = parts.parts[index];
					const res = await fetchImpl(`/api/v86/games/upload/${session.upload_id}/disk/${index}`, {
						method: 'PUT',
						headers: octetHeaders(),
						body: part.compressed
					});
					if (!res.ok) throw new Error(await res.text());
					onProgress(
						`Uploading game disc… ${Math.round(((index + 1) / parts.parts.length) * 100)}%`
					);
				}
			}
			for (const spec of session.variants) {
				if (spec.reuse) continue;
				const built = variants.find((v) => v.index === spec.index);
				if (!built) {
					throw new Error('A launcher CD build is missing for a variant.');
				}
				if (built.sha256 !== spec.sha256) {
					throw new Error('The server launcher plan does not match the built CD.');
				}
				const res = await fetchImpl(
					`/api/v86/games/upload/${session.upload_id}/iso/${spec.index}`,
					{ method: 'PUT', headers: octetHeaders(), body: built.bytes }
				);
				if (!res.ok) throw new Error(await res.text());
				onProgress(`Uploading launcher ${spec.index}…`);
			}
			onProgress('Building v86 disc…');
			const complete = await fetchImpl(`/api/v86/games/upload/${session.upload_id}/complete`, {
				method: 'POST',
				headers: authHeaders()
			});
			if (!complete.ok) throw new Error(await complete.text());
			const uploadId = session.upload_id;
			await new Promise((resolve, reject) => {
				const interval = setInterval(async () => {
					try {
						const res = await fetchImpl(`/api/v86/games/upload/${uploadId}`, {
							headers: authHeaders()
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
								headers: authHeaders()
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
				headers: authHeaders()
			}).catch(() => {});
			throw error;
		}
	}

	async function uploadJsDosBundle(projectId, file) {
		const start = await fetchImpl(`/api/projects/id/${projectId}/jsdos/upload`, {
			method: 'POST',
			headers: jsonHeaders(),
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
					headers: octetHeaders(),
					body: chunk
				}
			);
			if (!chunkRes.ok) throw new Error(await chunkRes.text());
			const progress = Math.round(((startByte + chunk.size) / file.size) * 100);
			onProgress(`Uploading game… ${progress}%`);
		}
		const complete = await fetchImpl(
			`/api/projects/id/${projectId}/jsdos/upload/${session.upload_id}/complete`,
			{ method: 'POST', headers: authHeaders() }
		);
		if (!complete.ok) throw new Error(await complete.text());
	}

	return { prepareV86Artifact, uploadJsDosBundle };
}
