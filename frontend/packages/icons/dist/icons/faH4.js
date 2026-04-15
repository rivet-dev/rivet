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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faH4.js
var require_faH4 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faH4.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "h4";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f86a";
    var svgPathData = "M479 103.8L441 256l135 0 0-160c0-17.7 14.3-32 32-32s32 14.3 32 32l0 320c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-96-176 0c-9.9 0-19.2-4.5-25.2-12.3s-8.2-17.9-5.8-27.5l48-192c4.3-17.1 21.7-27.6 38.8-23.3s27.6 21.7 23.3 38.8zM32 64c17.7 0 32 14.3 32 32l0 128 192 0 0-128c0-17.7 14.3-32 32-32s32 14.3 32 32l0 160 0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-128L64 288l0 128c0 17.7-14.3 32-32 32s-32-14.3-32-32L0 256 0 96C0 78.3 14.3 64 32 64z";
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
    exports.faH4 = exports.definition;
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

// src/_entries/faH4.js
var import_faH4 = __toESM(require_faH4());
var export_faH4 = import_faH4.definition;
export {
  export_faH4 as faH4
};
