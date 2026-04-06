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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKeySkeletonLeftRight.js
var require_faKeySkeletonLeftRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKeySkeletonLeftRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "key-skeleton-left-right";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3b4";
    var svgPathData = "M576 128a64 64 0 1 0 -128 0 64 64 0 1 0 128 0zm64 0c0 70.7-57.3 128-128 128c-59.6 0-109.8-40.8-124-96l-68 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-32 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32l196 0C402.2 40.8 452.4 0 512 0c70.7 0 128 57.3 128 128zM64 384a64 64 0 1 0 128 0A64 64 0 1 0 64 384zM0 384c0-70.7 57.3-128 128-128c59.6 0 109.8 40.8 124 96l68 0 0-32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 32 32 0 0-32c0-17.7 14.3-32 32-32s32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l-196 0c-14.2 55.2-64.3 96-124 96C57.3 512 0 454.7 0 384z";
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
    exports.faKeySkeletonLeftRight = exports.definition;
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

// src/_entries/faKeySkeletonLeftRight.js
var import_faKeySkeletonLeftRight = __toESM(require_faKeySkeletonLeftRight());
var export_faKeySkeletonLeftRight = import_faKeySkeletonLeftRight.definition;
export {
  export_faKeySkeletonLeftRight as faKeySkeletonLeftRight
};
