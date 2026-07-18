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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSkeleton.js
var require_faSkeleton = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSkeleton.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "skeleton";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f620";
    var svgPathData = "M256 0c17.7 0 32 14.3 32 32l0 32 128 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-128 0 0 32 192 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-192 0 0 32 96 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-96 0 0 64c0 .8 0 1.6-.1 2.3l67.6-29c8.3-3.5 17.2-5.4 26.1-5.4c36.7 0 66.4 29.7 66.4 66.4l0 1.5c0 18-7.2 35.3-19.9 48l-25.8 25.8c-11.7 11.7-27.6 18.3-44.1 18.3l-204.3 0c-16.6 0-32.4-6.6-44.1-18.3L83.9 467.9c-12.7-12.7-19.9-30-19.9-48l0-1.5c0-36.7 29.7-66.4 66.4-66.4c9 0 17.9 1.8 26.1 5.4l67.6 29c-.1-.8-.1-1.5-.1-2.3l0-64-96 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l96 0 0-32L32 224c-17.7 0-32-14.3-32-32s14.3-32 32-32l192 0 0-32L96 128c-17.7 0-32-14.3-32-32s14.3-32 32-32l128 0 0-32c0-17.7 14.3-32 32-32zM184 440a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm192 0a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faSkeleton = exports.definition;
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

// src/_entries/faSkeleton.js
var import_faSkeleton = __toESM(require_faSkeleton());
var export_faSkeleton = import_faSkeleton.definition;
export {
  export_faSkeleton as faSkeleton
};
