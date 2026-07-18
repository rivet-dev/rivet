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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShelvesEmpty.js
var require_faShelvesEmpty = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShelvesEmpty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shelves-empty";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e246";
    var svgPathData = "M64 32C64 14.3 49.7 0 32 0S0 14.3 0 32L0 192 0 448l0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l512 0c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32 0-256 0-160c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 128L64 160 64 32zm0 192l512 0 0 192L64 416l0-192z";
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
    exports.faShelvesEmpty = exports.definition;
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

// src/_entries/faShelvesEmpty.js
var import_faShelvesEmpty = __toESM(require_faShelvesEmpty());
var export_faShelvesEmpty = import_faShelvesEmpty.definition;
export {
  export_faShelvesEmpty as faShelvesEmpty
};
