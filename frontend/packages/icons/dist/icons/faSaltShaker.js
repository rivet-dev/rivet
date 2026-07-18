var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSaltShaker.js
var require_faSaltShaker = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSaltShaker.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "salt-shaker";
    var width = 384;
    var height = 512;
    var aliases = [129474];
    var unicode = "e446";
    var svgPathData = "M319.4 448L64.6 448l36.6-256 181.6 0 36.6 256zM59.8 512l264.5 0c33 0 59.8-26.8 59.8-59.8c0-2.8-.2-5.7-.6-8.5L338.1 126.7C327.7 54 265.4 0 192 0S56.3 54 45.9 126.7L.6 443.8c-.4 2.8-.6 5.6-.6 8.5c0 33 26.8 59.8 59.8 59.8zM192 48a16 16 0 1 1 0 32 16 16 0 1 1 0-32zM128 96a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zM240 80a16 16 0 1 1 0 32 16 16 0 1 1 0-32z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faSaltShaker = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faSaltShaker.js
var import_faSaltShaker = __toESM(require_faSaltShaker());
var export_faSaltShaker = import_faSaltShaker.definition;
export {
  export_faSaltShaker as faSaltShaker
};
