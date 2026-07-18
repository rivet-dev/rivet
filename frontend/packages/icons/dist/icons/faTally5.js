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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTally.js
var require_faTally = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTally.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tally";
    var width = 640;
    var height = 512;
    var aliases = ["tally-5"];
    var unicode = "f69c";
    var svgPathData = "M128 32c17.7 0 32 14.3 32 32l0 211.6 64-21.3L224 64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 168.9 64-21.3L352 64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 126.3 64-21.3L480 64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 83.6 53.9-18c16.8-5.6 34.9 3.5 40.5 20.2s-3.5 34.9-20.2 40.5L544 215.1 544 448c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-211.6-64 21.3L416 448c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-168.9-64 21.3L288 448c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-126.3-64 21.3L160 448c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-83.6-53.9 18c-16.8 5.6-34.9-3.5-40.5-20.2s3.5-34.9 20.2-40.5L96 296.9 96 64c0-17.7 14.3-32 32-32z";
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
    exports.faTally = exports.definition;
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

// src/_entries/faTally5.js
var import_faTally = __toESM(require_faTally());
var export_faTally5 = import_faTally.definition;
export {
  export_faTally5 as faTally5
};
