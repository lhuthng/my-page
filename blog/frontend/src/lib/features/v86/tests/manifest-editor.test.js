import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	blankVariant,
	parseManifestModel,
	serializeManifestModel,
	splitSavePaths
} from '../manifest-editor.js';
import { v86VariantError } from '../../editor/model/demo.js';

function roundTrip(manifest) {
	return serializeManifestModel(parseManifestModel(manifest));
}

test('single unnamed variant parses and re-serializes', () => {
	const model = parseManifestModel('exe=game.exe\nargs=/s');
	assert.deepEqual(model.variants, [{ name: '', exe: 'game.exe', args: '/s' }]);
	assert.equal(model.delayMs, '');
	assert.equal(model.revertMouseY, false);
	assert.deepEqual(model.savePaths, []);
	assert.deepEqual(model.unknown, []);
	assert.equal(roundTrip('exe=game.exe\nargs=/s'), 'name=\nexe=game.exe\nargs=/s');
});

test('named multi-variant manifests keep root fallback semantics', () => {
	const model = parseManifestModel('exe=base.exe\nname=English\nname2=Vietnamese\nexe2=vn.exe');
	assert.equal(model.variants.length, 2);
	assert.deepEqual(model.variants[1], { name: 'Vietnamese', exe: 'vn.exe', args: '' });
	// variant 2 inherits root args; serialization only writes set suffixed keys
	assert.equal(
		roundTrip('exe=base.exe\nargs=/x\nname=English\nname2=Vietnamese\nexe2=vn.exe'),
		'name=English\nexe=base.exe\nargs=/x\nname2=Vietnamese\nexe2=vn.exe'
	);
});

test('globals parse and emit only when non-default', () => {
	const model = parseManifestModel(
		'exe=g.exe\ndelay_ms=2000\nmouse_speed=2.0\nrevert_mouse_y=1\nsave_paths=Save.dat; A/x.sav'
	);
	assert.equal(model.delayMs, '2000');
	assert.equal(model.mouseSpeed, '2.0');
	assert.equal(model.revertMouseY, true);
	assert.deepEqual(model.savePaths, ['Save.dat', 'A/x.sav']);
	assert.equal(
		roundTrip(
			'exe=g.exe\ndelay_ms=2000\nmouse_speed=2.0\nrevert_mouse_y=1\nsave_paths=Save.dat; A/x.sav'
		),
		'name=\nexe=g.exe\ndelay_ms=2000\nmouse_speed=2.0\nrevert_mouse_y=1\nsave_paths=Save.dat; A/x.sav'
	);
	// defaults vanish, falsy revert vanishes
	assert.equal(roundTrip('exe=g.exe\nrevert_mouse_y=0'), 'name=\nexe=g.exe');
});

test('save_path/saves aliases merge into save_paths, unknown keys survive', () => {
	const model = parseManifestModel('exe=g.exe\nsaves=A.dat\ncustom_key=42');
	assert.deepEqual(model.savePaths, ['A.dat']);
	assert.deepEqual(model.unknown, [{ key: 'custom_key', value: '42' }]);
	assert.equal(
		roundTrip('exe=g.exe\nsaves=A.dat\ncustom_key=42'),
		'name=\nexe=g.exe\nsave_paths=A.dat\ncustom_key=42'
	);
});

test('comments, sections and casing never leak into the model', () => {
	const model = parseManifestModel('# hi\n[game]\nEXE=G.EXE\nName=En');
	assert.deepEqual(model.variants, [{ name: 'En', exe: 'G.EXE', args: '' }]);
});

test('serialized output always passes the existing save-time validation', () => {
	for (const raw of [
		'exe=game.exe',
		'exe=base.exe\nname=English\nname2=Vietnamese\nexe2=vn.exe',
		'exe=g.exe\ndelay_ms=2000\nsave_paths=S.dat'
	]) {
		assert.equal(v86VariantError(roundTrip(raw)), null);
	}
});

test('splitSavePaths accepts commas and semicolons', () => {
	assert.deepEqual(splitSavePaths('A.dat, B.dat; C.dat'), ['A.dat', 'B.dat', 'C.dat']);
	assert.deepEqual(splitSavePaths(''), []);
});

test('blankVariant shape', () => {
	assert.deepEqual(blankVariant(), { name: '', exe: '', args: '' });
});
