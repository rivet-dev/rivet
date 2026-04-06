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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBorderCenterV.js
var require_faBorderCenterV = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBorderCenterV.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "border-center-v";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f89d";
    var svgPathData = "M448 448a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm0-96a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM64 352A32 32 0 1 0 0 352a32 32 0 1 0 64 0zM384 160a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM64 160A32 32 0 1 0 0 160a32 32 0 1 0 64 0zm320 96a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM64 256A32 32 0 1 0 0 256a32 32 0 1 0 64 0zM384 64a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM0 64a32 32 0 1 0 64 0A32 32 0 1 0 0 64zM64 448A32 32 0 1 0 0 448a32 32 0 1 0 64 0zM288 256a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM352 64a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM288 448a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zM160 64A32 32 0 1 0 96 64a32 32 0 1 0 64 0zM96 448a32 32 0 1 0 64 0 32 32 0 1 0 -64 0zm64-192a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm32 192c0 17.7 14.3 32 32 32s32-14.3 32-32l0-384c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 384z";
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
    exports.faBorderCenterV = exports.definition;
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

// src/_entries/faBorderCenterV.js
var import_faBorderCenterV = __toESM(require_faBorderCenterV());
var export_faBorderCenterV = import_faBorderCenterV.definition;
export {
  export_faBorderCenterV as faBorderCenterV
};
