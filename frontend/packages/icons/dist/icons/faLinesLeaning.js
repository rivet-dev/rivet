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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLinesLeaning.js
var require_faLinesLeaning = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLinesLeaning.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lines-leaning";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e51e";
    var svgPathData = "M190.4 74.1c5.6-16.8-3.5-34.9-20.2-40.5s-34.9 3.5-40.5 20.2l-128 384c-5.6 16.8 3.5 34.9 20.2 40.5s34.9-3.5 40.5-20.2l128-384zm70.9-41.7c-17.4-2.9-33.9 8.9-36.8 26.3l-64 384c-2.9 17.4 8.9 33.9 26.3 36.8s33.9-8.9 36.8-26.3l64-384c2.9-17.4-8.9-33.9-26.3-36.8zM352 32c-17.7 0-32 14.3-32 32l0 384c0 17.7 14.3 32 32 32s32-14.3 32-32l0-384c0-17.7-14.3-32-32-32z";
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
    exports.faLinesLeaning = exports.definition;
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

// src/_entries/faLinesLeaning.js
var import_faLinesLeaning = __toESM(require_faLinesLeaning());
var export_faLinesLeaning = import_faLinesLeaning.definition;
export {
  export_faLinesLeaning as faLinesLeaning
};
