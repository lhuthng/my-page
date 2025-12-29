import "./shared/binding.d-CrH1UT-g.cjs";
import { MaybePromise, Plugin } from "./shared/define-config.d-2mCqGCzl.cjs";

//#region src/plugin/parallel-plugin-implementation.d.ts
type ParallelPluginImplementation = Plugin;
type Context = {
	/**
	* Thread number
	*/
	threadNumber: number
};
declare function defineParallelPluginImplementation<Options>(plugin: (Options: Options, context: Context) => MaybePromise<ParallelPluginImplementation>): (Options: Options, context: Context) => MaybePromise<ParallelPluginImplementation>;

//#endregion
export { Context, ParallelPluginImplementation, defineParallelPluginImplementation };