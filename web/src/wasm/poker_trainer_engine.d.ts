/* tslint:disable */
/* eslint-disable */

/**
 * Build a fresh default config the JS side can mutate before passing back.
 */
export function default_config(): any;

/**
 * Evaluate the user's chosen action against the given spot. Returns
 * a `DecisionFeedback` describing whether they were correct, the EVs of
 * the choice they made and the GTO-best alternative, and an explanation.
 */
export function evaluate_action(spot: any, action: string): any;

/**
 * Generate a new training spot for the given config.
 */
export function generate_spot(config: any): any;

/**
 * Set up a panic hook so Rust panics include a stack-style message in the
 * browser console (as a thrown JsError) instead of just `RuntimeError`.
 */
export function init(): void;

/**
 * List the training modes available, with display labels for the UI.
 */
export function training_modes(): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly default_config: () => [number, number, number];
    readonly evaluate_action: (a: any, b: number, c: number) => [number, number, number];
    readonly generate_spot: (a: any) => [number, number, number];
    readonly init: () => void;
    readonly training_modes: () => [number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
