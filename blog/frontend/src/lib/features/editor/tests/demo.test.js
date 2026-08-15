import test from 'node:test';
import assert from 'node:assert/strict';
import {
	applyDemoTypeTransition,
	parseV86Fields,
	v86VariantError,
	validateDemoFields
} from '../model/demo.js';

test('applyDemoTypeTransition clears the zip but not the url for embed/download/video', () => {
	for (const type of ['embed', 'download', 'video']) {
		const patch = applyDemoTypeTransition(type);
		assert.equal(patch.demoType, type);
		assert.equal(patch.demoZip, undefined);
		assert.equal(patch.demoZipName, '');
		assert.equal(patch.demoZipError, '');
		assert.equal('demoUrl' in patch, false, `${type} should not touch demoUrl`);
	}
});

test('applyDemoTypeTransition clears both the zip and the url for none', () => {
	const patch = applyDemoTypeTransition('none');
	assert.equal(patch.demoZip, undefined);
	assert.equal(patch.demoUrl, '');
});

test('applyDemoTypeTransition clears only the url for zip-based types', () => {
	for (const type of ['html5', 'webgl', 'jsdos', 'v86']) {
		const patch = applyDemoTypeTransition(type);
		assert.equal(patch.demoUrl, '', type);
		assert.equal('demoZip' in patch, false, `${type} should not touch demoZip`);
	}
});

test('parseV86Fields ignores comments, blank lines, and section headers', () => {
	const manifest = [
		'[general]',
		'# a comment',
		'; another comment',
		'',
		'exe = game.exe',
		'args=--fast'
	].join('\n');
	assert.deepEqual(parseV86Fields(manifest), { exe: 'game.exe', args: '--fast' });
});

test('parseV86Fields lower-cases keys and trims values', () => {
	assert.deepEqual(parseV86Fields('  EXE  =  Game.exe  '), { exe: 'Game.exe' });
});

test('v86VariantError accepts a single unnumbered variant', () => {
	assert.equal(v86VariantError('name=Game\nexe=game.exe'), null);
});

test('v86VariantError requires an exe suffix', () => {
	assert.match(v86VariantError('name=Game\nexe=game.bin') ?? '', /must be an \.exe file/);
});

test('v86VariantError requires an executable at all', () => {
	assert.match(v86VariantError('name=Game') ?? '', /requires an executable/);
});

test('v86VariantError accepts contiguous numbered variants', () => {
	const manifest = ['name1=A', 'exe1=a.exe', 'name2=B', 'exe2=b.exe'].join('\n');
	assert.equal(v86VariantError(manifest), null);
});

test('v86VariantError rejects a gap in numbered variants', () => {
	// name2 present but name1 missing: variant 1 has no name.
	const manifest = ['name2=B', 'exe1=a.exe', 'exe2=b.exe'].join('\n');
	assert.match(v86VariantError(manifest) ?? '', /Missing name for variant 1/);
});

test('v86VariantError falls back to the shared exe/args for a numbered variant', () => {
	const manifest = ['name1=A', 'name2=B', 'exe=shared.exe'].join('\n');
	assert.equal(v86VariantError(manifest), null);
});

test('v86VariantError rejects exeN referencing beyond the named variants', () => {
	const manifest = ['name=A', 'exe=a.exe', 'exe3=c.exe'].join('\n');
	assert.match(v86VariantError(manifest) ?? '', /only 1 named variant/);
});

test('validateDemoFields requires a zip only on create for html5/webgl', () => {
	assert.equal(
		validateDemoFields({ demoType: 'html5', mode: 'create', zip: undefined }).valid,
		false
	);
	assert.equal(validateDemoFields({ demoType: 'html5', mode: 'create', zip: {} }).valid, true);
	// Editing an existing html5 project without re-uploading a zip is fine.
	assert.equal(validateDemoFields({ demoType: 'html5', mode: 'edit', zip: undefined }).valid, true);
});

test('validateDemoFields requires a url for embed/download/video', () => {
	for (const demoType of ['embed', 'download', 'video']) {
		assert.equal(validateDemoFields({ demoType, mode: 'edit', demoUrl: '' }).valid, false);
		assert.equal(validateDemoFields({ demoType, mode: 'edit', demoUrl: 'https://x' }).valid, true);
	}
});

test('validateDemoFields requires a v86 system and a fresh zip on create', () => {
	const noSystem = validateDemoFields({ demoType: 'v86', mode: 'create', v86SystemVersionId: '' });
	assert.equal(noSystem.valid, false);
	assert.match(noSystem.error, /Select a v86 system/);

	const noZip = validateDemoFields({
		demoType: 'v86',
		mode: 'create',
		v86SystemVersionId: '3',
		zip: undefined
	});
	assert.equal(noZip.valid, false);
	assert.match(noZip.error, /game ZIP is required/);
});

test('validateDemoFields allows editing an existing v86 project without a fresh zip', () => {
	const result = validateDemoFields({
		demoType: 'v86',
		mode: 'edit',
		previousDemoType: 'v86',
		v86SystemVersionId: '3',
		zip: undefined,
		v86Manifest: 'name=A\nexe=a.exe'
	});
	assert.equal(result.valid, true);
});

test('validateDemoFields requires a fresh zip when switching an existing project to v86', () => {
	const result = validateDemoFields({
		demoType: 'v86',
		mode: 'edit',
		previousDemoType: 'html5',
		v86SystemVersionId: '3',
		zip: undefined
	});
	assert.equal(result.valid, false);
	assert.match(result.error, /game ZIP is required/);
});

test('validateDemoFields rejects an oversized v86 manifest', () => {
	const result = validateDemoFields({
		demoType: 'v86',
		mode: 'edit',
		previousDemoType: 'v86',
		v86SystemVersionId: '3',
		v86Manifest: 'a'.repeat(70000)
	});
	assert.equal(result.valid, false);
	assert.match(result.error, /64 KiB/);
});

test('validateDemoFields propagates a variant validation error', () => {
	const result = validateDemoFields({
		demoType: 'v86',
		mode: 'edit',
		previousDemoType: 'v86',
		v86SystemVersionId: '3',
		v86Manifest: 'exe=a.exe'
	});
	assert.equal(result.valid, false);
	assert.match(result.error, /Missing name/);
});

test('validateDemoFields accepts none and jsdos-on-edit without extra requirements', () => {
	assert.equal(validateDemoFields({ demoType: 'none', mode: 'edit' }).valid, true);
	assert.equal(validateDemoFields({ demoType: 'jsdos', mode: 'edit', zip: undefined }).valid, true);
});

test('validateDemoFields rejects an unknown demo type', () => {
	const result = validateDemoFields({ demoType: 'bogus', mode: 'edit' });
	assert.equal(result.valid, false);
	assert.match(result.error, /Unsupported demo type/);
});
