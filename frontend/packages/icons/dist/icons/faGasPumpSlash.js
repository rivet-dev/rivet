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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGasPumpSlash.js
var require_faGasPumpSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGasPumpSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gas-pump-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f5f4";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-72.2-56.6C569.5 397.9 576 379.7 576 360l0-136 0-32 0-24 0-13.5c0-17-6.7-33.3-18.7-45.3L480 32c-8.8-8.8-23.2-8.8-32 0s-8.8 23.2 0 32l32 32 0 64c0 29.8 20.4 54.9 48 62l0 138c0 8.5-2.7 16.4-7.2 22.9L384 275.7 384 64c0-35.3-28.7-64-64-64L160 0c-30.9 0-56.7 21.9-62.7 51L38.8 5.1zM96 171.6L96 448c-17.7 0-32 14.3-32 32s14.3 32 32 32l288 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-49.5L96 171.6z";
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
    exports.faGasPumpSlash = exports.definition;
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

// src/_entries/faGasPumpSlash.js
var import_faGasPumpSlash = __toESM(require_faGasPumpSlash());
var export_faGasPumpSlash = import_faGasPumpSlash.definition;
export {
  export_faGasPumpSlash as faGasPumpSlash
};
