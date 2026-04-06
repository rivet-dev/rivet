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

// src/node_modules/@fortawesome/free-solid-svg-icons/fa3.js
var require_fa3 = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/fa3.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "3";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "33";
    var svgPathData = "M80 288c-17.7 0-32-14.3-32-32s14.3-32 32-32l112 0c35.3 0 64-28.7 64-64s-28.7-64-64-64L32 96C14.3 96 0 81.7 0 64S14.3 32 32 32l160 0c70.7 0 128 57.3 128 128 0 38.2-16.8 72.5-43.3 96 26.6 23.5 43.3 57.8 43.3 96 0 70.7-57.3 128-128 128L32 480c-17.7 0-32-14.3-32-32s14.3-32 32-32l160 0c35.3 0 64-28.7 64-64s-28.7-64-64-64L80 288z";
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
    exports.fa3 = exports.definition;
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

// src/_entries/fa3.js
var import_fa3 = __toESM(require_fa3());
var export_fa3 = import_fa3.definition;
export {
  export_fa3 as fa3
};
