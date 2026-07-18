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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTablePicnic.js
var require_faTablePicnic = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTablePicnic.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "table-picnic";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e32d";
    var svgPathData = "M319.3 64L192.7 64c-.5 0-1 0-1.5 0L96 64C78.3 64 64 78.3 64 96s14.3 32 32 32l48.7 0-38.4 96L32 224c-17.7 0-32 14.3-32 32s14.3 32 32 32l48.7 0 68.9 0 212.7 0 68.9 0 48.7 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-74.3 0-38.4-96 48.7 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-95.3 0c-.5 0-1 0-1.5 0zM444.1 320l-68.9 0 43.2 107.9c6.6 16.4 25.2 24.4 41.6 17.8s24.4-25.2 17.8-41.6L444.1 320zm-307.2 0l-68.9 0L34.3 404.1c-6.6 16.4 1.4 35 17.8 41.6s35-1.4 41.6-17.8L136.9 320zm38.4-96l38.4-96 84.7 0 38.4 96-161.5 0z";
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
    exports.faTablePicnic = exports.definition;
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

// src/_entries/faTablePicnic.js
var import_faTablePicnic = __toESM(require_faTablePicnic());
var export_faTablePicnic = import_faTablePicnic.definition;
export {
  export_faTablePicnic as faTablePicnic
};
