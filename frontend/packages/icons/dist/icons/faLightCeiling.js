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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightCeiling.js
var require_faLightCeiling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightCeiling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "light-ceiling";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e016";
    var svgPathData = "M256 0c17.7 0 32 14.3 32 32l0 130c115.8 14.4 207.6 106.3 222 222.1c2.2 17.5-12.4 31.9-30 31.9L32 416C14.3 416-.2 401.6 2 384.1C16.4 268.2 108.2 176.4 224 162l0-130c0-17.7 14.3-32 32-32zm0 512c-35.3 0-64-28.7-64-64l128 0c0 35.3-28.7 64-64 64z";
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
    exports.faLightCeiling = exports.definition;
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

// src/_entries/faLightCeiling.js
var import_faLightCeiling = __toESM(require_faLightCeiling());
var export_faLightCeiling = import_faLightCeiling.definition;
export {
  export_faLightCeiling as faLightCeiling
};
