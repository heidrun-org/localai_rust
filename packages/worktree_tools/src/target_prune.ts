import ChildProcess from 'node:child_process';
import Fs from 'node:fs';
import Path from 'node:path';
import type { CargoCommand, ObsoleteTargetEntry, TargetPrunePlan } from './worktree_tools_types.ts';

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	TargetPrune — finds and deletes the files of the build folders that the current builds do not use any more
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

/**
 * Cargo never deletes the files of an old build. After some weeks, the folders `debug` and `release` of the Cargo
 * target folder hold several old versions of each crate, and only the newest version is used. This class asks Cargo
 * which compilation units the current builds use, and finds the files of every other unit. The name of each such file
 * holds the hash of its unit, so no file that a current build uses is ever in the result.
 */
export class TargetPrune {
	/**
	 * The Cargo commands whose files are kept: the scripts `build`, `cli`, and `test` of the `package.json` of each
	 * package. Each command runs in the folder of its package, like the scripts, because Cargo reads the file
	 * `.cargo/config.toml` of the package only there, and because Cargo gives a shared dependency other features, and
	 * so another hash, when it builds several packages together.
	 */
	static readonly cargoCommands: CargoCommand[] = [
		TargetPrune._packageCommand('localai_embeddings', ['build']),
		TargetPrune._packageCommand('localai_embeddings', ['test', '--no-run']),
		TargetPrune._packageCommand('localai_embeddings', ['build', '--release']),
		TargetPrune._packageCommand('localai_decisions', ['build', '--release']),
		TargetPrune._packageCommand('localai_decisions', ['test', '--release', '--no-run']),
		TargetPrune._packageCommand('localai_llm', ['build', '--release']),
		TargetPrune._packageCommand('localai_llm', ['test', '--release', '--no-run']),
	];

	/** The folders of a build folder where the name of each entry ends with the hash of a compilation unit. */
	static readonly hashedFolderNames = ['deps', '.fingerprint', 'build'];

	/** The name of the folder, inside a build folder, that holds the caches of the incremental compilation. */
	static readonly incrementalFolderName = 'incremental';

	/** The number of cache folders that the plan keeps for each crate, the most recently changed first. */
	static readonly keptIncrementalFoldersPerCrate = 2;

	/** The largest output of one Cargo command that the class reads, in bytes. */
	static readonly maximumOutputBytes = 256 * 1024 * 1024;

	/** The level of the Cargo log that prints the fingerprint folder of every compilation unit. */
	private static readonly _cargoLog = 'cargo::core::compiler::fingerprint=debug';

	/** Finds the hash of a compilation unit in the line of the Cargo log that names its fingerprint folder. */
	private static readonly _fingerprintPattern = /\.fingerprint\/[^/\s]+-([0-9a-f]{16})\//g;

	/** Finds the hash at the end of the name of a file or of a folder of a build folder. */
	private static readonly _nameHashPattern = /-([0-9a-f]{16})(?:\.[^/]*)?$/;

	/**
	 * Runs the Cargo commands and reads the hash of every compilation unit that they use. A unit that is already
	 * compiled counts too. A command compiles whatever is out of date, so it can take minutes.
	 * @param options.checkoutPath The absolute path of the root of the checkout.
	 * @param options.cargoCommands The Cargo commands. The default is `cargoCommands`.
	 * @returns The hashes of the units that the current builds use.
	 * @throws An error when a Cargo command does not start or fails, or when the Cargo log names no unit. Nothing is
	 * deleted by this method.
	 */
	static findUsedHashes(options: { checkoutPath: string; cargoCommands?: CargoCommand[] }): Set<string> {
		const usedHashes = new Set<string>();
		for (const cargoCommand of options.cargoCommands ?? TargetPrune.cargoCommands) {
			const commandText = TargetPrune.describe(cargoCommand);
			const result = ChildProcess.spawnSync('cargo', cargoCommand.arguments, {
				cwd: Path.join(options.checkoutPath, cargoCommand.folderRelativePath),
				encoding: 'utf8',
				maxBuffer: TargetPrune.maximumOutputBytes,
				env: {
					...process.env,
					CARGO_LOG: TargetPrune._cargoLog,
					CARGO_TERM_COLOR: 'never',
				},
			});
			if (result.error !== undefined) {
				throw new Error(`"${commandText}" did not start: ${result.error.message}`);
			}
			if (result.status !== 0) {
				const output = result.stderr.split('\n').slice(-30).join('\n').trim();
				throw new Error(
					`"${commandText}" failed with the exit code ${result.status}, so no file is deleted.\n${output}`,
				);
			}
			for (const match of result.stderr.matchAll(TargetPrune._fingerprintPattern)) {
				usedHashes.add(match[1] as string);
			}
		}
		if (usedHashes.size === 0) {
			throw new Error(
				'The Cargo log names no compilation unit, so no file is deleted. The format of the log may have '
					+ 'changed in this version of Cargo.',
			);
		}
		return usedHashes;
	}

	/**
	 * Gives the text of a Cargo command, for the messages.
	 * @param cargoCommand The Cargo command.
	 * @returns The text, for example `cargo build --release` in `packages/localai_llm`.
	 */
	static describe(cargoCommand: CargoCommand): string {
		const text = `cargo ${cargoCommand.arguments.join(' ')}`;
		if (cargoCommand.folderRelativePath === '.') {
			return text;
		}
		return `${text} in ${cargoCommand.folderRelativePath}`;
	}

	/**
	 * Lists the files and the folders of the build folders that no current build uses. In the folders `deps`,
	 * `.fingerprint`, and `build`, an entry is obsolete when the hash in its name is not one of the used hashes. An
	 * entry without a hash in its name is kept. In the folder `incremental`, the plan keeps the most recently changed
	 * cache folders of each crate, because the name of a cache folder holds no hash of a unit. The method deletes
	 * nothing.
	 * @param options.profilePaths The absolute paths of the build folders of the Cargo target folder, for example the
	 * folders `debug` and `release`.
	 * @param options.usedHashes The hashes of the units that the current builds use, from `findUsedHashes`.
	 * @returns The obsolete entries, and the number of files that the plan deletes and keeps.
	 */
	static plan(options: { profilePaths: string[]; usedHashes: Set<string> }): TargetPrunePlan {
		const obsoleteEntries: ObsoleteTargetEntry[] = [];
		let keptFileCount = 0;
		for (const profilePath of options.profilePaths) {
			for (const folderName of TargetPrune.hashedFolderNames) {
				const folderPlan = TargetPrune._planHashedFolder(profilePath, folderName, options.usedHashes);
				obsoleteEntries.push(...folderPlan.obsoleteEntries);
				keptFileCount += folderPlan.keptFileCount;
			}
			const incrementalPlan = TargetPrune._planIncrementalFolder(profilePath);
			obsoleteEntries.push(...incrementalPlan.obsoleteEntries);
			keptFileCount += incrementalPlan.keptFileCount;
		}
		let obsoleteFileCount = 0;
		for (const obsoleteEntry of obsoleteEntries) {
			obsoleteFileCount += obsoleteEntry.fileCount;
		}
		return {
			obsoleteEntries,
			obsoleteFileCount,
			keptFileCount,
		};
	}

	/**
	 * Deletes the obsolete entries of a plan.
	 * @param plan The plan from `plan`.
	 * @returns Nothing.
	 */
	static remove(plan: TargetPrunePlan): void {
		for (const obsoleteEntry of plan.obsoleteEntries) {
			Fs.rmSync(obsoleteEntry.path, { recursive: true, force: true });
		}
	}

	/**
	 * Gives a Cargo command that runs in the folder of one package of this repository.
	 * @param packageName The name of the folder of the package, inside the folder `packages`.
	 * @param cargoArguments The arguments of `cargo`.
	 * @returns The Cargo command.
	 */
	private static _packageCommand(packageName: string, cargoArguments: string[]): CargoCommand {
		return {
			folderRelativePath: Path.join('packages', packageName),
			arguments: cargoArguments,
		};
	}

	/**
	 * Plans one folder where the name of each entry ends with the hash of a compilation unit.
	 * @param profilePath The absolute path of the build folder.
	 * @param folderName The name of the folder inside the build folder.
	 * @param usedHashes The hashes of the units that the current builds use.
	 * @returns The obsolete entries of the folder, and the number of files that are kept. A missing folder gives none.
	 */
	private static _planHashedFolder(
		profilePath: string,
		folderName: string,
		usedHashes: Set<string>,
	): { obsoleteEntries: ObsoleteTargetEntry[]; keptFileCount: number } {
		const folderPath = Path.join(profilePath, folderName);
		const obsoleteEntries: ObsoleteTargetEntry[] = [];
		let keptFileCount = 0;
		if (Fs.existsSync(folderPath) === false) {
			return { obsoleteEntries, keptFileCount };
		}
		for (const entryName of Fs.readdirSync(folderPath)) {
			const entryPath = Path.join(folderPath, entryName);
			const hash = TargetPrune._readHash(entryName);
			const fileCount = TargetPrune._countFiles(entryPath);
			if (hash === null || usedHashes.has(hash) === true) {
				keptFileCount += fileCount;
				continue;
			}
			obsoleteEntries.push({
				path: entryPath,
				folderName: Path.join(Path.basename(profilePath), folderName),
				fileCount,
			});
		}
		return { obsoleteEntries, keptFileCount };
	}

	/**
	 * Plans the folder `incremental`. Its entries are named `<crate>-<identifier>`, and the identifier is not the hash
	 * of a unit, so the plan keeps the most recently changed entries of each crate.
	 * @param profilePath The absolute path of the build folder.
	 * @returns The obsolete entries of the folder, and the number of files that are kept. A missing folder gives none.
	 */
	private static _planIncrementalFolder(
		profilePath: string,
	): { obsoleteEntries: ObsoleteTargetEntry[]; keptFileCount: number } {
		const folderPath = Path.join(profilePath, TargetPrune.incrementalFolderName);
		const obsoleteEntries: ObsoleteTargetEntry[] = [];
		let keptFileCount = 0;
		if (Fs.existsSync(folderPath) === false) {
			return { obsoleteEntries, keptFileCount };
		}
		const entriesOfCrate = new Map<string, { entryName: string; modifiedMilliseconds: number }[]>();
		for (const entryName of Fs.readdirSync(folderPath)) {
			const crateName = entryName.slice(0, Math.max(entryName.lastIndexOf('-'), 0));
			const entries = entriesOfCrate.get(crateName) ?? [];
			entries.push({
				entryName,
				modifiedMilliseconds: Fs.lstatSync(Path.join(folderPath, entryName)).mtimeMs,
			});
			entriesOfCrate.set(crateName, entries);
		}
		for (const entries of entriesOfCrate.values()) {
			entries.sort((first, second) => {
				return second.modifiedMilliseconds - first.modifiedMilliseconds;
			});
			entries.forEach((entry, index) => {
				const entryPath = Path.join(folderPath, entry.entryName);
				const fileCount = TargetPrune._countFiles(entryPath);
				if (index < TargetPrune.keptIncrementalFoldersPerCrate) {
					keptFileCount += fileCount;
					return;
				}
				obsoleteEntries.push({
					path: entryPath,
					folderName: Path.join(Path.basename(profilePath), TargetPrune.incrementalFolderName),
					fileCount,
				});
			});
		}
		return { obsoleteEntries, keptFileCount };
	}

	/**
	 * Reads the hash of a compilation unit at the end of the name of an entry.
	 * @param entryName The name of a file or of a folder, for example `libserde-0123456789abcdef.rlib`.
	 * @returns The 16 hexadecimal characters of the hash, or null when the name holds no hash.
	 */
	private static _readHash(entryName: string): string | null {
		const match = TargetPrune._nameHashPattern.exec(entryName);
		if (match === null) {
			return null;
		}
		return match[1] as string;
	}

	/**
	 * Counts the files in a file or in a folder. The method does not follow symbolic links.
	 * @param entryPath The absolute path of the file or of the folder.
	 * @returns The number of files: one for a file.
	 */
	private static _countFiles(entryPath: string): number {
		if (Fs.lstatSync(entryPath).isDirectory() === false) {
			return 1;
		}
		let fileCount = 0;
		for (const entryName of Fs.readdirSync(entryPath)) {
			fileCount += TargetPrune._countFiles(Path.join(entryPath, entryName));
		}
		return fileCount;
	}
}
