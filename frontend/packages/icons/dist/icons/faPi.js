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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPi.js
var require_faPi = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPi.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pi";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f67e";
    var svgPathData = "M32 64C14.3 64 0 78.3 0 96s14.3 32 32 32l64 0 0 156c0 39.7-9.3 78.9-27 114.5l-1.6 3.2c-7.9 15.8-1.5 35 14.3 42.9s35 1.5 42.9-14.3l1.6-3.2C148.4 382.7 160 333.7 160 284l0-156 128 0 0 238.6c0 45 36.5 81.4 81.4 81.4c30.8 0 59-17.4 72.8-45l2.3-4.7c7.9-15.8 1.5-35-14.3-42.9s-35-1.5-42.9 14.3l-2.3 4.7c-3 5.9-9 9.6-15.6 9.6c-9.6 0-17.4-7.8-17.4-17.4L352 128l64 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 64z";
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
    exports.faPi = exports.definition;
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

// src/_entries/faPi.js
var import_faPi = __toESM(require_faPi());
var export_faPi = import_faPi.definition;
export {
  export_faPi as faPi
};
