/*
 * Used to handle the unrecoverable errors sent by the backend. Provides helper functions for easier error handling
 */
import { invoke } from "@tauri-apps/api";
import type { InvokeArgs } from "@tauri-apps/api/tauri";
import { message } from "@tauri-apps/api/dialog";
import { _ } from "svelte-i18n";
import { exit } from "@tauri-apps/api/process";
import { get } from "svelte/store";

export type AppError = {
	Recoverable?: RecoverableAppError;
	Unrecoverable?: { msg: string; };
};
export type RecoverableAppError = {
	ConfigError?: ConfigError;
	ModManagerError?: ModManagerError;
};

export type ConfigError =
	| { type: "DeSerialization"; msg: string; }
	| { type: "Io"; msg: string; }
	| { type: "GameLanguageNotSupported"; }
	| ({ type: "InvalidGamePath"; } & InvalidGamePath);

export type InvalidGamePath =
	| { invalidGamePath: "NotExisting"; }
	| { invalidGamePath: "NotADirectory"; }
	| { invalidGamePath: "InvalidPath"; }
	| { invalidGamePath: "InvalidFolderName"; }
	| { invalidGamePath: "LocalizationNotFound"; };

export type ModManagerError =
	| { type: "Io"; msg: string; }
	| { type: "Db"; msg: string; }
	| { type: "DeSerialization"; msg: string; }
	| ({ type: "InvalidArchive"; } & InvalidArchive)
	| { type: "ArchiveHandling"; msg: string; }
	| { type: "InvalidModInfo"; msg: string; }
	| { type: "ModNotExisting"; }
	| { type: "ModAlreadyActive"; }
	| { type: "ModAlreadyDeactivated"; }
	| { type: "ModVersionMismatch"; mismatch: [string, string]; }
	| { type: "AppNotInitialized"; }
	| { type: "ModConflict"; conflict: Array<[string, string]>; }
	| ({ type: "ConfigError"; } & ConfigError)
	| { type: "TauriError"; msg: string; };

export type InvalidArchive =
	| { invalidArchive: "PathNotExisting"; }
	| { invalidArchive: "PathNotFile"; }
	| { invalidArchive: "NoExtension"; }
	| { invalidArchive: "InvalidExtension"; };

/**
 * Invoke a command in the backend. This automatically handles unrecoverable app errors and forwards recoverable app errors. See the error.rs file in the backend code for more info.
 *
 * @param command The command
 * @param args The payload of the command
 */
export async function invokeBackend<T>(
	command: string,
	args?: InvokeArgs | undefined
): Promise<T | RecoverableAppError | undefined> {
	try {
		const result = await invoke<T>(command, args);
		return result;
	} catch (error) {
		let err = error as AppError;
		console.log(error);

		if (err.Unrecoverable) {
			await message(
				get(_)("error.fatal.dialogMessage", {
					values: {
						error: err.Unrecoverable.msg,
						githubIssueUrl: "https://github.com/TeyKey1/AW-Modloader/Issues"
					}
				}),
				{ title: "Fatal Error", type: "error" }
			);

			// Exit the entire process
			await exit(1);
		} else {
			// return the recoverable error to the caller
			return err.Recoverable!;
		}
	}
}

/**
 * Check if the data returned by invokeBackend() is an error
 * @param data the data to check
 * @returns boolean
 */
export function isError(data: any | RecoverableAppError): data is RecoverableAppError {
	return (
		!!(data as RecoverableAppError)?.ConfigError || !!(data as RecoverableAppError)?.ModManagerError
	);
}

/**
 * Returns the specific error message translated to the currently active locale
 * @param error
 * @returns the translated error message
 */
export function getErrorMessage(error: RecoverableAppError): string {
	if (error.ConfigError) {
		switch (error.ConfigError.type) {
			case "GameLanguageNotSupported":
				return get(_)("error.GameLanguageNotSupported");
			case "InvalidGamePath":
				return get(_)(`error.invalidGamePath.${error.ConfigError.invalidGamePath}`);
			default:
				return "Fatal unhandled ConfigError";
		}
	} else if (error.ModManagerError) {
		switch (error.ModManagerError.type) {
			case "ArchiveHandling":
				return get(_)("error.ArchiveHandling", {
					values: { error: error.ModManagerError.msg }
				});
			case "InvalidArchive":
				return get(_)(`error.invalidArchive.${error.ModManagerError.invalidArchive}`);
			case "InvalidModInfo":
				return get(_)("error.InvalidModInfo", { values: { error: error.ModManagerError.msg } });
			case "ModConflict":
				let conflictString = "";

				error.ModManagerError.conflict.forEach((conflict) => {
					conflictString += `\t${conflict[0]} -> ${conflict[1]}\n`;
				});

				return get(_)("error.ModConflict", { values: { conflicts: conflictString } });
			case "ModVersionMismatch":
				return get(_)("error.ModVersionMismatch", {
					values: {
						installedVersion: error.ModManagerError.mismatch[1],
						newVersion: error.ModManagerError.mismatch[0]
					}
				});
			default:
				return "Fatal unhandled ModManagerError";
		}
	}

	return "Unknown";
}
