import Fs from 'node:fs';
import Path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { BuildScriptOutputs } from '../src/build_script_outputs.ts';
import { TargetPaths } from '../src/target_paths.ts';
import { TestRepository } from './test_repository.ts';

describe('BuildScriptOutputs', () => {
	afterEach(() => {
		TestRepository.removeAll();
	});

	it('replaces the path of the main checkout with the path of the worktree, and keeps the date', () => {
		const mainCheckoutPath = TestRepository.create();
		const worktreePath = TestRepository.addWorktree(mainCheckoutPath, 'feature');
		const buildFolderName = 'localai_decisions-0123456789abcdef';
		const outputFolderPath = Path.join(TargetPaths.debugPath(worktreePath), 'build', buildFolderName);
		Fs.mkdirSync(outputFolderPath, { recursive: true });
		const outputPath = Path.join(outputFolderPath, BuildScriptOutputs.outputFileName);
		Fs.writeFileSync(outputPath, [
			`cargo:rerun-if-changed=${mainCheckoutPath}/packages/localai_decisions/src/clef_decision_order.cpp`,
			'cargo:rerun-if-env-changed=GGML_METAL',
			'',
		].join('\n'));
		const oldDate = new Date('2026-01-01T00:00:00Z');
		Fs.utimesSync(outputPath, oldDate, oldDate);

		expect(BuildScriptOutputs.rewrite(mainCheckoutPath, worktreePath)).toBe(1);

		expect(Fs.readFileSync(outputPath, 'utf8')).toBe([
			`cargo:rerun-if-changed=${worktreePath}/packages/localai_decisions/src/clef_decision_order.cpp`,
			'cargo:rerun-if-env-changed=GGML_METAL',
			'',
		].join('\n'));
		expect(Fs.statSync(outputPath).mtimeMs).toBe(oldDate.getTime());
	});

	it('replaces the path of the target folder of the main checkout in a record of the release folder', () => {
		const mainCheckoutPath = TestRepository.create();
		const worktreePath = TestRepository.addWorktree(mainCheckoutPath, 'feature');
		const buildFolderName = 'llama-cpp-sys-2-0123456789abcdef';
		const outputFolderPath = Path.join(TargetPaths.releasePath(worktreePath), 'build', buildFolderName);
		Fs.mkdirSync(outputFolderPath, { recursive: true });
		const outputPath = Path.join(outputFolderPath, BuildScriptOutputs.outputFileName);
		const libraryRelativePath = `release/build/${buildFolderName}/out/lib`;
		Fs.writeFileSync(outputPath, `cargo:rustc-link-search=native=${mainCheckoutPath}/target/${libraryRelativePath}\n`);

		expect(BuildScriptOutputs.rewrite(mainCheckoutPath, worktreePath)).toBe(1);

		expect(Fs.readFileSync(outputPath, 'utf8'))
			.toBe(`cargo:rustc-link-search=native=${worktreePath}/target/${libraryRelativePath}\n`);
	});

	it('changes nothing in a record without the path of the main checkout, or without a debug folder', () => {
		const mainCheckoutPath = TestRepository.create();
		const worktreePath = TestRepository.addWorktree(mainCheckoutPath, 'feature');
		expect(BuildScriptOutputs.rewrite(mainCheckoutPath, worktreePath)).toBe(0);
		const outputFolderPath = Path.join(TargetPaths.debugPath(worktreePath), 'build', 'serde-0123456789abcdef');
		Fs.mkdirSync(outputFolderPath, { recursive: true });
		Fs.writeFileSync(Path.join(outputFolderPath, BuildScriptOutputs.outputFileName), 'cargo:rustc-cfg=fast\n');
		expect(BuildScriptOutputs.rewrite(mainCheckoutPath, worktreePath)).toBe(0);
	});
});
