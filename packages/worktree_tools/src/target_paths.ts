import Path from 'node:path';

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	TargetPaths — the place of the Cargo target folder inside a checkout of the repository
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

/**
 * Gives the paths of the Cargo target folder of the Cargo workspace at the root of one checkout: the main checkout or
 * a worktree.
 */
export class TargetPaths {
	/** The path of the Cargo target folder, relative to the root of a checkout. */
	static readonly targetRelativePath = 'target';

	/** The name of the folder, inside the Cargo target folder, that holds the debug builds. */
	static readonly debugFolderName = 'debug';

	/** The name of the folder, inside the Cargo target folder, that holds the release builds. */
	static readonly releaseFolderName = 'release';

	/**
	 * Gives the path of the Cargo target folder of a checkout.
	 * @param checkoutPath The absolute path of the root of the main checkout or of a worktree.
	 * @returns The absolute path of the Cargo target folder.
	 */
	static targetPath(checkoutPath: string): string {
		return Path.join(checkoutPath, TargetPaths.targetRelativePath);
	}

	/**
	 * Gives the path of the debug build folder of a checkout.
	 * @param checkoutPath The absolute path of the root of the main checkout or of a worktree.
	 * @returns The absolute path of the `debug` folder inside the Cargo target folder.
	 */
	static debugPath(checkoutPath: string): string {
		return Path.join(TargetPaths.targetPath(checkoutPath), TargetPaths.debugFolderName);
	}

	/**
	 * Gives the path of the release build folder of a checkout.
	 * @param checkoutPath The absolute path of the root of the main checkout or of a worktree.
	 * @returns The absolute path of the `release` folder inside the Cargo target folder.
	 */
	static releasePath(checkoutPath: string): string {
		return Path.join(TargetPaths.targetPath(checkoutPath), TargetPaths.releaseFolderName);
	}

	/**
	 * Gives the paths of the two build folders of a checkout: the debug folder, then the release folder.
	 * @param checkoutPath The absolute path of the root of the main checkout or of a worktree.
	 * @returns The absolute paths of the `debug` folder and of the `release` folder.
	 */
	static profilePaths(checkoutPath: string): string[] {
		return [TargetPaths.debugPath(checkoutPath), TargetPaths.releasePath(checkoutPath)];
	}
}
