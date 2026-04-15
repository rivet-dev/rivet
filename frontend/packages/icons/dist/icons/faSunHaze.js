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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSunHaze.js
var require_faSunHaze = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSunHaze.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sun-haze";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f765";
    var svgPathData = "M425.5 1.2c5 2.1 8.6 6.6 9.6 11.9L455 121l107.9 19.8c5.3 1 9.8 4.6 11.9 9.6s1.5 10.7-1.6 15.2L510.9 256 448 256c0-70.7-57.3-128-128-128s-128 57.3-128 128l-62.9 0L66.8 165.7c-3.1-4.5-3.7-10.2-1.6-15.2s6.6-8.6 11.9-9.6L185 121 204.9 13.1c1-5.3 4.6-9.8 9.6-11.9s10.7-1.5 15.2 1.6L320 65.1 410.3 2.8c4.5-3.1 10.2-3.7 15.2-1.6zM320 160c53 0 96 43 96 96l-192 0c0-53 43-96 96-96zM616 288c13.3 0 24 10.7 24 24s-10.7 24-24 24l-112 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l112 0zm-208 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L88 336c-13.3 0-24-10.7-24-24s10.7-24 24-24l320 0zm144 88c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 424c-13.3 0-24-10.7-24-24s10.7-24 24-24l528 0zM64 488c0-13.3 10.7-24 24-24l112 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L88 512c-13.3 0-24-10.7-24-24zm208 0c0-13.3 10.7-24 24-24l320 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-320 0c-13.3 0-24-10.7-24-24z";
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
    exports.faSunHaze = exports.definition;
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

// src/_entries/faSunHaze.js
var import_faSunHaze = __toESM(require_faSunHaze());
var export_faSunHaze = import_faSunHaze.definition;
export {
  export_faSunHaze as faSunHaze
};
