import { defineConfig, init_define_config, version } from "./shared/src-DgKqmPhz.mjs";
import "./shared/parse-ast-index-BloqIIaY.mjs";
import "./shared/misc-DGAe2XOW.mjs";
import { init_load_config, loadConfig } from "./shared/load-config-D6Rgj8MR.mjs";

//#region src/config.ts
init_define_config();
init_load_config();
const VERSION = version;

//#endregion
export { VERSION, defineConfig, loadConfig };