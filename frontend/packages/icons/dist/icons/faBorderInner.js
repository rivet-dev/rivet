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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBorderInner.js
var require_faBorderInner = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBorderInner.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "border-inner";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f84e";
    var svgPathData = "M64 448A32 32 0 1 1 0 448a32 32 0 1 1 64 0zm96 0a32 32 0 1 1 -64 0 32 32 0 1 1 64 0zm0-384A32 32 0 1 1 96 64a32 32 0 1 1 64 0zM320 416a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM352 64a32 32 0 1 1 -64 0 32 32 0 1 1 64 0zm64 352a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm0-384a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM64 64A32 32 0 1 1 0 64a32 32 0 1 1 64 0zM416 320a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM64 352A32 32 0 1 1 0 352a32 32 0 1 1 64 0zM416 128a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM64 160A32 32 0 1 1 0 160a32 32 0 1 1 64 0zM0 256c0-17.7 14.3-32 32-32l160 0 0-160c0-17.7 14.3-32 32-32s32 14.3 32 32l0 160 160 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-160 0 0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-160L32 288c-17.7 0-32-14.3-32-32z";
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
    exports.faBorderInner = exports.definition;
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

// src/_entries/faBorderInner.js
var import_faBorderInner = __toESM(require_faBorderInner());
var export_faBorderInner = import_faBorderInner.definition;
export {
  export_faBorderInner as faBorderInner
};
