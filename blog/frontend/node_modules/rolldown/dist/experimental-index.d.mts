import { BindingReplacePluginConfig, BindingTransformPluginConfig, IsolatedDeclarationsOptions, IsolatedDeclarationsResult, NapiResolveOptions, ResolveResult, ResolverFactory, TransformOptions, TransformResult$1 as TransformResult, isolatedDeclaration, moduleRunnerTransform, transform } from "./shared/binding.d-B9SSHAER.mjs";
import { BuiltinPlugin$1 as BuiltinPlugin, InputOptions, RolldownPlugin, assetPlugin$1 as assetPlugin, buildImportAnalysisPlugin$1 as buildImportAnalysisPlugin, defineParallelPlugin, dynamicImportVarsPlugin$1 as dynamicImportVarsPlugin, importGlobPlugin$1 as importGlobPlugin, isolatedDeclarationPlugin$1 as isolatedDeclarationPlugin, jsonPlugin$1 as jsonPlugin, loadFallbackPlugin$1 as loadFallbackPlugin, manifestPlugin$1 as manifestPlugin, moduleFederationPlugin$1 as moduleFederationPlugin, modulePreloadPolyfillPlugin$1 as modulePreloadPolyfillPlugin, reporterPlugin$1 as reporterPlugin, viteResolvePlugin$1 as viteResolvePlugin, wasmFallbackPlugin$1 as wasmFallbackPlugin, wasmHelperPlugin$1 as wasmHelperPlugin, webWorkerPostPlugin$1 as webWorkerPostPlugin } from "./shared/define-config.d-CUCXQ5Qr.mjs";

//#region src/api/experimental.d.ts
/**
* This is an experimental API. It's behavior may change in the future.
*
* Calling this API will only execute the scan stage of rolldown.
*/
/**
* This is an experimental API. It's behavior may change in the future.
*
* Calling this API will only execute the scan stage of rolldown.
*/
declare const experimental_scan: (input: InputOptions) => Promise<void>;

//#endregion
//#region src/utils/compose-js-plugins.d.ts
declare function composeJsPlugins(plugins: RolldownPlugin[]): RolldownPlugin[];

//#endregion
//#region src/builtin-plugin/alias-plugin.d.ts
type AliasPluginAlias = {
	find: string | RegExp
	replacement: string
};
type AliasPluginConfig = {
	entries: AliasPluginAlias[]
};
declare function aliasPlugin(config: AliasPluginConfig): BuiltinPlugin;

//#endregion
//#region src/builtin-plugin/replace-plugin.d.ts
/**
* Replaces targeted strings in files while bundling.
*
* @example
* // Basic usage
* ```js
* replacePlugin({
*   'process.env.NODE_ENV': JSON.stringify('production'),
*    __buildDate__: () => JSON.stringify(new Date()),
*    __buildVersion: 15
* })
* ```
* @example
* // With options
* ```js
* replacePlugin({
*   'process.env.NODE_ENV': JSON.stringify('production'),
*   __buildDate__: () => JSON.stringify(new Date()),
*   __buildVersion: 15
* }, {
*   preventAssignment: false,
* })
* ```
*/
/**
* Replaces targeted strings in files while bundling.
*
* @example
* // Basic usage
* ```js
* replacePlugin({
*   'process.env.NODE_ENV': JSON.stringify('production'),
*    __buildDate__: () => JSON.stringify(new Date()),
*    __buildVersion: 15
* })
* ```
* @example
* // With options
* ```js
* replacePlugin({
*   'process.env.NODE_ENV': JSON.stringify('production'),
*   __buildDate__: () => JSON.stringify(new Date()),
*   __buildVersion: 15
* }, {
*   preventAssignment: false,
* })
* ```
*/
declare function replacePlugin(values?: BindingReplacePluginConfig["values"], options?: Omit<BindingReplacePluginConfig, "values">): BuiltinPlugin;

//#endregion
//#region src/builtin-plugin/transform-plugin.d.ts
type TransformPattern = string | RegExp | readonly (RegExp | string)[];
type TransformPluginConfig = Omit<BindingTransformPluginConfig, "include" | "exclude" | "jsxRefreshInclude" | "jsxRefreshExclude"> & {
	include?: TransformPattern
	exclude?: TransformPattern
	jsxRefreshInclude?: TransformPattern
	jsxRefreshExclude?: TransformPattern
};
declare function transformPlugin(config?: TransformPluginConfig): BuiltinPlugin;

//#endregion
export { IsolatedDeclarationsOptions, IsolatedDeclarationsResult, NapiResolveOptions as ResolveOptions, ResolveResult, ResolverFactory, TransformOptions, TransformResult, aliasPlugin, assetPlugin, buildImportAnalysisPlugin, composeJsPlugins as composePlugins, defineParallelPlugin, dynamicImportVarsPlugin, importGlobPlugin, isolatedDeclaration, isolatedDeclarationPlugin, jsonPlugin, loadFallbackPlugin, manifestPlugin, moduleFederationPlugin, modulePreloadPolyfillPlugin, moduleRunnerTransform, replacePlugin, reporterPlugin, experimental_scan as scan, transform, transformPlugin, viteResolvePlugin, wasmFallbackPlugin, wasmHelperPlugin, webWorkerPostPlugin };