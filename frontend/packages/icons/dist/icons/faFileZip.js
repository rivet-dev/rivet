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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileZip.js
var require_faFileZip = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileZip.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-zip";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5ee";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L224 0l0 128c0 17.7 14.3 32 32 32l128 0 0 144-144 0c-35.3 0-64 28.7-64 64l0 144L64 512c-35.3 0-64-28.7-64-64L0 64zm384 64l-128 0L256 0 384 128zM240 352l64 0c5.5 0 10.7 2.9 13.6 7.6s3.2 10.6 .7 15.6L265.9 480l38.1 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-64 0c-5.5 0-10.7-2.9-13.6-7.6s-3.2-10.6-.7-15.6L278.1 384 240 384c-8.8 0-16-7.2-16-16s7.2-16 16-16zm144 16l0 128c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-128c0-8.8 7.2-16 16-16s16 7.2 16 16zm32 0c0-8.8 7.2-16 16-16l24 0c30.9 0 56 25.1 56 56s-25.1 56-56 56l-8 0 0 32c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-48 0-80zm32 64l8 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-8 0 0 48z";
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
    exports.faFileZip = exports.definition;
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

// src/_entries/faFileZip.js
var import_faFileZip = __toESM(require_faFileZip());
var export_faFileZip = import_faFileZip.definition;
export {
  export_faFileZip as faFileZip
};
