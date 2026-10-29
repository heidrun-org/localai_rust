import Fs from 'node:fs';
import Path from 'node:path';
import { TargetPaths } from './target_paths.ts';

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	BuildScriptOutputs — moves the absolute paths of the cloned build script records to the worktree
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

/**
 * Cargo keeps what each build script printed in the file `target/<debug or release>/build/<crate>-<hash>/output`. Some
 * build scripts print absolute paths: the files to watch, or the folder of a compiled C++ library, such as the one of
 * `llama-cpp-sys-2`. After the clone, these paths name the files of the main checkout: Cargo can see a different list
 * of watched files and compile the crate again, and the linker of the worktree can read the libraries of the main
 * checkout. This class replaces the path of the main checkout with the path of the worktree in these files, for the
 * folder `packages` and for the Cargo target folder, and keeps their dates.
 */
export class BuildScriptOutputs {
	/** The name of the file where Cargo keeps what a build script printed. */
	static readonly outputFileName = 'output';

	/** The folders of a checkout whose absolute paths are moved from the main checkout to the worktree. */
	static readonly rewrittenFolderNames = ['packages', TargetPaths.targetRelativePath];

	/**
	 * Rewrites the build script records of the cloned debug folder and release folder of a worktree.
	 * @param mainCheckoutPath The absolute path of the main checkout.
	 * @param worktreePath The absolute path of the worktree.
	 * @returns The number of files that were changed.
	 */
	static rewrite(mainCheckoutPath: string, worktreePath: string): number {
		const prefixPairs = BuildScriptOutputs.rewrittenFolderNames.map((folderName) => {
			return {
				mainPrefix: `${Path.join(mainCheckoutPath, folderName)}${Path.sep}`,
				worktreePrefix: `${Path.join(worktreePath, folderName)}${Path.sep}`,
			};
		});
		let changedCount = 0;
		for (const profilePath of TargetPaths.profilePaths(worktreePath)) {
			changedCount += BuildScriptOutputs._rewriteFolder(Path.join(profilePath, 'build'), prefixPairs);
		}
		return changedCount;
	}

	/**
	 * Rewrites the build script records of one `build` folder.
	 * @param buildPath The absolute path of the folder `build` of one profile.
	 * @param prefixPairs For each folder of `rewrittenFolderNames`, its path in the main checkout and in the worktree,
	 * each with a final separator.
	 * @returns The number of files that were changed.
	 */
	private static _rewriteFolder(
		buildPath: string,
		prefixPairs: { mainPrefix: string; worktreePrefix: string }[],
	): number {
		if (Fs.existsSync(buildPath) === false) {
			return 0;
		}
		let changedCount = 0;
		for (const entryName of Fs.readdirSync(buildPath)) {
			const outputPath = Path.join(buildPath, entryName, BuildScriptOutputs.outputFileName);
			if (Fs.existsSync(outputPath) === false) {
				continue;
			}
			const text = Fs.readFileSync(outputPath, 'utf8');
			let newText = text;
			for (const prefixPair of prefixPairs) {
				newText = newText.split(prefixPair.mainPrefix).join(prefixPair.worktreePrefix);
			}
			if (newText === text) {
				continue;
			}
			const outputStat = Fs.statSync(outputPath);
			Fs.writeFileSync(outputPath, newText);
			Fs.utimesSync(outputPath, outputStat.atime, outputStat.mtime);
			changedCount += 1;
		}
		return changedCount;
	}
}
