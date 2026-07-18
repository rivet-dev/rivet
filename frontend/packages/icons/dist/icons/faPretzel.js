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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPretzel.js
var require_faPretzel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPretzel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pretzel";
    var width = 512;
    var height = 512;
    var aliases = [129384];
    var unicode = "e441";
    var svgPathData = "M138.6 32C62.1 32 0 94.1 0 170.6C0 253.3 40.9 326.5 103.7 371L71.3 417C58.6 435 62.9 460 81 472.7s43 8.4 55.7-9.7l40-56.8c21.8 6.4 44.9 9.8 68.7 9.8l10.6 0 10.6 0c23.9 0 46.9-3.4 68.7-9.8l40 56.8c12.7 18.1 37.7 22.4 55.7 9.7s22.4-37.7 9.7-55.7l-32.4-46C471.1 326.5 512 253.3 512 170.6C512 94.1 449.9 32 373.4 32L364 32c-42.6 0-81 18-108 46.8C229 50 190.6 32 148 32l-9.4 0zM216 180l0 7.4c0 15.5-4.8 30.7-13.7 43.4l-52.5 74.7C107.5 275.6 80 226.3 80 170.6c0-32.4 26.2-58.6 58.6-58.6l9.4 0c37.6 0 68 30.4 68 68zm40 113.5L285.2 335c-6.1 .7-12.3 1-18.6 1L256 336l-10.6 0c-6.3 0-12.5-.3-18.6-1L256 293.5zm53.7-62.6C300.8 218.2 296 203 296 187.4l0-7.4c0-37.6 30.4-68 68-68l9.4 0c32.4 0 58.6 26.2 58.6 58.6c0 55.7-27.5 105-69.7 134.9l-52.5-74.7z";
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
    exports.faPretzel = exports.definition;
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

// src/_entries/faPretzel.js
var import_faPretzel = __toESM(require_faPretzel());
var export_faPretzel = import_faPretzel.definition;
export {
  export_faPretzel as faPretzel
};
