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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPodium.js
var require_faPodium = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPodium.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "podium";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f680";
    var svgPathData = "M182.4 72c8.3 14.3 23.8 24 41.6 24l64 0c26.5 0 48-21.5 48-48s-21.5-48-48-48L224 0c-17.8 0-33.3 9.7-41.6 24L168 24c-48.6 0-88 39.4-88 88l0 32-56 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l400 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-296 0 0-32c0-22.1 17.9-40 40-40l14.4 0zM48 224L73.7 455.1c3.6 32.4 31 56.9 63.6 56.9l173.4 0c32.6 0 60-24.5 63.6-56.9L400 224 48 224z";
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
    exports.faPodium = exports.definition;
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

// src/_entries/faPodium.js
var import_faPodium = __toESM(require_faPodium());
var export_faPodium = import_faPodium.definition;
export {
  export_faPodium as faPodium
};
