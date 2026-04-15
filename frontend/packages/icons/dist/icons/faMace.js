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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMace.js
var require_faMace = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMace.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mace";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6f8";
    var svgPathData = "M244.8 95l28-84.1C275 4.4 281.1 0 288 0s13 4.4 15.2 10.9l28 84.1c40.4 13.5 72.3 45.4 85.8 85.8l84.1 28c6.5 2.2 10.9 8.3 10.9 15.2s-4.4 13-10.9 15.2l-84.1 28c-13.5 40.4-45.4 72.3-85.8 85.8l-28 84.1C301 443.6 294.9 448 288 448s-13-4.4-15.2-10.9l-28-84.1c-9.8-3.3-19-7.6-27.7-12.9L54.6 502.6c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L171.9 294.9c-5.3-8.6-9.6-17.9-12.9-27.7l-84.1-28C68.4 237 64 230.9 64 224s4.4-13 10.9-15.2l84.1-28c13.5-40.4 45.4-72.3 85.8-85.8zM288 256a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faMace = exports.definition;
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

// src/_entries/faMace.js
var import_faMace = __toESM(require_faMace());
var export_faMace = import_faMace.definition;
export {
  export_faMace as faMace
};
